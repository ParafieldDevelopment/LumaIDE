//! Guest-side helpers for extensions compiled to WASM.
//!
//! This module has no Wasmtime dependency, so WASM extensions can depend on
//! `luma-extensions` with `default-features = false` and still use the same
//! [`LumaExtension`] trait as native extensions.

use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};

use anyhow::Result;
use luma_api::configuration::ConfigurationManager;
use luma_api::event::{EventBridge, EventBus, EventPriority};
use luma_api::{CORE_API, CoreAPI, EXTENSIONS_API, ExtensionsCoreAPI, Shared};
use serde_json::Value;

use crate::abi::ABI_VERSION;
use crate::extension::LumaExtension;

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "luma")]
unsafe extern "C" {
    // Avoid the plain name `log`: Rust's `compiler_builtins` also exports a
    // `log` symbol (f64 -> f64) on wasm32, which would clash at link time.
    fn luma_log(level: i32, ptr: *const u8, len: usize);
    fn luma_abi_version() -> i32;
    fn luma_loaded_extension_names_json_len() -> i32;
    fn luma_loaded_extension_names_json(ptr: *mut u8, len: usize) -> i32;
    fn luma_extension_failures_json_len() -> i32;
    fn luma_extension_failures_json(ptr: *mut u8, len: usize) -> i32;
    fn luma_event_register(
        name_ptr: *const u8,
        name_len: usize,
        priority: i32,
        ignore_cancelled: i32,
    ) -> i64;
    fn luma_event_unregister(id: i64) -> i32;
    fn luma_event_fire(
        name_ptr: *const u8,
        name_len: usize,
        payload_ptr: *const u8,
        payload_len: usize,
        cancelled_ptr: *mut u32,
    ) -> i32;
}

#[cfg(not(target_arch = "wasm32"))]
mod fallback {
    use super::*;
    pub(super) unsafe fn log(level: i32, ptr: *const u8, len: usize) {
        let message = String::from_utf8_lossy(unsafe { std::slice::from_raw_parts(ptr, len) });
        match level {
            1 => log::error!("[wasm extension] {message}"),
            2 => log::warn!("[wasm extension] {message}"),
            4 => log::debug!("[wasm extension] {message}"),
            5 => log::trace!("[wasm extension] {message}"),
            _ => log::info!("[wasm extension] {message}"),
        }
    }
    pub(super) unsafe fn abi_version() -> i32 {
        ABI_VERSION as i32
    }
    pub(super) unsafe fn loaded_extension_names_json_len() -> i32 {
        0
    }
    pub(super) unsafe fn loaded_extension_names_json(_ptr: *mut u8, _len: usize) -> i32 {
        -1
    }
    pub(super) unsafe fn extension_failures_json_len() -> i32 {
        0
    }
    pub(super) unsafe fn extension_failures_json(_ptr: *mut u8, _len: usize) -> i32 {
        -1
    }
    pub(super) unsafe fn event_register(
        _name_ptr: *const u8,
        _name_len: usize,
        _priority: i32,
        _ignore_cancelled: i32,
    ) -> i64 {
        0
    }
    pub(super) unsafe fn event_unregister(_id: i64) -> i32 {
        0
    }
    pub(super) unsafe fn event_fire(
        _name_ptr: *const u8,
        _name_len: usize,
        _payload_ptr: *const u8,
        _payload_len: usize,
        _cancelled_ptr: *mut u32,
    ) -> i32 {
        0
    }
}

static INSTANCE: Mutex<Option<Box<dyn LumaExtension>>> = Mutex::new(None);

/// Write a log message through the host's `luma.luma_log` import.
pub fn log_message(level: u32, message: &str) {
    unsafe { host_log(level as i32, message.as_ptr(), message.len()) };
}

#[cfg(target_arch = "wasm32")]
unsafe fn host_log(level: i32, ptr: *const u8, len: usize) {
    unsafe { luma_log(level, ptr, len) }
}

