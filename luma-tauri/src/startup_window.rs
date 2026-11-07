use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};
use anyhow::{Context};
use crate::STARTUP_COLOR;

pub fn startup_window(app: tauri::AppHandle) -> anyhow::Result<()> {
    if let Some(window) = app.get_webview_window("startup") {
        window
            .show()
            .context("Failed to show the startup window")?;

        window
            .set_focus()
            .context("Failed to focus the startup window")?;
    } else {
        let mut builder = WebviewWindowBuilder::new(
            &app,
            "startup",
            WebviewUrl::App("windows/startup.html".into()),
        )

            .visible(false)
            .decorations(!cfg!(target_os = "windows"))
            .min_inner_size(650.0, 500.0)
            .title("Luma - Project Selection")
            .on_page_load(|window, _| {
                if let Err(err) = window.show() {
                    log::error!("Failed to show the startup window: {err:#}");
                }
            })
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

        let window = builder
            .build()
            .context("Failed to build window")?;
    }

    Ok(())
}