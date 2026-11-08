use anyhow::Context;
use crate::settings_window::settings_window;

#[tauri::command]
pub fn webview_ready(window: tauri::WebviewWindow) -> Result<(), tauri::ipc::InvokeError> {
    log::info!("webview_ready label={}", window.label());

    window.show()
        .context(format!("Failed to show the {} window", window.label()))
        .map_err(tauri::ipc::InvokeError::from_anyhow)?;
    window.set_focus()
        .context(format!("Failed to focus the {} window", window.label()))
        .map_err(tauri::ipc::InvokeError::from_anyhow)?;

    Ok(())
}
#[tauri::command]
pub async fn open_settings_window(app_handle: tauri::AppHandle,window: tauri::WebviewWindow) -> Result<(), tauri::ipc::InvokeError> {
    let result = settings_window(app_handle, Some(&window))
        .context("Failed to initialize settings window")
        .context("Error opening settings window");

    if let Err(error) = &result {
        log::error!("{error:#}");
    }

    result.map_err(tauri::ipc::InvokeError::from_anyhow)?;

    Ok(())
}
