//! Stable C ABI shared by the host and native (DLL/cdylib) extensions.
//!
//! Rust traits and `Arc<dyn Trait>` cannot cross a dynamic library boundary
//! safely, so the host passes a table of plain function pointers instead.
//! The extension loader keeps the opaque host context alive for as long as the
//! extension is loaded.
//!
//! Symbols a native extension must export:
//!
//! | Symbol | Signature | Description |
//! | --- | --- | --- |
//! | `luma_extension_abi_version` | `fn() -> u32` | Must return [`ABI_VERSION`] |
//! | `luma_extension_create` | `fn(*const LumaHostApiV1) -> *mut c_void` | Create an instance |
//! | `luma_extension_initialize` | `fn(*mut c_void) -> u32` | Initialize the instance |
//! | `luma_extension_shutdown` | `fn(*mut c_void) -> u32` | Shut the instance down |
//! | `luma_extension_destroy` | `fn(*mut c_void)` | Drop the instance |
//!
//! Use [`declare_luma_extension!`](crate::declare_luma_extension) to generate
//! all of them from a [`LumaExtension`](crate::extension::LumaExtension) type.

use std::ffi::{CString, c_char, c_void};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use anyhow::{Context, Result, bail};
use log::{debug, error, info, trace, warn};
use luma_api::configuration::{
    ConfigurationManager, ConfigurationScope, ConfigurationValue, Identifier,
};
use luma_api::{CORE_API, CoreAPI, EXTENSIONS_API, ExtensionsCoreAPI, Shared};

/// Current version of the native extension ABI. Bump on breaking changes.
pub const ABI_VERSION: u32 = 2;

pub const LOG_ERROR: u32 = 1;
pub const LOG_WARN: u32 = 2;
pub const LOG_INFO: u32 = 3;
pub const LOG_DEBUG: u32 = 4;
pub const LOG_TRACE: u32 = 5;

/// Function table supplied by the host to `luma_extension_create`.
///
/// Every function takes the opaque `context` pointer supplied in this struct.
/// Strings are UTF-8 byte slices (`ptr` + `len`), not NUL-terminated. Strings
/// returned by the host are NUL-terminated and must be released with
/// `free_string`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct LumaHostApiV1 {
    pub version: u32,
    pub context: *const c_void,
    pub log: unsafe extern "C" fn(*const c_void, u32, *const u8, usize),
    pub data_dir: unsafe extern "C" fn(*const c_void) -> *mut c_char,
    pub configuration_dir: unsafe extern "C" fn(*const c_void) -> *mut c_char,
    pub workspaces_dir: unsafe extern "C" fn(*const c_void) -> *mut c_char,
    /// Read a configuration value as JSON. Returns 0 on success.
    pub config_get: unsafe extern "C" fn(*const c_void, *const u8, usize, *mut *mut c_char) -> u32,
    /// Write a JSON configuration value. Returns 0 on success.
    pub config_set: unsafe extern "C" fn(*const c_void, *const u8, usize, *const u8, usize) -> u32,
    /// Open a workspace (calls `CoreAPI::open_workspace`). Returns 0 on success.
    pub open_workspace: unsafe extern "C" fn(*const c_void, *const u8, usize) -> u32,
    /// Write a JSON array of loaded extension names into `out`; returns its length.
    pub loaded_extensions: unsafe extern "C" fn(*const c_void, *mut *mut c_char) -> usize,
    /// Write a JSON array of extension failures into `out`; returns its length.
    pub extension_failures: unsafe extern "C" fn(*const c_void, *mut *mut c_char) -> usize,
    pub free_string: unsafe extern "C" fn(*const c_void, *mut c_char),
}

/// Snapshot of extension state exposed to extensions during initialization.
#[derive(Debug, Default, Clone)]
pub struct ExtensionState {
    pub loaded: Vec<String>,
    pub failures: Vec<String>,
}

/// Shared, atomic snapshot used by the host loader and extension imports.
pub type ExtensionStateHandle = Arc<RwLock<ExtensionState>>;

/// Owns the opaque context pointer referenced by a [`LumaHostApiV1`].
pub type HostHandle = Arc<HostContext>;

