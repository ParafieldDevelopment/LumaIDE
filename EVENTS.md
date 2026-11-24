# Luma Event Bus

Luma has a synchronous, Bukkit-style event bus in `luma-api`. It is shared by the
IDE core, native extensions and WASM extensions, so a listener registered by an
addon receives the same events as a listener registered in Rust.

## Semantics

| Feature | Behaviour |
| --- | --- |
| Priorities | `Lowest → Low → Normal → High → Highest → Monitor` (same order as Bukkit). |
| Cancellation | `context.set_cancelled(true)`; later handlers with `ignore_cancelled = true` are skipped. `Monitor` handlers observe the final state but cannot change it. |
| Propagation | `context.stop_propagation()` skips all remaining handlers, including bridged ones. |
| Isolation | A panicking handler is logged and does not stop the remaining handlers. |
| Registration | `HandlerId` allows single removal; `unregister_owner(owner)` removes all handlers belonging to an extension. |
| Payloads | Typed Rust handlers receive `&E`; extension handlers receive JSON. All events are `Serialize + DeserializeOwned`. |

There is no asynchronous dispatch yet. Handlers run on the thread that fires
the event; keep them short.

## Rust (host or extension)

```rust
use luma_api::event::{Event, EventBus, EventContext, EventPriority};
use luma_api::events::WORKSPACE_LOADED_EVENT;
use luma_api::events::WorkspaceLoadedEvent;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct MyEvent { message: String }

impl Event for MyEvent {
    fn name() -> &'static str { "my-addon:my-event" }
}

fn register(bus: &EventBus) {
    // Typed handler. "my-addon" is the owner used by unregister_owner().
    bus.register::<WorkspaceLoadedEvent, _>(
        "my-addon", EventPriority::Normal, false,
        |event, _context| println!("opened {}", event.workspace.face().path),
    );

    // Named handler (also used for custom events from other addons).
    bus.register_named(
        WORKSPACE_LOADED_EVENT,
        "my-addon", EventPriority::Monitor, false,
        |payload, context| {
            if payload["workspace"]["face"]["path"].is_null() {
                context.set_cancelled(true);
            }
        },
    );
}

fn fire(bus: &EventBus) {
    let outcome = bus.fire(MyEvent { message: "hello".into() });
    if outcome.cancelled {
        // A handler rejected the event.
    }
}
```

`luma_api::event_bus()` returns the process-wide host bus. Inside an extension,
use `core_api.event_bus()` (or `CORE_API.get()`) instead: that bus is bridged to
the host and automatically relays registrations and fired events across the
native ABI or WASM import boundary.

## Built-in events

All names are namespaced with `luma:` and defined in `luma-api/src/events.rs`.

| Event | Name | Cancellable | Fired from |
| --- | --- | --- | --- |
| `WorkspaceOpenEvent` | `luma:workspace-open` | yes | `Workspace::load`, before configuration is loaded |
| `WorkspaceLoadedEvent` | `luma:workspace-loaded` | no | `Workspace::load`, before legacy `on_workspace_load` callbacks |
| `ExtensionLoadedEvent` | `luma:extension-loaded` | no | `ExtensionManager::load_all`, after each extension initializes |
| `ExtensionUnloadedEvent` | `luma:extension-unloaded` | no | `ExtensionManager::unload_all`, after each extension shuts down |
| `ConfigurationChangedEvent` | `luma:configuration-changed` | no | `ConfigurationManager::set` and `reset` |
| `FilesChangedEvent` | `luma:files-changed` | no | Tauri filesystem watcher, after debouncing (all windows) |

Cancelling `luma:workspace-open` aborts the workspace open flow and returns an
error to the caller. Configuration change handlers run while the configuration
manager is locked: do not call back into the same manager from a handler.

## Extension ABI

The native host API (`LumaHostApiV1`, ABI **v3**) adds:

- `event_register(name, priority, ignore_cancelled, callback, userdata) -> u64`
- `event_unregister(id) -> u32`
- `event_fire(name, payload_json, cancelled_in_out) -> u32`

WASM guests get equivalent `luma` module imports (`luma_event_register`,
`luma_event_unregister`, `luma_event_fire`). `declare_wasm_extension!` supplies
the matching `luma_event_alloc` / `luma_event_dispatch` exports used by the host
to deliver events into the guest. Extensions built against ABI v1/v2 must be
rebuilt.

The host removes a guest's listeners when the extension unloads, even if the
guest never calls `unregister`. Native listeners are also released when the
host context drops.
