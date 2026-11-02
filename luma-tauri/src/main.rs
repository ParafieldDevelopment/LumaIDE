#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::Serialize;
use tokio::time::{sleep, Duration};
async fn run_setup(app: tauri::AppHandle) {
    log::info!("Tauri startup completed.");
    luma_core::init();
    sleep(Duration::from_millis(500)).await;
    log::info!("Luma startup completed");
}

#[tokio::main]
async fn main() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::new()
                .targets([tauri_plugin_log::Target::new(
                    tauri_plugin_log::TargetKind::Stdout,
                )])
                .level(log::LevelFilter::Info)
                .build(),
        )
        .plugin(tauri_plugin_opener::init())
        .register_uri_scheme_protocol("luma-plugin", |ctx, request| {
            tauri::http::Response::builder()
                .status(501)
                .body(Vec::new())
                .unwrap()
        })
        .setup(|app| {
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(run_setup(handle));
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}