pub struct HostContext {
    core_api: Shared<dyn CoreAPI>,
    extension_state: ExtensionStateHandle,
}

/// Build a host function table for a loaded extension. Keep the returned
/// [`HostHandle`] alive for the entire lifetime of that extension.
pub fn create_host_api(
    core_api: &Shared<dyn CoreAPI>,
    extension_state: ExtensionStateHandle,
) -> (LumaHostApiV1, HostHandle) {
    let handle = Arc::new(HostContext {
        core_api: Arc::clone(core_api),
        extension_state,
    });
    let api = LumaHostApiV1 {
        version: ABI_VERSION,
        context: Arc::as_ptr(&handle) as *const c_void,
        log: host_log,
        data_dir: host_data_dir,
        configuration_dir: host_configuration_dir,
        workspaces_dir: host_workspaces_dir,
        config_get: host_config_get,
        config_set: host_config_set,
        open_workspace: host_open_workspace,
        loaded_extensions: host_loaded_extensions,
        extension_failures: host_extension_failures,
        free_string: host_free_string,
    };
    (api, handle)
}

impl LumaHostApiV1 {
    /// Log a message to the host's logger.
    ///
    /// # Safety
    /// The function table must come from [`create_host_api`] and its context
    /// handle must still be alive.
    pub unsafe fn log_message(&self, level: u32, message: &str) {
        unsafe {
            (self.log)(self.context, level, message.as_ptr(), message.len());
        }
    }

    /// # Safety
    /// See [`LumaHostApiV1::log_message`].
    pub unsafe fn data_directory(&self) -> PathBuf {
        unsafe { self.read_path(self.data_dir) }
    }

    /// # Safety
    /// See [`LumaHostApiV1::log_message`].
    pub unsafe fn configuration_directory(&self) -> PathBuf {
        unsafe { self.read_path(self.configuration_dir) }
    }

    /// # Safety
    /// See [`LumaHostApiV1::log_message`].
    pub unsafe fn workspaces_directory(&self) -> PathBuf {
        unsafe { self.read_path(self.workspaces_dir) }
    }

    unsafe fn read_path(
        &self,
        getter: unsafe extern "C" fn(*const c_void) -> *mut c_char,
    ) -> PathBuf {
        let ptr = unsafe { getter(self.context) };
        if ptr.is_null() {
            return PathBuf::new();
        }
        let path = unsafe { std::ffi::CStr::from_ptr(ptr) }
            .to_string_lossy()
            .into_owned();
        unsafe { self.free_string(ptr) };
        PathBuf::from(path)
    }

    /// Read a configuration value as JSON.
    ///
    /// # Safety
    /// See [`LumaHostApiV1::log_message`].
    pub unsafe fn configuration(&self, id: &str) -> Result<String> {
        let mut out: *mut c_char = std::ptr::null_mut();
        let status = unsafe { (self.config_get)(self.context, id.as_ptr(), id.len(), &mut out) };
        if status != 0 {
            bail!("Configuration '{id}' could not be read (status {status})");
        }
        if out.is_null() {
            bail!("Configuration '{id}' was not found");
        }
        let value = unsafe { std::ffi::CStr::from_ptr(out) }
            .to_string_lossy()
            .into_owned();
        unsafe { self.free_string(out) };
        Ok(value)
    }

    /// Write a JSON configuration value.
    ///
    /// # Safety
    /// See [`LumaHostApiV1::log_message`].
    pub unsafe fn set_configuration(&self, id: &str, json_value: &str) -> Result<()> {
        let status = unsafe {
            (self.config_set)(
                self.context,
                id.as_ptr(),
                id.len(),
                json_value.as_ptr(),
                json_value.len(),
            )
        };
        if status != 0 {
            bail!("Configuration '{id}' could not be written (status {status})");
        }
        Ok(())
    }

    /// Names of extensions loaded before this one.
    ///
    /// # Safety
    /// See [`LumaHostApiV1::log_message`].
    pub unsafe fn loaded_extension_names(&self) -> Vec<String> {
        unsafe { self.read_string_array(self.loaded_extensions) }
    }

