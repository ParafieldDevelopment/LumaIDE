use anyhow::Context;
use tauri::{Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

use crate::constants::STARTUP_COLOR;

pub const STARTUP_LABEL: &str = "startup";

pub fn startup_window(app: tauri::AppHandle) -> anyhow::Result<WebviewWindow> {
    if let Some(window) = app.get_webview_window(STARTUP_LABEL) {
        window
            .set_focus()
            .context("Failed to focus the startup window")?;
        return Ok(window);
    }

    let builder = WebviewWindowBuilder::new(
        &app,
        STARTUP_LABEL,
        WebviewUrl::App("windows/startup.html".into()),
    )
    .visible(false)
    .decorations(!cfg!(target_os = "windows"))
    .min_inner_size(750.0, 500.0)
    .title("Luma - Workspace Selection")
    .focused(true)
    .background_color(STARTUP_COLOR);

    #[cfg(target_os = "macos")]
    let builder = builder
        .title_bar_style(tauri::TitleBarStyle::Overlay)
        .hidden_title(true)
        .traffic_light_position(tauri::LogicalPosition::new(15.0, 18.0));

    builder.build().context("Failed to build startup window")
}
