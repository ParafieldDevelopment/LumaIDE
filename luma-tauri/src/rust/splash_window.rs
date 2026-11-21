use anyhow::Context;
use std::sync::OnceLock;
use std::time::{Duration, Instant};
use tauri::webview::Color;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

pub const SPLASH_LABEL: &str = "splash";

/// Minimum time the splash stays on screen once it is visible.
const MIN_VISIBLE: Duration = Duration::from_millis(2500);
/// How long other windows will wait for the splash to appear before giving up.
const MAX_WAIT_FOR_SPLASH: Duration = Duration::from_secs(3);
/// Splash is force-closed this long after "splash-finish" if it didn't close itself.
const FORCE_CLOSE_AFTER: Duration = Duration::from_millis(1500);

static SHOWN_AT: OnceLock<Instant> = OnceLock::new();

pub fn splash_window(app: AppHandle) -> anyhow::Result<WebviewWindow> {
    // 850x550 matches the 2550x1650 splash art at 3x.
    WebviewWindowBuilder::new(
        &app,
        SPLASH_LABEL,
        WebviewUrl::App("windows/splash.html".into()),
    )
    .visible(false)
    .decorations(false)
    .resizable(false)
    .maximizable(false)
    .minimizable(false)
    .shadow(true)
    .always_on_top(true)
    .skip_taskbar(true)
    .center()
    .inner_size(850.0, 550.0)
    .title("Luma")
    .background_color(Color(0, 0, 0, 255))
    .build()
    .context("Failed to build splash window")
}

/// Called when the splash window has been shown.
pub fn mark_shown() {
    let _ = SHOWN_AT.set(Instant::now());
}

/// Blocks (call from a blocking thread) until the splash has been visible for
/// at least `MIN_VISIBLE`. Returns immediately if there is no splash.
pub fn hold_until_splash_done(app: &AppHandle) {
    let started = Instant::now();

    // The splash page may still be loading; wait for it to show up.
    while SHOWN_AT.get().is_none() {
        if app.get_webview_window(SPLASH_LABEL).is_none()
            || started.elapsed() > MAX_WAIT_FOR_SPLASH
        {
            return;
        }
        std::thread::sleep(Duration::from_millis(25));
    }

    if let Some(shown) = SHOWN_AT.get() {
        let elapsed = shown.elapsed();
        if elapsed < MIN_VISIBLE {
            std::thread::sleep(MIN_VISIBLE - elapsed);
        }
    }
}

/// Tells the splash to fade out and close, with a hard fallback.
pub fn finish(app: &AppHandle) {
    let Some(splash) = app.get_webview_window(SPLASH_LABEL) else {
        return;
    };

    if let Err(error) = app.emit_to(SPLASH_LABEL, "splash-finish", ()) {
        log::warn!("Failed to emit splash-finish: {error}");
    }

    std::thread::spawn(move || {
        std::thread::sleep(FORCE_CLOSE_AFTER);
        let _ = splash.close();
    });
}