#[cfg(not(target_arch = "wasm32"))]
unsafe fn host_log(level: i32, ptr: *const u8, len: usize) {
    unsafe { fallback::log(level, ptr, len) }
}

fn host_abi_version() -> i32 {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        luma_abi_version()
    }
    #[cfg(not(target_arch = "wasm32"))]
    unsafe {
        fallback::abi_version()
    }
}

#[cfg(target_arch = "wasm32")]
fn loaded_names_len() -> i32 {
    unsafe { luma_loaded_extension_names_json_len() }
}

#[cfg(not(target_arch = "wasm32"))]
fn loaded_names_len() -> i32 {
    unsafe { fallback::loaded_extension_names_json_len() }
}

#[cfg(target_arch = "wasm32")]
fn loaded_names_write(ptr: *mut u8, len: usize) -> i32 {
    unsafe { luma_loaded_extension_names_json(ptr, len) }
}

#[cfg(not(target_arch = "wasm32"))]
fn loaded_names_write(ptr: *mut u8, len: usize) -> i32 {
    unsafe { fallback::loaded_extension_names_json(ptr, len) }
}

#[cfg(target_arch = "wasm32")]
fn failures_len() -> i32 {
    unsafe { luma_extension_failures_json_len() }
}

#[cfg(not(target_arch = "wasm32"))]
fn failures_len() -> i32 {
    unsafe { fallback::extension_failures_json_len() }
}

#[cfg(target_arch = "wasm32")]
fn failures_write(ptr: *mut u8, len: usize) -> i32 {
    unsafe { luma_extension_failures_json(ptr, len) }
}

#[cfg(not(target_arch = "wasm32"))]
fn failures_write(ptr: *mut u8, len: usize) -> i32 {
    unsafe { fallback::extension_failures_json(ptr, len) }
}

