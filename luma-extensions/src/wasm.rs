//! WASM extension loader, powered by Wasmtime.
//!
//! A WASM module must export:
//!
//! - `luma_extension_initialize() -> i32` — returns 0 on success.
//! - `luma_extension_shutdown() -> i32` — optional, returns 0 on success.
//! - `memory` — used by the `luma.luma_log` host import and event delivery.
//! - `luma_event_alloc(len: i32) -> i32` and
//!   `luma_event_dispatch(name_ptr, name_len, payload_ptr, payload_len, cancelled_ptr) -> i32`
//!   — optional, required to receive host events.
//!
//! The host provides these imports in module `luma`:
//!
//! - `luma_log(level: i32, ptr: i32, len: i32)`
//! - `luma_abi_version() -> i32`
//! - `luma_loaded_extension_names_json_len() -> i32`
//! - `luma_loaded_extension_names_json(ptr: i32, len: i32) -> i32`
//! - `luma_extension_failures_json_len() -> i32`
//! - `luma_extension_failures_json(ptr: i32, len: i32) -> i32`
//! - `luma_event_register(name_ptr, name_len, priority, ignore_cancelled) -> i64`
//! - `luma_event_unregister(id: i64) -> i32`
//! - `luma_event_fire(name_ptr, name_len, payload_ptr, payload_len, cancelled_ptr) -> i32`
//!
//! Rust extensions can generate the exports with
//! [`declare_wasm_extension!`](crate::declare_wasm_extension) and implement
//! [`LumaExtension`](crate::extension::LumaExtension).

use std::sync::{Arc, Mutex, Weak};

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
use luma_api::event::{EventBus, EventPriority, HandlerId};
use luma_api::Shared;

/// Fuel budget for initialization (reset again for shutdown and event
/// delivery). Prevents an extension from hanging the IDE in an infinite loop.
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

        // The slot is filled with a weak reference after instantiation so the
        // guest can route host events back into its own event bus without
        // creating a strong reference cycle through the store's host state.
        let guest_slot: Arc<Mutex<Option<Weak<Mutex<WasmGuest>>>>> =
            Arc::new(Mutex::new(None));

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
        linker
            .func_wrap("luma", "luma_event_register", event_register)
            .context("Failed to register luma.luma_event_register import")?;
        linker
            .func_wrap("luma", "luma_event_unregister", event_unregister)
            .context("Failed to register luma.luma_event_unregister import")?;
        linker
            .func_wrap("luma", "luma_event_fire", event_fire)
            .context("Failed to register luma.luma_event_fire import")?;

        let mut store = Store::new(
            &self.engine,
            WasmHostState {
                name: name.clone(),
                extension_state,
                event_bus: luma_api::event_bus(),
                guest_slot: Arc::clone(&guest_slot),
            },
        );
        store
            .set_fuel(MAX_WASM_FUEL)
            .context("Failed to set WASM fuel budget")?;
        let instance = linker
            .instantiate(&mut store, &module)
            .with_context(|| format!("Failed to instantiate WASM extension '{name}'"))?;

        let memory = instance
            .get_memory(&mut store, "memory")
            .context("WASM extension does not export 'memory'")?;
        let guest = Arc::new(Mutex::new(WasmGuest {
            name: name.clone(),
            instance,
            store,
            memory,
            initialize: None,
            shutdown: None,
            alloc: None,
            dispatch: None,
            shutdown_called: false,
        }));
        {
            let mut guest = guest
                .lock()
                .map_err(|_| anyhow::anyhow!("WASM guest lock poisoned"))?;
            let WasmGuest {
                instance,
                store,
                initialize,
                shutdown,
                alloc,
                dispatch,
                ..
            } = &mut *guest;
            *initialize = Some(
                instance
                    .get_typed_func::<(), i32>(&mut *store, "luma_extension_initialize")
                    .context("WASM module is missing luma_extension_initialize")?,
            );
            *shutdown = instance
                .get_typed_func::<(), i32>(&mut *store, "luma_extension_shutdown")
                .ok();
            *alloc = instance
                .get_typed_func::<i32, i32>(&mut *store, "luma_event_alloc")
                .ok();
            *dispatch = instance
                .get_typed_func::<(i32, i32, i32, i32, i32), i32>(
                    &mut *store,
                    "luma_event_dispatch",
                )
                .ok();
        }
        if let Ok(mut slot) = guest_slot.lock() {
            *slot = Some(Arc::downgrade(&guest));
        }

        let status = {
            let mut guest = guest
                .lock()
                .map_err(|_| anyhow::anyhow!("WASM guest lock poisoned"))?;
            let WasmGuest {
                initialize, store, ..
            } = &mut *guest;
            initialize
                .as_ref()
                .context("WASM initialize function was not resolved")?
                .call(store, ())
                .with_context(|| format!("WASM extension '{name}' trapped during initialization"))?
        };
        if status != 0 {
            bail!("WASM extension '{name}' failed to initialize (status {status})");
        }

        Ok(WasmExtension {
            name,
            version: descriptor.manifest.version.clone(),
            guest,
        })
    }
}

