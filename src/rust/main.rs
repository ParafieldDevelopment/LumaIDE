// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use crate::splash::start_splash_screen;

mod splash;

#[tokio::main]
async fn main() {
    start_splash_screen().await;
    // luma_lib::start_tauri().await;
}
