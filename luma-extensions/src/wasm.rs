//! WASM extension loader, powered by Wasmtime.
//!
//! A WASM module must export:
//!
//! - `luma_extension_initialize() -> i32` — returns 0 on success.
//! - `luma_extension_shutdown() -> i32` — optional, returns 0 on success.
//! - `memory` — used by the `luma.luma_log` host import.
//!
//! The host provides these imports in module `luma`:
//!
//! - `luma_log(level: i32, ptr: i32, len: i32)`
//! - `luma_abi_version() -> i32`
//! - `luma_loaded_extension_names_json_len() -> i32`
//! - `luma_loaded_extension_names_json(ptr: i32, len: i32) -> i32`
//! - `luma_extension_failures_json_len() -> i32`
//! - `luma_extension_failures_json(ptr: i32, len: i32) -> i32`
//!
//! Rust extensions can generate the exports with
//! [`declare_wasm_extension!`](crate::declare_wasm_extension) and implement
//! [`LumaExtension`](crate::extension::LumaExtension).

use anyhow::{Result, bail};
use log::{debug, error, info, trace, warn};
use semver::Version;
use wasmtime::error::Context;
use wasmtime::{Caller, Config, Engine, Instance, Linker, Memory, Module, Store, TypedFunc};

use crate::abi::{
    ABI_VERSION, ExtensionState, ExtensionStateHandle, LOG_DEBUG, LOG_ERROR, LOG_TRACE, LOG_WARN,
};
use crate::manager::LoadedExtension;
use crate::manifest::{ExtensionDescriptor, ExtensionKind};

/// Fuel budget for initialization (reset again for shutdown). Prevents an
/// extension from hanging IDE startup or shutdown in an infinite loop.
const MAX_WASM_FUEL: u64 = 50_000_000;

/// Shared Wasmtime engine. Reuse it so compiled modules can be cached.
pub struct WasmRuntime {
    engine: Engine,
}

impl Default for WasmRuntime {
    fn default() -> Self {
        let mut config = Config::new();
        config.consume_fuel(true);
        Self {
            engine: Engine::new(&config).expect("Failed to create Wasmtime engine"),
        }
    }
}

impl WasmRuntime {
    pub fn new() -> Result<Self> {
        Ok(Self::default())
    }

    /// Load and initialize a WASM module described by `descriptor`.
    pub fn load(
        &self,
        descriptor: &ExtensionDescriptor,
        extension_state: ExtensionStateHandle,
    ) -> Result<WasmExtension> {
        let module_path = descriptor
            .wasm_module()
            .ok_or_else(|| anyhow::anyhow!("Manifest does not describe a WASM extension"))?;
        if !module_path.is_file() {
            bail!("WASM module not found: {}", module_path.display());
        }
        let name = descriptor.manifest.name.clone();
        let module = Module::from_file(&self.engine, &module_path)
            .context("Failed to compile WASM module")?;

        let mut linker = Linker::<WasmHostState>::new(&self.engine);
        linker
            .func_wrap("luma", "luma_log", log_import)
            .context("Failed to register luma.luma_log import")?;
        linker
            .func_wrap("luma", "luma_abi_version", || ABI_VERSION as i32)
            .context("Failed to register luma.luma_abi_version import")?;
        linker
            .func_wrap(
                "luma",
                "luma_loaded_extension_names_json_len",
                loaded_extension_names_json_len,
            )
            .context("Failed to register luma.luma_loaded_extension_names_json_len import")?;
        linker
            .func_wrap(
                "luma",
                "luma_loaded_extension_names_json",
                write_loaded_extension_names_json,
            )
            .context("Failed to register luma.luma_loaded_extension_names_json import")?;
        linker
            .func_wrap(
                "luma",
                "luma_extension_failures_json_len",
                extension_failures_json_len,
            )
            .context("Failed to register luma.luma_extension_failures_json_len import")?;
        linker
            .func_wrap(
                "luma",
                "luma_extension_failures_json",
                write_extension_failures_json,
            )
            .context("Failed to register luma.luma_extension_failures_json import")?;

        let mut store = Store::new(
            &self.engine,
            WasmHostState {
                name: name.clone(),
                extension_state,
            },
        );
        store
            .set_fuel(MAX_WASM_FUEL)
            .context("Failed to set WASM fuel budget")?;
        let instance = linker
            .instantiate(&mut store, &module)
            .with_context(|| format!("Failed to instantiate WASM extension '{name}'"))?;

        let initialize = instance
            .get_typed_func::<(), i32>(&mut store, "luma_extension_initialize")
            .context("WASM module is missing luma_extension_initialize")?;
        let shutdown = instance
            .get_typed_func::<(), i32>(&mut store, "luma_extension_shutdown")
            .ok();

        let status = initialize
            .call(&mut store, ())
            .with_context(|| format!("WASM extension '{name}' trapped during initialization"))?;
        if status != 0 {
            bail!("WASM extension '{name}' failed to initialize (status {status})");
        }

        Ok(WasmExtension {
            name,
            version: descriptor.manifest.version.clone(),
            _instance: instance,
            store,
            shutdown,
            shutdown_called: false,
        })
    }
}

