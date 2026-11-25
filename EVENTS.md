# Luma Event Bus

Luma has a synchronous, Bukkit-style event bus in `luma-api`. It is shared by the
IDE core, native extensions, WASM extensions and the Vue frontend, so a listener
registered by an addon receives the same events as a listener registered in Rust
or TypeScript.

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
| `FileOpenedEvent` | `luma:file-opened` | no | Frontend `openFile`, when a tab is opened |
| `FileClosedEvent` | `luma:file-closed` | no | Frontend `closeFile` (tab X, Close Tab, Close All Tabs) |
| `FileActivatedEvent` | `luma:file-activated` | no | Frontend tab selection changes; `path` is `null` when no tab is active |
| `FileSavedEvent` | `luma:file-saved` | no | `write_file`, after the bytes are on disk |
| `FileCreatedEvent` | `luma:file-created` | no | `create_file` / `create_directory` |
| `FileDeletedEvent` | `luma:file-deleted` | no | `delete_entry`; `is_dir` distinguishes files and folders |
| `DirectoryOpenedEvent` | `luma:directory-opened` | no | `read_directory`, including explorer node expansion |
| `WorkspaceClosedEvent` | `luma:workspace-closed` | no | IDE window `Destroyed`, after the watcher stops |
| `TerminalOpenedEvent` | `luma:terminal-opened` | no | `terminal_open_session` |
| `TerminalClosedEvent` | `luma:terminal-closed` | no | Terminal close, shell exit or app shutdown (once per session) |
| Extension UI command | `luma:extension-command:<extension>.<command-id>` | no | Frontend command/menu invocation declared in `extension.json` |

File/tab/terminal events carry `window` (the Tauri window label) where the
action happened. `WorkspaceClosedEvent` carries the full `Workspace` that was
closed.

Cancelling `luma:workspace-open` aborts the workspace open flow and returns an
error to the caller. Configuration change handlers run while the configuration
manager is locked: do not call back into the same manager from a handler.

## Frontend bridge

`luma-tauri` exposes the bus to Vue through `src/vite/vue/composables/useEvents.ts`:

```ts
import { registerEventHandler, fireEvent } from "~/vue/composables/useEvents";

// Listen to any built-in or extension event.
const dispose = await registerEventHandler("luma:file-opened", {
  priority: "monitor",
  handler: (payload, meta) => console.log(payload.path, meta.cancelled),
});

// Fire a custom event; extensions and Rust handlers see it too.
const outcome = await fireEvent("my-addon:thing-happened", { id: 42 });
if (outcome.cancelled) console.log("a handler rejected it");

await dispose();
```

Under the hood:

- `event_register(name, priority, ignoreCancelled)` adds an ordinary named
  handler to the host bus. Matching events are emitted to that window as the
  Tauri event `luma-event` with `subscriptionId`, `name`, `payload` and
  `cancelled`.
- `event_unregister(id)` removes one subscription. All subscriptions for a
  window are removed when that window is destroyed.
- `event_fire(name, payload, cancelled)` dispatches by name and returns the
  final `cancelled` / `handlersCalled` synchronously.

Frontend subscriptions cannot synchronously cancel an event that Rust already
fired: the bus is synchronous and does not wait for a webview reply. The
`cancelled` value delivered to the frontend is therefore observational. Events
fired *from* the frontend can be cancelled by any Rust or extension handler,
and the final flag is returned to the caller.

Use `useLumaEvents()` inside a component to dispose its subscriptions
automatically on unmount.

Extension HTML panels use the same bus through a restricted `postMessage`
bridge. See [Extension frontend UI](EXTENSION_UI.md) for manifest declarations,
the generated `luma:extension-command:*` events and the sandboxing rules.

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
