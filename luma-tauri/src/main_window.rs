use anyhow::Context;
use tauri::{Manager, TitleBarStyle, WebviewUrl, WebviewWindowBuilder};
use tauri::webview::Color;
use crate::STARTUP_COLOR;

pub fn main_window(app: tauri::AppHandle) -> anyhow::Result<()> {
    if let Some(window) = app.get_webview_window("main") {
        window
            .show()
            .context("Failed to show the main window")?;

        window
            .set_focus()
            .context("Failed to focus the main window")?;
    } else {
        let mut builder =  WebviewWindowBuilder::new(
            &app,
            "main",
            WebviewUrl::App("windows/index.html".into()),
        )
            .visible(false)
            .decorations(false)
            .focused(false)
            .on_page_load(|window, _| {
                if let Err(err) = window.show() {
                    log::error!("Failed to show startup window: {err:#}");
                }
            })
            .background_color(
                STARTUP_COLOR
            );

        #[cfg(target_os = "macos")]
        {
            builder = builder
                .title_bar_style(tauri::TitleBarStyle::Overlay)
                .traffic_light_position(tauri::LogicalPosition::new(12.0, 16.0));
        }

        let _window = builder
            .build()
            .context("Failed to build window")?;
    }

    Ok(())
}