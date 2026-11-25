# Extension Frontend Contributions

Extensions can contribute UI when Luma loads a workspace. They do **not** get
arbitrary JavaScript execution inside the main Vue webview. Instead, an
`extension.json` manifest declares commands, menu items and tool windows, and
optional HTML assets are rendered in a sandboxed iframe. Extension logic in
Rust, native code or WASM communicates with those panels through the shared
[event bus](EVENTS.md).

## Manifest example

```json
{
  "name": "my-addon",
  "version": "0.1.0",
  "type": "wasm",
  "entry": "my_addon.wasm",
  "frontend": {
    "entry": "frontend/index.html",
    "contributions": {
      "commands": [
        {
          "id": "openPanel",
          "label": "My Addon: Open Panel",
          "category": "My Addon",
          "keys": "Alt+Shift+M",
          "icon": "M4 4h16v16H4z"
        }
      ],
      "menus": [
        {
          "location": "titlebar/main",
          "commandId": "openPanel",
          "group": "09_my-addon",
          "order": 10
        }
      ],
      "toolWindows": [
        {
          "id": "panel",
          "title": "My Addon",
          "position": "right-bottom",
          "icon": "frontend/icon.svg",
          "order": 40,
          "hotkey": "Alt+Shift+P",
          "defaultOpen": false
        }
      ],
      "events": ["luma:file-opened", "my-addon:button-clicked"]
    }
  }
}
```

Paths are relative to the extension root and must stay inside it. Ids are
namespaced automatically: the command above is registered in the frontend as
`my-addon.openPanel`, and its tool window as `my-addon.panel`.

Every declared command also gets a bus event:

```
luma:extension-command:<extension>.<command-id>
```

Invoking the action fires that event with:

```json
{
  "extension": "my-addon",
  "command": "my-addon.openPanel",
  "workspace": { "path": "...", "title": "..." }
}
```

A native or WASM extension can listen for that event with the normal event bus
API and update state, write a file, or emit another event that the iframe is
allowed to receive.

## Frontend code

`frontend/index.html` can be a plain HTML/CSS/JS app or a built Vue/Svelte/etc.
bundle. Ship all of its assets in the extension archive and reference them with
relative URLs. Luma loads the entry through Tauri's asset protocol into an
iframe with `sandbox="allow-scripts"` (no `allow-same-origin`).

The iframe communicates with Luma using `postMessage`:

```js
const SOURCE = "luma-extension-frame";

window.parent.postMessage(
  {
    source: SOURCE,
    direction: "iframe-to-host",
    event: "my-addon:button-clicked",
    payload: { clicked: true },
  },
  "*",
);

window.addEventListener("message", (message) => {
  const data = message.data;
  if (data?.source !== SOURCE || data.direction !== "host-to-iframe") return;

  if (data.event === "luma:extension-ui:workspace") {
    console.log("workspace:", data.payload?.path);
  } else {
    console.log(data.event, data.payload, data.cancelled);
  }
});
```

Only names listed under `frontend.contributions.events` are forwarded in either
direction. The host also adds each command's generated event to that allowlist.
Iframe-to-host messages become ordinary bus events via `fireEvent`, so Rust,
native extensions, WASM extensions and other Vue components receive them.

On load, Luma pushes the current workspace to the iframe with the internal
control event `luma:extension-ui:workspace`, and repeats it whenever
`workspace-opened` fires.

See `extensions/example_extension/frontend/index.html` for a working example.

## Why iframes instead of direct Vue component injection?

Directly evaluating extension JavaScript in the main webview would let a buggy
or malicious extension read workspace data, call Tauri IPC, and mutate any Vue
component. This design keeps extension UI isolated while still using Luma's
real registries:

- contributed commands appear in the command palette and keyboard shortcuts;
- contributed menus appear wherever that menu location is rendered;
- contributed tool windows use the same docking/layout system as built-in tools;
- panels can use any frontend framework inside their own sandboxed document;
- all shell/extension communication remains on the typed event bus.

The host draws the tool window chrome and passes declarative props; extensions
cannot replace or patch arbitrary Vue components. This is deliberate: it keeps
the shell stable while still allowing complete custom UIs inside extension
panels.

## Limitations and security notes

- Extension assets must be under the Tauri asset protocol scopes configured in
  `luma-tauri/tauri.conf.json` (normally the Luma application data directory).
- Sandboxed panels cannot use Luma's Tauri IPC directly. Ask for data or actions
  with an allowed bus event instead.
- Event forwarding is asynchronous. A panel cannot synchronously cancel an
  event that Rust already dispatched.
- The event allowlist protects the host bridge, but a declared event is trusted
  enough to be fired into the shared bus. Only declare events your extension
  actually needs.
- Tool-window layout is keyed by the namespaced id. Renaming an extension,
  command or tool window resets its saved placement.
