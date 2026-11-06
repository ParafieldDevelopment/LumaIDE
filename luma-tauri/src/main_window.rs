use anyhow::Context;
use tauri::{Manager, TitleBarStyle, WebviewUrl, WebviewWindowBuilder};
use tauri::webview::Color;

pub fn main_window(app: tauri::AppHandle) -> anyhow::Result<()> {
    if let Some(window) = app.get_webview_window("main") {
        window
            .show()
            .context("Failed to show the main window")?;

        window
            .set_focus()
            .context("Failed to focus the main window")?;
    } else {
        WebviewWindowBuilder::new(
            &app,
            "main",
            WebviewUrl::App("windows/index.html".into()),
        )
            .visible(false)

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