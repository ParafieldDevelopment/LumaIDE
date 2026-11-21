# Luma extensions

An extension is a ZIP archive installed in `LUMA_DIR/extensions` (on Windows,
`%APPDATA%/Luma/extensions`). **Everything lives at the root of the archive**:

```
example-extension.zip
├── extension.json
├── native/
│   └── x86_64-pc-windows-msvc/
│       └── example_extension.dll
└── (optional auxiliary files)
```

The loader extracts each ZIP into `LUMA_DIR/extensions/.cache/<name>-<hash>/`.
The hash comes from the archive bytes, so replacing a ZIP automatically
produces a fresh extraction. Archives are validated against zip-slip and
symlink attacks. Unpacked directories containing `extension.json` are also
accepted for local development.

Additional search directories can be set with the `LUMA_EXTENSIONS_PATH`
environment variable (platform path separator between entries).

## `extension.json`

| Field | Required | Description |
| --- | --- | --- |
| `name` | yes | Unique identifier: letters, digits, `.`, `_`, `-`. |
| `description` | no | Human-readable description. |
| `version` | yes | Semver, e.g. `1.2.3` or `0.2.0-beta.1`. |
| `type` | yes | `"native"` or `"wasm"`. |
| `entry` | yes | Native: map of target triple → library path. WASM: one module path. |
| `dependencies` | no | `[{ "name": "...", "version": "^1.0.0", "optional": false }]`; `version` defaults to `*`. |
| `authors` | no | String list. |
| `license` | no | SPDX identifier. |

All paths are relative to the archive root. Native entries are selected with
the host target triple (`x86_64-pc-windows-msvc`, `aarch64-apple-darwin`,
`x86_64-unknown-linux-gnu`, …). Dependencies are validated for presence and
semver compatibility and loaded first; cycles, missing required dependencies
and version mismatches are errors. Optional unsatisfied dependencies are
skipped with a warning. A failing extension is reported and skipped without
preventing other extensions from loading.

## Extension API

Extensions can depend on all `luma-*` crates. For extension crates, depend on
`luma-extensions` with `default-features = false`: the host-side ZIP extraction
(and its `wasm-bindgen` dependency on `wasm32`) is not needed inside a guest.
Implement `luma_extensions::LumaExtension` and export it with:

- `luma_extensions::declare_luma_extension!(MyExtension)` for native DLLs.
  This generates the stable C ABI symbols `luma_extension_abi_version`,
  `luma_extension_create`, `luma_extension_initialize`,
  `luma_extension_shutdown` and `luma_extension_destroy`. Native extensions
  receive a function table (`LumaHostApiV1`) that bridges logging, directories,
  configuration and workspace opening into `luma_api::CoreAPI`.
- `luma_extensions::declare_wasm_extension!(MyExtension)` for WASM modules.
  The host imports `luma.luma_log`, `luma.luma_abi_version` and the extension
  state imports from module `luma`; the module exports
  `luma_extension_initialize` and `luma_extension_shutdown`
  (WASM guests get a sandboxed subset of `CoreAPI`; use native for full host
  integration).

## Building and testing locally (Windows)

```powershell
# Packages example-extension.zip and installs it in %APPDATA%/Luma/extensions
powershell -File extensions/example_extension/pack.ps1

# Also build a WASM variant (requires rustup target add wasm32-unknown-unknown)
powershell -File extensions/example_extension/pack.ps1 -Wasm
```

CI (`.github/workflows/build-extensions.yml`) builds native ZIPs for Windows
x86_64/aarch64 and a WASM ZIP, then uploads both individual archives and a
combined `luma-extensions.zip`. The combined archive contains the individual
extension ZIPs at its root; extract it into `LUMA_DIR/extensions` rather than
dropping the combined file in directly. The native matrix is structured so
Linux and macOS targets can be added by adding entries (and the corresponding
library filenames) to the matrix.
