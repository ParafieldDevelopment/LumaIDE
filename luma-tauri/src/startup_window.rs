use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};
use anyhow::{Context, Result};
use tauri::webview::Color;

pub fn startup_window(app: tauri::AppHandle) -> Result<()> {
    if let Some(window) = app.get_webview_window("startup") {
        window
            .show()
            .context("Failed to show the startup window")?;

        window
            .set_focus()
            .context("Failed to focus the startup window")?;
    } else {
        WebviewWindowBuilder::new(
            &app,
            "startup",
            WebviewUrl::App("windows/startup.html".into()),
        )
            .visible(false)
            .title("Luma - Project Selection")
            .on_page_load(|window, _| {
                if let Err(err) = window.show() {
                    log::error!("Failed to show startup window: {err:#}");
                }
            })
            .focused(true)
            .background_color(
                Color {
                    0: 6,
                    1: 7,
                    2: 10,
                    3: 158,
                }
            )
            .build()
            .context("Failed to create the startup window")?;
    }

    Ok(())
}