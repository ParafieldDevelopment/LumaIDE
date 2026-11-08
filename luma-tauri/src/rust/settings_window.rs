use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};
use anyhow::{Context};
use crate::STARTUP_COLOR;

pub fn settings_window(app: tauri::AppHandle, parent: Option<&tauri::WebviewWindow>) -> anyhow::Result<()> {
    log::info!("settings_window called (parent={:?})", parent.map(|p| p.label()));
    if let Some(window) = app.get_webview_window("settings") {
        window
            .set_focus()
            .context("Failed to focus the settings window")?;
    } else {
        let mut builder = WebviewWindowBuilder::new(
            &app,
            "settings",
            WebviewUrl::App("windows/settings.html".into()),
        )

            .visible(false)
            .focused(false)
            .decorations(!cfg!(target_os = "windows"))
            .min_inner_size(750.0, 500.0)
            .title("Luma - Settings")
            .background_color(STARTUP_COLOR);

        #[cfg(target_os = "macos")]
        {
            builder = builder
                .title_bar_style(tauri::TitleBarStyle::Overlay)
                .hidden_title(true)
                .traffic_light_position(
                    tauri::LogicalPosition::new(15.0, 18.0)
                );
        }

        if let Some(parent) = parent {
            builder = builder
                .parent(parent)
                .context("Failed to set parent window")?;

            parent
                .set_enabled(false)
                .context("Failed to disable parent window")?;
        }
        
        let window = builder
            .build()
            .context("Failed to build window")?;

        if let Some(parent) = parent.cloned() {
            window.on_window_event(move |event| {
                if let tauri::WindowEvent::Destroyed = event {
                    if let Err(err) = parent.set_enabled(true) {
                        log::error!("Failed to re-enable parent window: {err}");
                    }
                }
            });
        }
    }

    Ok(())
}
