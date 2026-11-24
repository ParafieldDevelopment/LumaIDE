//! Coordinates discovery, dependency ordering and loading of extensions.

#[cfg(feature = "host")]
use std::collections::HashSet;
#[cfg(feature = "host")]
use std::path::{Path, PathBuf};
#[cfg(feature = "host")]
use std::sync::{Arc, RwLock};

use anyhow::Result;
use luma_api::events::{ExtensionLoadedEvent, ExtensionUnloadedEvent};
#[cfg(feature = "host")]
use log::info;
use log::warn;
#[cfg(feature = "host")]
use luma_api::{CoreAPI, Shared};
use semver::Version;

#[cfg(feature = "host")]
use crate::abi::{ExtensionState, ExtensionStateHandle};
#[cfg(feature = "host")]
use crate::discovery::discover_extensions_with_failures;
#[cfg(feature = "host")]
use crate::manifest::ExtensionDescriptor;
use crate::manifest::ExtensionKind;
#[cfg(feature = "host")]
use crate::resolver::resolve_load_order_with_failures;

#[cfg(all(feature = "host", feature = "native"))]
use crate::native::NativeExtension;
#[cfg(all(feature = "host", feature = "wasm"))]
use crate::wasm::WasmRuntime;

/// Common interface implemented by loaded native and WASM extensions.
pub trait LoadedExtension: Send {
    fn name(&self) -> &str;
    fn kind(&self) -> ExtensionKind;
    fn version(&self) -> &Version;
    fn shutdown(&mut self) -> Result<()>;
}

/// Why an extension could not be loaded. Loading continues with other
/// extensions so one broken addon does not take down the IDE.
#[derive(Debug, Clone)]
pub struct ExtensionLoadFailure {
    pub name: String,
    pub error: String,
}

/// Owns all currently loaded extensions.
pub struct ExtensionManager {
    loaded: Vec<Box<dyn LoadedExtension>>,
    failures: Vec<ExtensionLoadFailure>,
    /// Snapshot exposed to extensions through the host ABI/imports.
    #[cfg(feature = "host")]
    extension_state: ExtensionStateHandle,
    #[cfg(all(feature = "host", feature = "wasm"))]
    wasm_runtime: WasmRuntime,
}

impl Default for ExtensionManager {
    fn default() -> Self {
        Self {
            loaded: Vec::new(),
            failures: Vec::new(),
            #[cfg(feature = "host")]
            extension_state: Arc::new(RwLock::new(ExtensionState::default())),
            #[cfg(all(feature = "host", feature = "wasm"))]
            wasm_runtime: WasmRuntime::default(),
        }
    }
}

