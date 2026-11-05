use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};
use anyhow::{Context, Result};

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
            .title("Luma - Startup")
            .on_page_load(|window, _| {
                if let Err(err) = window.show() {
                    log::error!("Failed to show startup window: {err:#}");
                }
            })
            .build()
            .context("Failed to create the startup window")?;
    }

    Ok(())
}