pub struct WasmHostState {
    name: String,
    extension_state: ExtensionStateHandle,
    event_bus: Shared<EventBus>,
    guest_slot: Arc<Mutex<Option<Weak<Mutex<WasmGuest>>>>>,
}

type WasmEventDispatchFunc = TypedFunc<(i32, i32, i32, i32, i32), i32>;

/// A live WASM guest. The store is moved into this struct so host event
/// dispatch can call into the module after initialization has completed.
pub struct WasmGuest {
    name: String,
    instance: Instance,
    store: Store<WasmHostState>,
    memory: Memory,
    initialize: Option<TypedFunc<(), i32>>,
    shutdown: Option<TypedFunc<(), i32>>,
    alloc: Option<TypedFunc<i32, i32>>,
    dispatch: Option<WasmEventDispatchFunc>,
    shutdown_called: bool,
}

impl WasmGuest {
    /// Call a guest listener with a named event and return its cancelled flag.
    fn dispatch_event(&mut self, name: &str, payload: &serde_json::Value, cancelled: bool) -> Result<bool> {
        let alloc = self
            .alloc
            .as_ref()
            .context("WASM extension does not export luma_event_alloc")?;
        let dispatch = self
            .dispatch
            .as_ref()
            .context("WASM extension does not export luma_event_dispatch")?;

        let name_bytes = name.as_bytes();
        let payload_bytes = payload.to_string();
        let payload_bytes = payload_bytes.as_bytes();
        let cancel_offset = align_up(name_bytes.len() + payload_bytes.len(), 4);
        let total = cancel_offset + 4;
        let ptr = alloc
            .call(&mut self.store, total as i32)
            .context("luma_event_alloc trapped")?;
        if ptr <= 0 {
            bail!("luma_event_alloc returned an invalid pointer");
        }
        let ptr = ptr as usize;

        self.memory
            .write(&mut self.store, ptr, name_bytes)
            .context("Failed to write event name into guest memory")?;
        self.memory
            .write(&mut self.store, ptr + name_bytes.len(), payload_bytes)
            .context("Failed to write event payload into guest memory")?;
        self.memory
            .write(
                &mut self.store,
                ptr + cancel_offset,
                &(cancelled as u32).to_le_bytes(),
            )
            .context("Failed to write cancellation flag into guest memory")?;

        let status = dispatch
            .call(
                &mut self.store,
                (
                    ptr as i32,
                    name_bytes.len() as i32,
                    (ptr + name_bytes.len()) as i32,
                    payload_bytes.len() as i32,
                    (ptr + cancel_offset) as i32,
                ),
            )
            .with_context(|| format!("WASM extension '{}' trapped in event dispatch", self.name))?;
        if status != 0 {
            bail!(
                "WASM extension '{}' event dispatch failed (status {status})",
                self.name
            );
        }

        let mut bytes = [0u8; 4];
        self.memory
            .read(&mut self.store, ptr + cancel_offset, &mut bytes)
            .context("Failed to read cancellation flag from guest memory")?;
        Ok(u32::from_le_bytes(bytes) != 0)
    }
}

fn align_up(value: usize, alignment: usize) -> usize {
    (value + alignment - 1) & !(alignment - 1)
}