    /// `name: error` descriptions of extensions that failed before this one.
    ///
    /// # Safety
    /// See [`LumaHostApiV1::log_message`].
    pub unsafe fn extension_failures(&self) -> Vec<String> {
        unsafe { self.read_string_array(self.extension_failures) }
    }

    unsafe fn read_string_array(
        &self,
        getter: unsafe extern "C" fn(*const c_void, *mut *mut c_char) -> usize,
    ) -> Vec<String> {
        let mut out: *mut c_char = std::ptr::null_mut();
        let _count = unsafe { getter(self.context, &mut out) };
        if out.is_null() {
            return Vec::new();
        }
        let json = unsafe { std::ffi::CStr::from_ptr(out) }
            .to_string_lossy()
            .into_owned();
        unsafe { self.free_string(out) };
        serde_json::from_str(&json).unwrap_or_default()
    }

    /// # Safety
    /// `ptr` must have been returned by one of this table's string functions.
    pub unsafe fn free_string(&self, ptr: *mut c_char) {
        if !ptr.is_null() {
            unsafe { (self.free_string)(self.context, ptr) };
        }
    }
}

// The context and function pointers are owned by the loader and stay valid
// for as long as the extension instance exists.
pub struct SharedHostApi(pub LumaHostApiV1);
unsafe impl Send for SharedHostApi {}
unsafe impl Sync for SharedHostApi {}

static INSTALLED_HOST: std::sync::OnceLock<SharedHostApi> = std::sync::OnceLock::new();

/// Log through the host ABI. Falls back to the local logger before the host is
/// installed (e.g. while a native extension is being created).
pub fn host_log_message(level: u32, message: &str) {
    if let Some(host) = INSTALLED_HOST.get() {
        unsafe { host.0.log_message(level, message) };
    } else {
        match level {
            LOG_ERROR => error!("[extension] {message}"),
            LOG_WARN => warn!("[extension] {message}"),
            LOG_DEBUG => debug!("[extension] {message}"),
            LOG_TRACE => trace!("[extension] {message}"),
            _ => info!("[extension] {message}"),
        }
    }
}

/// Host-side implementation of `CoreAPI` inside an extension process.
///
/// Native extensions can depend on all `luma-*` crates. The globals
/// (`CORE_API`, `EXTENSIONS_API`) are populated by
/// [`install_extension_host`] so those crates see the host's API.
pub struct NativeCoreApi {
    api: SharedHostApi,
    data_dir: PathBuf,
    configuration_dir: PathBuf,
    workspaces_dir: PathBuf,
    ide_configuration: Shared<RwLock<ConfigurationManager>>,
    workspace_configuration: Shared<RwLock<ConfigurationManager>>,
    workspace_callbacks: RwLock<Vec<Box<dyn Fn() + Send + Sync>>>,
}

impl NativeCoreApi {
    /// Build a `CoreAPI` view over a host function table.
    ///
    /// # Safety
    /// The table must be valid (see [`LumaHostApiV1::log_message`]).
    pub unsafe fn from_host(host: LumaHostApiV1) -> Result<Self> {
        if host.version != ABI_VERSION {
            bail!(
                "Unsupported host ABI version {} (expected {ABI_VERSION})",
                host.version
            );
        }
        Ok(Self {
            data_dir: unsafe { host.data_directory() },
            configuration_dir: unsafe { host.configuration_directory() },
            workspaces_dir: unsafe { host.workspaces_directory() },
            api: SharedHostApi(host),
            ide_configuration: Arc::new(RwLock::new(ConfigurationManager::new())),
            workspace_configuration: Arc::new(RwLock::new(ConfigurationManager::new())),
            workspace_callbacks: RwLock::new(Vec::new()),
        })
    }
}

impl CoreAPI for NativeCoreApi {
    fn get_data_dir(&self) -> &PathBuf {
        &self.data_dir
    }

    fn get_configuration_dir(&self) -> &PathBuf {
        &self.configuration_dir
    }

    fn get_workspaces_dir(&self) -> &PathBuf {
        &self.workspaces_dir
    }

    fn boot(&self) -> Result<()> {
        // The real core has already booted before extensions are loaded.
        Ok(())
    }

    fn on_workspace_load(&self, callback: Box<dyn Fn() + Send + Sync>) {
        if let Ok(mut callbacks) = self.workspace_callbacks.write() {
            callbacks.push(callback);
        }
    }