pub struct WasmHostState {
    name: String,
    extension_state: ExtensionStateHandle,
}

/// A loaded and initialized WASM extension.
pub struct WasmExtension {
    name: String,
    version: Version,
    // Keep the instance handle alive for as long as the store is alive.
    _instance: Instance,
    store: Store<WasmHostState>,
    shutdown: Option<TypedFunc<(), i32>>,
    shutdown_called: bool,
}

impl LoadedExtension for WasmExtension {
    fn name(&self) -> &str {
        &self.name
    }

    fn kind(&self) -> ExtensionKind {
        ExtensionKind::Wasm
    }

    fn version(&self) -> &Version {
        &self.version
    }

    fn shutdown(&mut self) -> Result<()> {
        if self.shutdown_called {
            return Ok(());
        }
        self.shutdown_called = true;
        let Some(shutdown) = &self.shutdown else {
            return Ok(());
        };
        self.store
            .set_fuel(MAX_WASM_FUEL)
            .context("Failed to reset WASM fuel budget")?;
        let status = shutdown
            .call(&mut self.store, ())
            .with_context(|| format!("WASM extension '{}' trapped during shutdown", self.name))?;
        if status == 0 {
            Ok(())
        } else {
            bail!(
                "WASM extension '{}' failed to shut down (status {status})",
                self.name
            )
        }
    }
}

fn log_import(
    mut caller: Caller<'_, WasmHostState>,
    level: i32,
    ptr: i32,
    len: i32,
) -> wasmtime::Result<()> {
    if ptr < 0 || len < 0 {
        return Err(wasmtime::format_err!(
            "Invalid luma.luma_log arguments: ptr={ptr}, len={len}"
        ));
    }
    let memory = caller
        .get_export("memory")
        .and_then(|export| export.into_memory())
        .ok_or_else(|| wasmtime::format_err!("WASM extension does not export 'memory'"))?;
    let start = ptr as usize;
    let end = start
        .checked_add(len as usize)
        .ok_or_else(|| wasmtime::format_err!("luma.luma_log range overflow"))?;
    let bytes = memory
        .data(&caller)
        .get(start..end)
        .ok_or_else(|| wasmtime::format_err!("luma.luma_log pointer is outside guest memory"))?;
    let message = String::from_utf8_lossy(bytes);
    let name = caller.data().name.clone();
    match level as u32 {
        LOG_ERROR => error!("[{name}] {message}"),
        LOG_WARN => warn!("[{name}] {message}"),
        LOG_DEBUG => debug!("[{name}] {message}"),
        LOG_TRACE => trace!("[{name}] {message}"),
        _ => info!("[{name}] {message}"),
    }
    Ok(())
}

fn loaded_extension_names_json_len(caller: Caller<'_, WasmHostState>) -> i32 {
    state_json_len(&caller, |state| &state.loaded)
}

fn extension_failures_json_len(caller: Caller<'_, WasmHostState>) -> i32 {
    state_json_len(&caller, |state| &state.failures)
}

fn write_loaded_extension_names_json(
    mut caller: Caller<'_, WasmHostState>,
    ptr: i32,
    len: i32,
) -> i32 {
    let json = state_json(&caller, |state| &state.loaded);
    write_guest_json(&mut caller, ptr, len, &json)
}

fn write_extension_failures_json(mut caller: Caller<'_, WasmHostState>, ptr: i32, len: i32) -> i32 {
    let json = state_json(&caller, |state| &state.failures);
    write_guest_json(&mut caller, ptr, len, &json)
}

fn state_json_len(
    caller: &Caller<'_, WasmHostState>,
    select: impl FnOnce(&ExtensionState) -> &Vec<String>,
) -> i32 {
    state_json(caller, select).len() as i32
}

fn state_json(
    caller: &Caller<'_, WasmHostState>,
    select: impl FnOnce(&ExtensionState) -> &Vec<String>,
) -> String {
    let state = caller
        .data()
        .extension_state
        .read()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    serde_json::to_string(select(&state)).unwrap_or_else(|_| "[]".to_string())
}

fn write_guest_json(caller: &mut Caller<'_, WasmHostState>, ptr: i32, len: i32, json: &str) -> i32 {
    if ptr < 0 || len < 0 || json.len() > len as usize {
        return 1;
    }
    let Some(memory): Option<Memory> = caller
        .get_export("memory")
        .and_then(|export| export.into_memory())
    else {
        return 2;
    };
    match memory.write(&mut *caller, ptr as usize, json.as_bytes()) {
        Ok(()) => 0,
        Err(_) => 3,
    }
}
