use std::sync::{
    Arc,
    OnceLock,
    atomic::{AtomicBool, Ordering},
};
use std::thread::{sleep, sleep_ms};
use std::time::Duration;
use log::info;
use tauri::{AppHandle, Listener, Manager, RunEvent, WebviewWindow};
use tauri_plugin_log::{Target, TargetKind};
use tauri::Emitter;

mod backend;
pub static APPLICATION: OnceLock<AppHandle> = OnceLock::new();

const SPLASH_TIMEOUT_SECS: u64 = 30;

/// If the main window ever fails to load (mistyped URL, dev server down),
/// the splash would sit there forever. Escape hatch after SPLASH_TIMEOUT_SECS.
fn finish_splash(app: &AppHandle) {
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.show();
        let _ = main.set_focus();
    }

    if let Some(splash) = app.get_webview_window("splashscreen") {
        let _ = splash.close();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub async fn start_tauri() {
    let splash_shown = Arc::new(AtomicBool::new(false));

    let _ = tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::new()
                .targets([
                    Target::new(TargetKind::Stdout),
                    Target::new(TargetKind::LogDir { file_name: None }),
                ])
                .level(log::LevelFilter::Info)
                .build(),
        )
        .plugin(tauri_plugin_opener::init())
        .setup(move |app| {
            APPLICATION
                .set(app.handle().clone())
                .expect("Variable APPLICATION already set");

            let app_handle = app.handle().clone();

            tauri::async_runtime::spawn(async move {
                match backend::prepare().await {
                    Ok(()) => {
                        info!("Backend initialization completed!");

                        if let Err(error) =
                            app_handle.emit("luma:backend-ready", ())
                        {
                            log::error!("Failed to emit backend-ready event: {error}");
                        }
                    }
                    Err(error) => {
                        log::error!("Backend initialization failed: {error}");
                    }
                }
            });

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("failed to build app")
        .run(|_app_handle, event| {
            match event {
                RunEvent::Ready => {
                    info!("Tauri startup completed!");
                }

                RunEvent::Exit => {
                    info!("Application shutting down!");

                    // Cleanup code
                }

                RunEvent::ExitRequested { api, .. } => {
                    info!("Application exit requested!");
                }

                _ => {}
            }
        });
}
