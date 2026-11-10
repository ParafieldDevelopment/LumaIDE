use tauri::{
    Manager,
    Runtime,
    WebviewUrl,
    WebviewWindow,
    WebviewWindowBuilder,
    WindowEvent,
};

use anyhow::Context;
use log::{error, info};

use crate::STARTUP_COLOR;
use crate::tauri_utils::with_parent;

pub fn settings_window(
    app: tauri::AppHandle,
    parent: Option<&tauri::WebviewWindow>,
) -> anyhow::Result<()> {
    info!(
        "settings_window called (parent={:?})",
        parent.map(|p| p.label())
    );

    // Reuse an existing settings window.
    if let Some(window) = app.get_webview_window("settings") {
        window
            .set_focus()
            .context("Failed to focus settings window")?;

        return Ok(());
    }

    // Create the settings window builder.
    let builder = WebviewWindowBuilder::new(
        &app,
        "settings",
        WebviewUrl::App("windows/settings.html".into()),
    )
        .visible(false)
        .focused(true)
        .decorations(!cfg!(target_os = "windows"))
        .min_inner_size(750.0, 500.0)
        .title("Luma - Settings")
        .background_color(STARTUP_COLOR);

    // macOS configuration.
    #[cfg(target_os = "macos")]
    let builder = builder
        .title_bar_style(tauri::TitleBarStyle::Overlay)
        .hidden_title(true)
        .traffic_light_position(
            tauri::LogicalPosition::new(15.0, 18.0),
        );

    // Attach the parent.
    let builder = with_parent(builder, parent)
        .context("Failed to attach parent window")?;

    // Windows requires manual modality.
    #[cfg(target_os = "windows")]
    if let Some(parent) = parent {
        parent
            .set_enabled(false)
            .context("Failed to disable parent window")?;
    }

    // Build the settings window.
    let window = match builder.build() {
        Ok(window) => window,

        Err(err) => {
            // Recover the parent if window creation fails.
            #[cfg(target_os = "windows")]
            if let Some(parent) = parent {
                if let Err(recovery_err) = parent.set_enabled(true) {
                    error!(
                        "Failed to restore parent window: {recovery_err}"
                    );
                }
            }

            return Err(err).context("Failed to build settings window");
        }
    };

    // Restore the parent when settings is destroyed.
    if let Some(parent) = parent.cloned() {
        window.on_window_event(move |event| {
            if !matches!(event, WindowEvent::Destroyed) {
                return;
            }

            #[cfg(target_os = "windows")]
            if let Err(err) = parent.set_enabled(true) {
                error!("Failed to re-enable parent window: {err}");
            }

            if let Err(err) = parent.set_focus() {
                error!("Failed to restore parent window focus: {err}");
            }
        });
    }

    Ok(())
}
