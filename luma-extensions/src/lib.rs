//! Extension loading for Luma IDE.
//!
//! Extensions are ZIP archives in `LUMA_DIR/extensions` with `extension.json`
//! at the archive root. Both native libraries (selected by host target triple)
//! and WASM modules are supported. See [`manifest`] for the manifest format.

pub mod abi;
pub mod extension;
pub mod guest;
pub mod manifest;
pub mod resolver;

#[cfg(feature = "host")]
pub mod discovery;
#[cfg(any(feature = "host", feature = "native", feature = "wasm"))]
pub mod manager;
#[cfg(feature = "native")]
pub mod native;
#[cfg(feature = "wasm")]
pub mod wasm;

#[cfg(feature = "host")]
use std::sync::{Arc, Mutex};

#[cfg(feature = "host")]
use anyhow::{Context, Result};
#[cfg(feature = "host")]
use log::info;
#[cfg(feature = "host")]
use luma_api::{CoreAPI, EXTENSIONS_API, ExtensionsCoreAPI, Shared};

pub use extension::LumaExtension;
pub use manifest::{ExtensionDescriptor, ExtensionEntry, ExtensionKind, ExtensionManifest};

#[cfg(any(feature = "host", feature = "native", feature = "wasm"))]
pub use manager::LoadedExtension;
#[cfg(feature = "host")]
pub use manager::{ExtensionLoadFailure, ExtensionManager};

#[cfg(feature = "host")]
use crate::discovery::{CACHE_DIR_NAME, default_extension_roots};

/// The extension subsystem, exposed to the rest of the IDE through
/// `luma_api::ExtensionsCoreAPI`.
#[cfg(feature = "host")]
pub struct ExtensionsCore {
    manager: Mutex<ExtensionManager>,
}

#[cfg(feature = "host")]
impl ExtensionsCore {
    fn new() -> Self {
        Self {
            manager: Mutex::new(ExtensionManager::new()),
        }
    }

    /// Create the API and publish it in `EXTENSIONS_API`.
    pub fn init() -> Result<Shared<dyn ExtensionsCoreAPI>> {
        let api: Shared<dyn ExtensionsCoreAPI> = Arc::new(Self::new());
        EXTENSIONS_API
            .set(Arc::clone(&api))
            .map_err(|_| anyhow::anyhow!("Extensions API already exists!"))?;
        Ok(api)
    }

    /// Access the manager from an `ExtensionsCore` instance.
    pub fn with_manager<T>(&self, f: impl FnOnce(&ExtensionManager) -> T) -> Result<T> {
        let manager = self
            .manager
            .lock()
            .map_err(|_| anyhow::anyhow!("Extension manager lock poisoned"))?;
        Ok(f(&manager))
    }
}

#[cfg(feature = "host")]
impl ExtensionsCoreAPI for ExtensionsCore {
    fn boot(&self, core_api: &Shared<dyn CoreAPI>) -> Result<()> {
        let roots = default_extension_roots(core_api.get_data_dir());
        let cache_dir = roots
            .first()
            .cloned()
            .unwrap_or_else(|| core_api.get_data_dir().join("extensions"))
            .join(CACHE_DIR_NAME);

        let mut manager = self
            .manager
            .lock()
            .map_err(|_| anyhow::anyhow!("Extension manager lock poisoned"))?;
        manager
            .load_all(core_api, roots, &cache_dir)
            .context("Failed to load extensions")?;
        Ok(())
    }

    fn loaded_extension_names(&self) -> Vec<String> {
        self.manager
            .lock()
            .map(|manager| {
                manager
                    .loaded()
                    .iter()
                    .map(|item| item.name().to_string())
                    .collect()
            })
            .unwrap_or_default()
    }

    fn extension_failures(&self) -> Vec<String> {
        self.manager
            .lock()
            .map(|manager| {
                manager
                    .failures()
                    .iter()
                    .map(|failure| format!("{}: {}", failure.name, failure.error))
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// Log from extension code, using the host logger in both native and WASM
/// extensions.
pub fn log(level: u32, message: &str) {
    #[cfg(target_arch = "wasm32")]
    guest::log_message(level, message);
    #[cfg(not(target_arch = "wasm32"))]
    abi::host_log_message(level, message);
}

/// Initialize the extension subsystem and boot all installed extensions.
#[cfg(feature = "host")]
pub fn init(core_api: &Shared<dyn CoreAPI>) -> Result<Shared<dyn ExtensionsCoreAPI>> {
    info!("Luma Extensions initialized");

    let extensions_api = ExtensionsCore::init().context("Failed to initialize Luma Extensions")?;
    extensions_api
        .boot(core_api)
        .context("Failed to boot Luma Extensions")?;

    Ok(extensions_api)
}
