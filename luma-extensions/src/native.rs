//! Loader for native extensions (Windows DLLs, Linux `.so`, macOS `.dylib`).

use std::ffi::c_void;
use std::path::Path;

use anyhow::{Context, Result, bail};
use libloading::Library;
use luma_api::{CoreAPI, Shared};
use semver::Version;

use crate::abi::{ABI_VERSION, ExtensionStateHandle, HostHandle, LumaHostApiV1, create_host_api};
use crate::manager::LoadedExtension;
use crate::manifest::{ExtensionDescriptor, ExtensionKind};

/// Rust target triple of the host process.
pub const HOST_TARGET: &str = env!("LUMA_HOST_TARGET");

type AbiVersionFn = unsafe extern "C" fn() -> u32;
type CreateFn = unsafe extern "C" fn(*const LumaHostApiV1) -> *mut c_void;
type InitializeFn = unsafe extern "C" fn(*mut c_void) -> u32;
type ShutdownFn = unsafe extern "C" fn(*mut c_void) -> u32;
type DestroyFn = unsafe extern "C" fn(*mut c_void);

/// A loaded native extension. Dropping it shuts down and unloads the library.
pub struct NativeExtension {
    name: String,
    version: Version,
    instance: *mut c_void,
    shutdown_fn: ShutdownFn,
    destroy_fn: DestroyFn,
    shutdown_called: bool,
    // Keep these alive until the instance has been destroyed.
    _host: HostHandle,
    _library: Library,
}

impl NativeExtension {
    /// Load, create and initialize the native library described by `descriptor`.
    pub fn load(
        descriptor: &ExtensionDescriptor,
        core_api: &Shared<dyn CoreAPI>,
        extension_state: ExtensionStateHandle,
    ) -> Result<Self> {
        let Some(library_path) = descriptor.native_library(HOST_TARGET) else {
            bail!(
                "Extension '{}' has no native entry for host target '{HOST_TARGET}'",
                descriptor.manifest.name
            );
        };
        Self::load_path(descriptor, &library_path, core_api, extension_state)
    }

    /// Load a specific native library (used by tests).
    pub fn load_path(
        descriptor: &ExtensionDescriptor,
        library_path: &Path,
        core_api: &Shared<dyn CoreAPI>,
        extension_state: ExtensionStateHandle,
    ) -> Result<Self> {
        if !library_path.is_file() {
            bail!(
                "Native extension library not found: {}",
                library_path.display()
            );
        }
        unsafe { Self::load_unchecked(descriptor, library_path, core_api, extension_state) }
    }

    unsafe fn load_unchecked(
        descriptor: &ExtensionDescriptor,
        library_path: &Path,
        core_api: &Shared<dyn CoreAPI>,
        extension_state: ExtensionStateHandle,
    ) -> Result<Self> {
        let library = unsafe { Library::new(library_path) }
            .with_context(|| format!("Failed to load {}", library_path.display()))?;

        let (abi_version, create, initialize, shutdown, destroy) = unsafe {
            let abi_version = *library
                .get::<AbiVersionFn>(b"luma_extension_abi_version\0")
                .context("Library is missing luma_extension_abi_version")?;
            let create = *library
                .get::<CreateFn>(b"luma_extension_create\0")
                .context("Library is missing luma_extension_create")?;
            let initialize = *library
                .get::<InitializeFn>(b"luma_extension_initialize\0")
                .context("Library is missing luma_extension_initialize")?;
            let shutdown = *library
                .get::<ShutdownFn>(b"luma_extension_shutdown\0")
                .context("Library is missing luma_extension_shutdown")?;
            let destroy = *library
                .get::<DestroyFn>(b"luma_extension_destroy\0")
                .context("Library is missing luma_extension_destroy")?;
            (abi_version, create, initialize, shutdown, destroy)
        };

        let extension_abi = unsafe { abi_version() };
        if extension_abi != ABI_VERSION {
            bail!(
                "Extension '{}' uses ABI v{extension_abi}, host expects v{ABI_VERSION}",
                descriptor.manifest.name
            );
        }

        let (host_api, host) = create_host_api(core_api, extension_state);
        let instance = unsafe { create(&host_api) };
        if instance.is_null() {
            bail!(
                "Extension '{}' failed to create its instance",
                descriptor.manifest.name
            );
        }

        let mut extension = Self {
            name: descriptor.manifest.name.clone(),
            version: descriptor.manifest.version.clone(),
            instance,
            shutdown_fn: shutdown,
            destroy_fn: destroy,
            shutdown_called: false,
            _host: host,
            _library: library,
        };

        let status = unsafe { initialize(instance) };
        if status != 0 {
            // Drop calls shutdown/destroy as well; mark it as done so a failed
            // initialize does not trigger a second shutdown.
            extension.shutdown_called = true;
            unsafe { (extension.destroy_fn)(extension.instance) };
            extension.instance = std::ptr::null_mut();
            bail!(
                "Extension '{}' failed to initialize (status {status})",
                descriptor.manifest.name
            );
        }

        Ok(extension)
    }
}

// SAFETY: the extension instance is only accessed through `LoadedExtension`
// (which takes `&mut self`) while the manager holding it is locked, so it is
// never aliased across threads. The native library stays loaded until drop.
unsafe impl Send for NativeExtension {}

impl LoadedExtension for NativeExtension {
    fn name(&self) -> &str {
        &self.name
    }

    fn kind(&self) -> ExtensionKind {
        ExtensionKind::Native
    }

    fn version(&self) -> &Version {
        &self.version
    }

    fn shutdown(&mut self) -> Result<()> {
        if self.shutdown_called {
            return Ok(());
        }
        self.shutdown_called = true;
        let status = unsafe { (self.shutdown_fn)(self.instance) };
        if status == 0 {
            Ok(())
        } else {
            bail!(
                "Extension '{}' failed to shut down (status {status})",
                self.name
            )
        }
    }
}

impl Drop for NativeExtension {
    fn drop(&mut self) {
        if self.instance.is_null() {
            return;
        }
        if !self.shutdown_called {
            unsafe { (self.shutdown_fn)(self.instance) };
        }
        unsafe { (self.destroy_fn)(self.instance) };
        self.instance = std::ptr::null_mut();
    }
}