    fn notify_workspace_loaded(&self) {
        luma_api::notify_listeners(&self.workspace_callbacks);
    }

    fn open_workspace(&self, workspace_path: &std::path::Path) -> Result<()> {
        let host = self.api.0;
        let path = workspace_path.to_string_lossy();
        let status =
            unsafe { (host.open_workspace)(host.context, path.as_bytes().as_ptr(), path.len()) };
        if status == 0 {
            Ok(())
        } else {
            bail!(
                "Host rejected workspace '{}' (status {status})",
                workspace_path.display()
            )
        }
    }

    fn get_ide_configuration(&self) -> &Shared<RwLock<ConfigurationManager>> {
        &self.ide_configuration
    }

    fn get_workspace_configuration(&self) -> &Shared<RwLock<ConfigurationManager>> {
        &self.workspace_configuration
    }
}

/// Minimal `ExtensionsCoreAPI` available inside a native extension.
pub struct NativeExtensionsCoreApi {
    api: SharedHostApi,
}

impl ExtensionsCoreAPI for NativeExtensionsCoreApi {
    fn boot(&self, _core_api: &Shared<dyn CoreAPI>) -> Result<()> {
        Ok(())
    }

    fn loaded_extension_names(&self) -> Vec<String> {
        unsafe { self.api.0.loaded_extension_names() }
    }

    fn extension_failures(&self) -> Vec<String> {
        unsafe { self.api.0.extension_failures() }
    }
}

/// Install host APIs into this library copy's globals and return the trait
/// objects to pass to `LumaExtension::initialize`.
///
/// # Safety
/// The table must be valid (see [`LumaHostApiV1::log_message`]).
pub unsafe fn install_extension_host(
    host: LumaHostApiV1,
) -> Result<(Shared<dyn CoreAPI>, Shared<dyn ExtensionsCoreAPI>)> {
    let _ = INSTALLED_HOST.set(SharedHostApi(host));
    let core_api: Shared<dyn CoreAPI> = Arc::new(unsafe { NativeCoreApi::from_host(host) }?);
    let extensions_core: Shared<dyn ExtensionsCoreAPI> = Arc::new(NativeExtensionsCoreApi {
        api: SharedHostApi(host),
    });
    let _ = CORE_API.set(Arc::clone(&core_api));
    let _ = EXTENSIONS_API.set(Arc::clone(&extensions_core));
    Ok((core_api, extensions_core))
}

/// Fetch the APIs installed by [`install_extension_host`].
pub fn extension_apis() -> Result<(Shared<dyn CoreAPI>, Shared<dyn ExtensionsCoreAPI>)> {
    let core_api = CORE_API
        .get()
        .ok_or_else(|| anyhow::anyhow!("Extension host API is not installed"))?;
    let extensions_core = EXTENSIONS_API
        .get()
        .ok_or_else(|| anyhow::anyhow!("Extensions API is not installed"))?;
    Ok((Arc::clone(core_api), Arc::clone(extensions_core)))
}

// ---------------------------------------------------------------------------
// Host-side implementations of the function table
// ---------------------------------------------------------------------------

/// Log a failure reported through the extension ABI.
pub fn report_extension_error(phase: &str, error: impl std::fmt::Display) {
    // A cdylib has its own `log` globals, so the host logger may never see a
    // local `log::error!`. Always report through the host function table.
    host_log_message(LOG_ERROR, &format!("{phase} failed: {error:#}"));
}

unsafe extern "C" fn host_log(_context: *const c_void, level: u32, ptr: *const u8, len: usize) {
    let Some(message) = (unsafe { read_str(ptr, len) }) else {
        return;
    };
    match level {
        LOG_ERROR => error!("[extension] {message}"),
        LOG_WARN => warn!("[extension] {message}"),
        LOG_DEBUG => debug!("[extension] {message}"),
        LOG_TRACE => trace!("[extension] {message}"),
        _ => info!("[extension] {message}"),
    }
}

unsafe extern "C" fn host_data_dir(context: *const c_void) -> *mut c_char {
    unsafe { path_from_context(context, |core| core.get_data_dir().clone()) }
}

