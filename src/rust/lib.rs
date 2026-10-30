use std::sync::OnceLock;
use tauri::{AppHandle, RunEvent};

pub static APPLICATION: OnceLock<AppHandle> = OnceLock::new();
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn start_tauri() {
    tauri::Builder::default()
        .build(tauri::generate_context!())
        .expect("error while running tauri application")
        .run(|app, event| {
            match event {
                RunEvent::Ready => {
                    APPLICATION.set(app.clone()).expect("Variable APPLICATION already set");
                }

                _ => {}
            }
        });
}