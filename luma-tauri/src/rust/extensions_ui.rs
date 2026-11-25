//! Tauri commands exposing validated extension UI contributions to the frontend.
//!
//! Extensions are loaded before any webview exists, so the Vue app pulls this
//! information on mount instead of relying on an early bus event.

use luma_api::EXTENSIONS_API;
use luma_extensions::FrontendContribution;

/// UI declarations from loaded extensions.
#[tauri::command]
pub fn list_extension_ui() -> Vec<FrontendContribution> {
    let Some(api) = EXTENSIONS_API.get() else {
        return Vec::new();
    };
    serde_json::from_str(&api.frontend_contributions_json()).unwrap_or_default()
}

/// Absolute path of an extension asset, when it exists inside the extension root.
#[tauri::command]
pub fn extension_asset_path(extension: String, path: String) -> Option<String> {
    EXTENSIONS_API
        .get()
        .and_then(|api| api.extension_asset_path(&extension, &path))
        .map(|path| path.to_string_lossy().into_owned())
}