unsafe extern "C" fn host_configuration_dir(context: *const c_void) -> *mut c_char {
    unsafe { path_from_context(context, |core| core.get_configuration_dir().clone()) }
}

unsafe extern "C" fn host_workspaces_dir(context: *const c_void) -> *mut c_char {
    unsafe { path_from_context(context, |core| core.get_workspaces_dir().clone()) }
}

unsafe fn path_from_context(
    context: *const c_void,
    get: impl FnOnce(&Shared<dyn CoreAPI>) -> PathBuf,
) -> *mut c_char {
    let Some(host_context) = (unsafe { context_cast(context) }) else {
        return std::ptr::null_mut();
    };
    to_c_string(get(&host_context.core_api).to_string_lossy())
}

unsafe extern "C" fn host_config_get(
    context: *const c_void,
    id_ptr: *const u8,
    id_len: usize,
    out: *mut *mut c_char,
) -> u32 {
    if out.is_null() {
        return 1;
    }
    let result = (|| -> Result<String> {
        let id = unsafe { read_str(id_ptr, id_len) }.context("Invalid configuration id")?;
        let host_context = unsafe { context_cast(context) }.context("Invalid host context")?;
        let manager = host_context
            .core_api
            .get_ide_configuration()
            .read()
            .map_err(|e| anyhow::anyhow!("Configuration lock poisoned: {e}"))?;
        let value = manager.get(Identifier::parse(&id)?)?;
        serde_json::to_string(&value).context("Failed to serialize configuration value")
    })();
    match result {
        Ok(json) => {
            unsafe { *out = to_c_string(json) };
            0
        }
        Err(error) => {
            warn!("[extension] config_get failed: {error:#}");
            1
        }
    }
}

unsafe extern "C" fn host_config_set(
    context: *const c_void,
    id_ptr: *const u8,
    id_len: usize,
    value_ptr: *const u8,
    value_len: usize,
) -> u32 {
    let result = (|| -> Result<()> {
        let id = unsafe { read_str(id_ptr, id_len) }.context("Invalid configuration id")?;
        let json =
            unsafe { read_str(value_ptr, value_len) }.context("Invalid configuration value")?;
        let value: ConfigurationValue =
            serde_json::from_str(&json).context("Invalid configuration JSON value")?;
        let host_context = unsafe { context_cast(context) }.context("Invalid host context")?;
        let mut manager = host_context
            .core_api
            .get_ide_configuration()
            .write()
            .map_err(|e| anyhow::anyhow!("Configuration lock poisoned: {e}"))?;
        manager.set(Identifier::parse(&id)?, value, ConfigurationScope::Global)
    })();
    match result {
        Ok(()) => 0,
        Err(error) => {
            warn!("[extension] config_set failed: {error:#}");
            1
        }
    }
}

unsafe extern "C" fn host_open_workspace(
    context: *const c_void,
    ptr: *const u8,
    len: usize,
) -> u32 {
    let result = (|| -> Result<()> {
        let path = unsafe { read_str(ptr, len) }.context("Invalid workspace path")?;
        let host_context = unsafe { context_cast(context) }.context("Invalid host context")?;
        host_context
            .core_api
            .open_workspace(std::path::Path::new(&path))
    })();
    match result {
        Ok(()) => 0,
        Err(error) => {
            warn!("[extension] open_workspace failed: {error:#}");
            1
        }
    }
}

unsafe extern "C" fn host_loaded_extensions(
    context: *const c_void,
    out: *mut *mut c_char,
) -> usize {
    write_string_array(context, out, |state| &state.loaded)
}

unsafe extern "C" fn host_extension_failures(
    context: *const c_void,
    out: *mut *mut c_char,
) -> usize {
    write_string_array(context, out, |state| &state.failures)
}