#[cfg(target_arch = "wasm32")]
fn event_register(name: &str, priority: EventPriority, ignore_cancelled: bool) -> i64 {
    unsafe {
        luma_event_register(
            name.as_ptr(),
            name.len(),
            u8::from(priority) as i32,
            ignore_cancelled as i32,
        )
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn event_register(name: &str, priority: EventPriority, ignore_cancelled: bool) -> i64 {
    unsafe {
        fallback::event_register(
            name.as_ptr(),
            name.len(),
            u8::from(priority) as i32,
            ignore_cancelled as i32,
        )
    }
}

fn event_unregister(id: i64) -> i32 {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        luma_event_unregister(id)
    }
    #[cfg(not(target_arch = "wasm32"))]
    unsafe {
        fallback::event_unregister(id)
    }
}

#[cfg(target_arch = "wasm32")]
fn event_fire(name: &str, payload: &str, cancelled: &mut u32) -> i32 {
    unsafe {
        luma_event_fire(
            name.as_ptr(),
            name.len(),
            payload.as_ptr(),
            payload.len(),
            cancelled as *mut u32,
        )
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn event_fire(name: &str, payload: &str, cancelled: &mut u32) -> i32 {
    unsafe {
        fallback::event_fire(
            name.as_ptr(),
            name.len(),
            payload.as_ptr(),
            payload.len(),
            cancelled as *mut u32,
        )
    }
}

/// Bridges a guest-side [`EventBus`] to the host bus through WASM imports.
struct WasmEventBridge;

impl EventBridge for WasmEventBridge {
    fn register(&self, name: &str, priority: EventPriority, ignore_cancelled: bool) -> Option<u64> {
        let id = event_register(name, priority, ignore_cancelled);
        (id > 0).then_some(id as u64)
    }

    fn unregister(&self, remote_id: u64) {
        event_unregister(remote_id as i64);
    }

    fn fire(&self, name: &str, payload: &Value, cancelled: bool) -> bool {
        let payload = payload.to_string();
        let mut cancelled = cancelled as u32;
        let status = event_fire(name, &payload, &mut cancelled);
        if status != 0 {
            log_message(2, &format!("Host rejected event '{name}' (status {status})"));
        }
        cancelled != 0
    }
}

fn loaded_extension_names() -> Vec<String> {
    host_json_array(loaded_names_len, loaded_names_write)
}

fn extension_failures() -> Vec<String> {
    host_json_array(failures_len, failures_write)
}

fn host_json_array(
    len: impl FnOnce() -> i32,
    write: impl FnOnce(*mut u8, usize) -> i32,
) -> Vec<String> {
    let len = len();
    if len <= 0 {
        return Vec::new();
    }
    let mut buffer = vec![0u8; len as usize];
    if write(buffer.as_mut_ptr(), buffer.len()) != 0 {
        return Vec::new();
    }
    serde_json::from_slice(&buffer).unwrap_or_default()
}

/// Create and initialize a WASM extension. Call this from the module's
/// `luma_extension_initialize` export; returns 0 on success.
pub fn initialize_extension<T>() -> i32
where
    T: LumaExtension + Default + 'static,
{
    if host_abi_version() != ABI_VERSION as i32 {
        log_message(1, &format!("Host ABI mismatch: expected {ABI_VERSION}"));
        return 1;
    }
    install_guest_apis();
    let Ok((core_api, extensions_core)) = extension_apis() else {
        return 1;
    };

    let mut instance = T::create(&core_api, &extensions_core);
    if let Err(error) = instance.initialize(&core_api, &extensions_core) {
        log_message(1, &format!("initialize failed: {error:#}"));
        return 1;
    }

    match INSTANCE.lock() {
        Ok(mut slot) => {
            *slot = Some(Box::new(instance));
            0
        }
        Err(_) => {
            log_message(1, "Extension instance lock poisoned");
            1
        }
    }
}

/// Shut down the current WASM extension instance. Returns 0 on success.
pub fn shutdown_extension() -> i32 {
    let taken = match INSTANCE.lock() {
        Ok(mut slot) => slot.take(),
        Err(_) => {
            log_message(1, "Extension instance lock poisoned");
            return 1;
        }
    };
    let Some(mut instance) = taken else {
        return 0;
    };
    match instance.shutdown() {
        Ok(()) => 0,
        Err(error) => {
            log_message(1, &format!("shutdown failed: {error:#}"));
            1
        }
    }
}

fn install_guest_apis() {
    let core_api: Shared<dyn CoreAPI> = Arc::new(WasmCoreApi {
        ide_configuration: Arc::new(RwLock::new(ConfigurationManager::new())),
        workspace_configuration: Arc::new(RwLock::new(ConfigurationManager::new())),
        workspace_callbacks: RwLock::new(Vec::new()),
        event_bus: Arc::new(EventBus::with_bridge(Arc::new(WasmEventBridge))),
    });
    let extensions_core: Shared<dyn ExtensionsCoreAPI> = Arc::new(WasmExtensionsCoreApi);
    let _ = CORE_API.set(core_api);
    let _ = EXTENSIONS_API.set(extensions_core);
}

fn extension_apis() -> Result<(Shared<dyn CoreAPI>, Shared<dyn ExtensionsCoreAPI>)> {
    let core_api = CORE_API
        .get()
        .ok_or_else(|| anyhow::anyhow!("Guest host API is not installed"))?;
    let extensions_core = EXTENSIONS_API
        .get()
        .ok_or_else(|| anyhow::anyhow!("Guest extensions API is not installed"))?;
    Ok((Arc::clone(core_api), Arc::clone(extensions_core)))
}

/// Limited `CoreAPI` available inside a WASM guest. The host does not expose
/// filesystem paths to WASM modules; use the C ABI (native extensions) when
/// full host integration is required.
struct WasmCoreApi {
    ide_configuration: Shared<RwLock<ConfigurationManager>>,
    workspace_configuration: Shared<RwLock<ConfigurationManager>>,
    workspace_callbacks: RwLock<Vec<Box<dyn Fn() + Send + Sync>>>,
    event_bus: Shared<EventBus>,
}

impl CoreAPI for WasmCoreApi {
    fn get_data_dir(&self) -> &PathBuf {
        // WASM guests are sandboxed and get no host directories.
        static EMPTY: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
        EMPTY.get_or_init(PathBuf::new)
    }

    fn get_configuration_dir(&self) -> &PathBuf {
        self.get_data_dir()
    }

    fn get_workspaces_dir(&self) -> &PathBuf {
        self.get_data_dir()
    }

    fn boot(&self) -> Result<()> {
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

    fn open_workspace(&self, path: &std::path::Path) -> Result<()> {
        log_message(
            2,
            &format!("open_workspace is unavailable in WASM: {}", path.display()),
        );
        Ok(())
    }

    fn get_ide_configuration(&self) -> &Shared<RwLock<ConfigurationManager>> {
        &self.ide_configuration
    }

    fn get_workspace_configuration(&self) -> &Shared<RwLock<ConfigurationManager>> {
        &self.workspace_configuration
    }

    fn event_bus(&self) -> &Shared<EventBus> {
        &self.event_bus
    }
}

struct WasmExtensionsCoreApi;

impl ExtensionsCoreAPI for WasmExtensionsCoreApi {
    fn boot(&self, _core_api: &Shared<dyn CoreAPI>) -> Result<()> {
        Ok(())
    }

    fn loaded_extension_names(&self) -> Vec<String> {
        loaded_extension_names()
    }

    fn extension_failures(&self) -> Vec<String> {
        extension_failures()
    }
}

/// Scratch buffer used by the host to hand event data to the guest. The host
/// calls [`luma_event_alloc`] once, fills it, then calls
/// [`luma_event_dispatch`] with pointers into it.
#[cfg(target_arch = "wasm32")]
static EVENT_BUFFER: Mutex<Vec<u8>> = Mutex::new(Vec::new());

/// Reserve guest memory for an incoming named event. Returns a pointer or 0.
#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn luma_event_alloc(len: i32) -> i32 {
    if len < 0 {
        return 0;
    }
    let mut buffer = EVENT_BUFFER
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    buffer.resize(len as usize, 0);
    buffer.as_mut_ptr() as i32
}

/// Deliver a named event from the host to listeners registered on the guest's
/// bridged event bus. `cancelled_ptr` holds the incoming/outgoing flag.
#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn luma_event_dispatch(
    name_ptr: i32,
    name_len: i32,
    payload_ptr: i32,
    payload_len: i32,
    cancelled_ptr: i32,
) -> i32 {
    if name_ptr < 0 || name_len < 0 || payload_ptr < 0 || payload_len < 0 {
        return 1;
    }
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let name_bytes = unsafe { read_guest_bytes(name_ptr, name_len) };
        let payload_bytes = unsafe { read_guest_bytes(payload_ptr, payload_len) };
        let name = String::from_utf8_lossy(&name_bytes);
        let mut cancelled = cancelled_ptr >= 0 && unsafe { *(cancelled_ptr as *const u32) } != 0;
        if let Ok(payload) = serde_json::from_slice::<Value>(&payload_bytes) {
            let bus = CORE_API
                .get()
                .map(|api| Arc::clone(api.event_bus()))
                .unwrap_or_else(luma_api::event_bus);
            cancelled = bus.fire_named_seeded(&name, payload, cancelled).cancelled;
        }
        if cancelled_ptr >= 0 {
            unsafe { *(cancelled_ptr as *mut u32) = cancelled as u32 };
        }
    }));
    if result.is_ok() {
        0
    } else {
        1
    }
}

#[cfg(target_arch = "wasm32")]
unsafe fn read_guest_bytes(ptr: i32, len: i32) -> Vec<u8> {
    unsafe { std::slice::from_raw_parts(ptr as *const u8, len as usize).to_vec() }
}

/// Generate the WASM module exports for an extension.
#[macro_export]
macro_rules! declare_wasm_extension {
    ($ty:ty) => {
        #[unsafe(no_mangle)]
        pub extern "C" fn luma_extension_initialize() -> i32 {
            $crate::guest::initialize_extension::<$ty>()
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn luma_extension_shutdown() -> i32 {
            $crate::guest::shutdown_extension()
        }
    };
}