/// A loaded and initialized WASM extension.
pub struct WasmExtension {
    name: String,
    version: Version,
    guest: Arc<Mutex<WasmGuest>>,
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
        let mut guest = self
            .guest
            .lock()
            .map_err(|_| anyhow::anyhow!("WASM guest lock poisoned"))?;
        if guest.shutdown_called {
            return Ok(());
        }
        guest.shutdown_called = true;
        let Some(shutdown) = guest.shutdown.take() else {
            return Ok(());
        };
        guest
            .store
            .set_fuel(MAX_WASM_FUEL)
            .context("Failed to reset WASM fuel budget")?;
        let status = shutdown
            .call(&mut guest.store, ())
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
    let bytes = read_guest_bytes(&mut caller, ptr, len)?;
    let message = String::from_utf8_lossy(&bytes);
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

fn event_register(
    mut caller: Caller<'_, WasmHostState>,
    name_ptr: i32,
    name_len: i32,
    priority: i32,
    ignore_cancelled: i32,
) -> i64 {
    let result = (|| -> anyhow::Result<HandlerId> {
        let name_bytes = read_guest_bytes(&mut caller, name_ptr, name_len)?;
        let name = String::from_utf8(name_bytes).context("Invalid event name")?;
        let guest = caller
            .data()
            .guest_slot
            .lock()
            .ok()
            .and_then(|slot| slot.as_ref().and_then(Weak::upgrade))
            .context("WASM guest is not available for event delivery")?;
        let host_name = caller.data().name.clone();
        let bus = Arc::clone(&caller.data().event_bus);
        let callback_name = name.clone();

        Ok(bus.register_named(
            name,
            host_name,
            EventPriority::from_u8(priority.clamp(0, u8::MAX as i32) as u8),
            ignore_cancelled != 0,
            move |payload, context| {
                let cancelled = context.is_cancelled();
                match guest.try_lock() {
                    Ok(mut guest) => match guest.dispatch_event(&callback_name, payload, cancelled) {
                        Ok(cancelled) => context.set_cancelled(cancelled),
                        Err(error) => warn!("Failed to deliver '{callback_name}' to guest: {error:#}"),
                    },
                    Err(_) => warn!("Guest is busy while delivering '{callback_name}'"),
                }
            },
        ))
    })();
    match result {
        Ok(id) => id.as_u64() as i64,
        Err(error) => {
            warn!("luma_event_register failed: {error:#}");
            0
        }
    }
}

fn event_unregister(caller: Caller<'_, WasmHostState>, id: i64) -> i32 {
    if id < 0 {
        return 1;
    }
    caller.data().event_bus.unregister(HandlerId::from(id as u64));
    0
}

fn event_fire(
    mut caller: Caller<'_, WasmHostState>,
    name_ptr: i32,
    name_len: i32,
    payload_ptr: i32,
    payload_len: i32,
    cancelled_ptr: i32,
) -> i32 {
    let result = (|| -> anyhow::Result<()> {
        let name_bytes = read_guest_bytes(&mut caller, name_ptr, name_len)?;
        let payload_bytes = read_guest_bytes(&mut caller, payload_ptr, payload_len)?;
        let name = String::from_utf8(name_bytes).context("Invalid event name")?;
        let payload: serde_json::Value =
            serde_json::from_slice(&payload_bytes).context("Invalid event JSON payload")?;
        let initial = if cancelled_ptr < 0 {
            false
        } else {
            let mut bytes = [0u8; 4];
            let memory = caller
                .get_export("memory")
                .and_then(|export| export.into_memory())
                .context("WASM extension does not export 'memory'")?;
            memory
                .read(&caller, cancelled_ptr as usize, &mut bytes)
                .context("Invalid cancellation pointer")?;
            u32::from_le_bytes(bytes) != 0
        };
        let outcome = caller
            .data()
            .event_bus
            .fire_named_seeded(&name, payload, initial);
        if cancelled_ptr >= 0 {
            let memory = caller
                .get_export("memory")
                .and_then(|export| export.into_memory())
                .context("WASM extension does not export 'memory'")?;
            memory
                .write(&mut caller, cancelled_ptr as usize, &(outcome.cancelled as u32).to_le_bytes())
                .context("Invalid cancellation pointer")?;
        }
        Ok(())
    })();
    match result {
        Ok(()) => 0,
        Err(error) => {
            warn!("luma_event_fire failed: {error:#}");
            1
        }
    }
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

fn read_guest_bytes(
    caller: &mut Caller<'_, WasmHostState>,
    ptr: i32,
    len: i32,
) -> wasmtime::Result<Vec<u8>> {
    if ptr < 0 || len < 0 {
        return Err(wasmtime::format_err!(
            "Invalid guest pointer: ptr={ptr}, len={len}"
        ));
    }
    let memory = caller
        .get_export("memory")
        .and_then(|export| export.into_memory())
        .ok_or_else(|| wasmtime::format_err!("WASM extension does not export 'memory'"))?;
    let start = ptr as usize;
    let end = start
        .checked_add(len as usize)
        .ok_or_else(|| wasmtime::format_err!("Guest memory range overflow"))?;
    let bytes = memory
        .data(&*caller)
        .get(start..end)
        .ok_or_else(|| wasmtime::format_err!("Pointer is outside guest memory"))?;
    Ok(bytes.to_vec())
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