fn write_string_array(
    context: *const c_void,
    out: *mut *mut c_char,
    select: impl FnOnce(&ExtensionState) -> &Vec<String>,
) -> usize {
    let result = (|| -> Result<Vec<String>> {
        let context = unsafe { context_cast(context) }.context("Invalid host context")?;
        let state = context
            .extension_state
            .read()
            .map_err(|e| anyhow::anyhow!("Extension state lock poisoned: {e}"))?;
        Ok(select(&state).clone())
    })();
    match result {
        Ok(values) => {
            if !out.is_null() {
                let json = serde_json::to_string(&values).unwrap_or_else(|_| "[]".to_string());
                unsafe { *out = to_c_string(json) };
            }
            values.len()
        }
        Err(error) => {
            warn!("[extension] extension state read failed: {error:#}");
            0
        }
    }
}

unsafe extern "C" fn host_free_string(_context: *const c_void, ptr: *mut c_char) {
    if !ptr.is_null() {
        unsafe { drop(CString::from_raw(ptr)) };
    }
}

unsafe fn context_cast(context: *const c_void) -> Option<&'static HostContext> {
    if context.is_null() {
        return None;
    }
    // SAFETY: callers only invoke table functions while the loader-owned
    // HostHandle (an Arc<HostContext>) is alive.
    unsafe { Some(&*(context as *const HostContext)) }
}

unsafe fn read_str(ptr: *const u8, len: usize) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    let bytes = unsafe { std::slice::from_raw_parts(ptr, len) };
    String::from_utf8(bytes.to_vec()).ok()
}

fn to_c_string(value: impl AsRef<str>) -> *mut c_char {
    CString::new(value.as_ref().as_bytes())
        .unwrap_or_default()
        .into_raw()
}

/// Generate the C ABI exports for a native extension.
///
/// ```ignore
/// use luma_extensions::declare_luma_extension;
/// declare_luma_extension!(MyExtension);
/// ```
///
/// `MyExtension` must implement [`LumaExtension`](crate::extension::LumaExtension)
/// and `Default` (or override `LumaExtension::create`).
#[macro_export]
macro_rules! declare_luma_extension {
    ($ty:ty) => {
        const _: fn() = || {
            fn assert_luma_extension<T: $crate::extension::LumaExtension>() {}
            assert_luma_extension::<$ty>();
        };

        #[unsafe(no_mangle)]
        pub extern "C" fn luma_extension_abi_version() -> u32 {
            $crate::abi::ABI_VERSION
        }

        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn luma_extension_create(
            host: *const $crate::abi::LumaHostApiV1,
        ) -> *mut ::std::ffi::c_void {
            if host.is_null() {
                return ::std::ptr::null_mut();
            }
            let host = unsafe { *host };
            let (core_api, extensions_core) =
                match unsafe { $crate::abi::install_extension_host(host) } {
                    Ok(apis) => apis,
                    Err(error) => {
                        $crate::abi::host_log_message(
                            $crate::abi::LOG_ERROR,
                            &format!("Failed to install host API: {error:#}"),
                        );
                        return ::std::ptr::null_mut();
                    }
                };
            let instance =
                <$ty as $crate::extension::LumaExtension>::create(&core_api, &extensions_core);
            Box::into_raw(Box::new(instance)) as *mut ::std::ffi::c_void
        }

        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn luma_extension_initialize(
            instance: *mut ::std::ffi::c_void,
        ) -> u32 {
            if instance.is_null() {
                return 1;
            }
            let instance = unsafe { &mut *(instance as *mut $ty) };
            let Ok((core_api, extensions_core)) = $crate::abi::extension_apis() else {
                return 1;
            };
            match $crate::extension::LumaExtension::initialize(
                instance,
                &core_api,
                &extensions_core,
            ) {
                Ok(()) => 0,
                Err(error) => {
                    $crate::abi::report_extension_error("initialize", error);
                    1
                }
            }
        }

        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn luma_extension_shutdown(instance: *mut ::std::ffi::c_void) -> u32 {
            if instance.is_null() {
                return 1;
            }
            let instance = unsafe { &mut *(instance as *mut $ty) };
            match $crate::extension::LumaExtension::shutdown(instance) {
                Ok(()) => 0,
                Err(error) => {
                    $crate::abi::report_extension_error("shutdown", error);
                    1
                }
            }
        }

        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn luma_extension_destroy(instance: *mut ::std::ffi::c_void) {
            if !instance.is_null() {
                unsafe { drop(Box::from_raw(instance as *mut $ty)) };
            }
        }
    };
}