impl ExtensionManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Discover, resolve and load all extensions under `roots`.
    ///
    /// Archives are extracted into `cache_dir`. `core_api` is passed to native
    /// extensions through the C ABI function table.
    #[cfg(feature = "host")]
    pub fn load_all(
        &mut self,
        core_api: &Shared<dyn CoreAPI>,
        roots: impl IntoIterator<Item = PathBuf>,
        cache_dir: &Path,
    ) -> Result<()> {
        // Discovery and dependency resolution problems are recorded as extension
        // failures instead of aborting IDE startup. One broken archive or
        // manifest must not prevent every other extension from loading.
        let discovery = match discover_extensions_with_failures(roots, cache_dir) {
            Ok(discovery) => discovery,
            Err(error) => {
                self.record_failure("<discovery>", &error);
                info!("Extensions loaded: 0 ({} failed)", self.failures.len());
                return Ok(());
            }
        };
        for failure in &discovery.failures {
            let name = failure.path.display().to_string();
            warn!(
                "Failed to discover extension at '{name}': {}",
                failure.error
            );
            self.record_failure(name, &failure.error);
        }
        let descriptors = discovery.descriptors;
        let resolution = match resolve_load_order_with_failures(&descriptors) {
            Ok(resolution) => resolution,
            Err(error) => {
                self.record_failure("<resolution>", &error);
                info!("Extensions loaded: 0 ({} failed)", self.failures.len());
                return Ok(());
            }
        };

        let mut failed_names = HashSet::new();
        for (name, error) in &resolution.failures {
            warn!("Skipping extension '{name}': {error}");
            failed_names.insert(name.clone());
            self.record_failure(name.clone(), error);
        }

        for (extension, skipped) in &resolution.skipped_optional {
            for dependency in skipped {
                warn!(
                    "Optional dependency '{dependency}' of extension '{extension}' is unavailable; skipping it"
                );
            }
        }

        let mut loaded_names = HashSet::new();

        for index in resolution.order {
            let descriptor = &descriptors[index];
            let name = &descriptor.manifest.name;
            self.update_extension_state(&loaded_names);

            let blocked =
                descriptor.manifest.dependencies.iter().any(|dependency| {
                    !dependency.optional && failed_names.contains(&dependency.name)
                });
            if blocked {
                let error = "A required dependency failed to load".to_string();
                warn!("Skipping extension '{name}': {error}");
                failed_names.insert(name.clone());
                self.record_failure(name.clone(), &error);
                continue;
            }

            match self.load_one(descriptor, core_api) {
                Ok(extension) => {
                    info!(
                        "Loaded {} extension '{name}' v{}",
                        descriptor.manifest.kind, descriptor.manifest.version
                    );
                    luma_api::event_bus().fire(ExtensionLoadedEvent {
                        name: name.clone(),
                        kind: descriptor.manifest.kind.to_string(),
                        version: descriptor.manifest.version.to_string(),
                    });
                    loaded_names.insert(name.clone());
                    self.loaded.push(extension);
                }
                Err(error) => {
                    // A guest may have registered listeners before failing.
                    luma_api::event_bus().unregister_owner(name);
                    warn!("Failed to load extension '{name}': {error:#}");
                    failed_names.insert(name.clone());
                    self.record_failure(name.clone(), &error);
                }
            }
        }

        // Publish the final snapshot too: the last extension (and the last
        // failure) would otherwise be missing from queries made after boot.
        self.update_extension_state(&loaded_names);

        info!(
            "Extensions loaded: {} ({} failed)",
            loaded_names.len(),
            self.failures.len()
        );
        Ok(())
    }

    #[cfg(feature = "host")]
    fn load_one(
        &mut self,
        descriptor: &ExtensionDescriptor,
        core_api: &Shared<dyn CoreAPI>,
    ) -> Result<Box<dyn LoadedExtension>> {
        match descriptor.manifest.kind {
            ExtensionKind::Native => {
                #[cfg(feature = "native")]
                {
                    Ok(Box::new(NativeExtension::load(
                        descriptor,
                        core_api,
                        Arc::clone(&self.extension_state),
                    )?))
                }
                #[cfg(not(feature = "native"))]
                {
                    let _ = (descriptor, core_api);
                    anyhow::bail!("Native extension support was not compiled in")
                }
            }
            ExtensionKind::Wasm => {
                #[cfg(feature = "wasm")]
                {
                    Ok(Box::new(
                        self.wasm_runtime
                            .load(descriptor, Arc::clone(&self.extension_state))?,
                    ))
                }
                #[cfg(not(feature = "wasm"))]
                {
                    let _ = descriptor;
                    anyhow::bail!("WASM extension support was not compiled in")
                }
            }
        }
    }

    /// Shut down extensions in reverse load order.
    pub fn unload_all(&mut self) -> Result<()> {
        let mut first_error = None;
        while let Some(mut extension) = self.loaded.pop() {
            let name = extension.name().to_string();
            if let Err(error) = extension.shutdown() {
                warn!("Failed to shut down extension '{name}': {error:#}");
                if first_error.is_none() {
                    first_error = Some(error);
                }
            }
            luma_api::event_bus().unregister_owner(&name);
            luma_api::event_bus().fire(ExtensionUnloadedEvent { name });
        }
        match first_error {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }

    #[cfg(feature = "host")]
    fn record_failure(&mut self, name: impl Into<String>, error: impl std::fmt::Display) {
        self.failures.push(ExtensionLoadFailure {
            name: name.into(),
            error: format!("{error:#}"),
        });
    }

    #[cfg(feature = "host")]
    fn update_extension_state(&self, loaded_names: &HashSet<String>) {
        let mut state = self
            .extension_state
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.loaded = loaded_names.iter().cloned().collect();
        state.loaded.sort();
        state.failures = self.failure_strings();
    }

    #[cfg(feature = "host")]
    fn failure_strings(&self) -> Vec<String> {
        self.failures
            .iter()
            .map(|failure| format!("{}: {}", failure.name, failure.error))
            .collect()
    }

    pub fn loaded(&self) -> &[Box<dyn LoadedExtension>] {
        &self.loaded
    }

    pub fn failures(&self) -> &[ExtensionLoadFailure] {
        &self.failures
    }
}

impl Drop for ExtensionManager {
    fn drop(&mut self) {
        let _ = self.unload_all();
    }
}
