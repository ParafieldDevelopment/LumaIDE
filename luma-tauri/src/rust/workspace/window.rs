use anyhow::Context;
use luma_api::workspaces::Workspace;
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

use crate::constants::STARTUP_COLOR;
use crate::windows::startup::STARTUP_LABEL;
use crate::workspace::store;

pub fn is_ide_label(label: &str) -> bool {
    label == "main" || label.starts_with("main-")
}

pub fn free_main_label(app: &tauri::AppHandle) -> String {
    if app.get_webview_window("main").is_none() {
        return "main".to_string();
    }

    let mut index = 2;
    loop {
        let label = format!("main-{index}");
        if app.get_webview_window(&label).is_none() {
            return label;
        }
        index += 1;
    }
}

/// Reuse this IDE window unless a new one was requested or the caller is the
/// startup window.
pub fn target_window_label(
    app: &tauri::AppHandle,
    caller: &tauri::WebviewWindow,
    new_window: bool,
) -> String {
    if !new_window && is_ide_label(caller.label()) {
        caller.label().to_string()
    } else {
        free_main_label(app)
    }
}

pub fn open_main_window(
    app: &tauri::AppHandle,
    workspace: &Workspace,
    label: &str,
) -> anyhow::Result<()> {
    let title = format!("Luma — {}", workspace.face.title);

    let window = if let Some(window) = app.get_webview_window(label) {
        window.set_title(&title)?;
        window
    } else {
        let window =
            WebviewWindowBuilder::new(app, label, WebviewUrl::App("windows/main.html".into()))
                .visible(false)
                .decorations(!cfg!(target_os = "windows"))
                .min_inner_size(1050.0, 600.0)
                .title(&title)
                .background_color(STARTUP_COLOR)
                .build()
                .context("Failed to create the IDE window")?;

        let owned_label = label.to_string();
        window.on_window_event(move |event| {
            if matches!(event, tauri::WindowEvent::Destroyed) {
                crate::filesystem::watch::stop(&owned_label);
                store::store().remove(&owned_label);
            }
        });
        window
    };

    window.show().context("Failed to show the IDE window")?;
    window
        .set_focus()
        .context("Failed to focus the IDE window")?;

    if let Some(startup) = app.get_webview_window(STARTUP_LABEL)
        && startup.label() != window.label()
    {
        startup
            .close()
            .context("Failed to close workspace selection window")?;
    }

    Ok(())
}
