#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
pub mod startup_window;
mod error_report;
pub mod base_invokes;
pub mod settings_window;
pub mod tauri_utils;

use std::panic::{self, AssertUnwindSafe};
use std::process::ExitCode;
use anyhow::{Context, Result};
use luma_diagnostics::ErrorReport;
use tauri::Manager;
use tauri::ipc::{CapabilityBuilder};
use tauri::webview::Color;
use tauri_plugin_prevent_default::Flags;

const STARTUP_COLOR: Color = Color {
    0: 6,
    1: 7,
    2: 10,
    3: 158,
};

fn run_setup() -> Result<()> {
    let coreapi = luma_core::init().context("Failed to initialize Luma core")?;
    let mut manager = coreapi.get_ide_configuration().write().unwrap();

    luma_extensions::init(&coreapi).context("Failed to initialize Luma extensions")?;

    manager.freeze();
    manager.load_global(&coreapi.get_configuration_file_name())
        .context("Failed to load global configuration")?;

    Ok(())
}

fn main() -> ExitCode {
    luma_diagnostics::install_panic_logger(error_report::report_meta());

    match panic::catch_unwind(AssertUnwindSafe(run_setup)) {
        Ok(Ok(())) => {
            log::info!("Luma initialization completed");
        }
        Ok(Err(error)) => {
            error_report::raise(&error_report::build_error_report(
                "Initialization Failure",
                "Luma IDE failed to initialize.".to_string(),
                format!("{error:#}"),
            ));
            return ExitCode::FAILURE;
        }
        Err(payload) => {
            error_report::raise(&ErrorReport::from_panic_now(&payload));
            return ExitCode::FAILURE;
        }
    }

    let app = match build_tauri_app() {
        Ok(app) => app,
        Err(error) => {
            error_report::raise(&error_report::build_error_report(
                "Startup Failure",
                "Luma could not start: the Tauri runtime failed to initialize.".to_string(),
                format!("{error:#}"),
            ));
            return ExitCode::FAILURE;
        }
    };

    let run_outcome = panic::catch_unwind(AssertUnwindSafe(move || {
        let app = app;
        app.run_return(move |_, _| {})
    }));

    match run_outcome {
        Ok(code) => ExitCode::from(code.min(u8::MAX as i32) as u8),

        Err(payload) => {
            error_report::raise(&ErrorReport::from_panic_now(&payload));
            ExitCode::FAILURE
        }
    }
}
fn build_tauri_app() -> Result<tauri::App> {
    use tauri_plugin_window_state::StateFlags;

    // Store window state in the shared Roaming/Luma directory instead of the
    // bundle-identifier directory Tauri would use by default.
    let window_state_path = luma_core::data_dir()
        .context("Failed to resolve the Luma data directory")?
        .join(".window-state.json");

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_decorum::init())
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_os::init())
        .plugin(
            tauri_plugin_prevent_default::Builder::new()
                .with_flags(
                    Flags::all() & !Flags::CONTEXT_MENU & !Flags::DEV_TOOLS
                )
                .build()
        )
        .plugin(tauri_plugin_single_instance::init(|_app, _args, _cwd| {}))
        .plugin(
            tauri_plugin_window_state::Builder::new()
                // DECORATIONS would re-apply a saved `decorations: true` on top of
                // windows built with `.decorations(false)` (e.g. the startup window).
                .with_state_flags(
                    StateFlags::all() & !StateFlags::DECORATIONS & !StateFlags::VISIBLE
                )
                // The plugin joins this onto `app_config_dir()`, but `Path::join`
                // replaces the base when given an absolute path, so passing the
                // full path stores window state in Roaming/Luma instead of the
                // bundle-identifier directory.
                .with_filename(window_state_path.to_string_lossy().into_owned())
                .build(),
        )
        .invoke_handler(
            tauri::generate_handler![

                // base_invokes

                base_invokes::webview_ready,
                base_invokes::open_settings_window,
                base_invokes::open_devtools,
                base_invokes::open_workspace,

            ]
        )
        .plugin(
            tauri_plugin_log::Builder::new()
                .targets([tauri_plugin_log::Target::new(
                    tauri_plugin_log::TargetKind::Stdout,
                )])
                .level(log::LevelFilter::Info)
                .build(),
        )
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            log::info!("Tauri ready");

            let capability = CapabilityBuilder::new("startup")
                .window("*")
                .permission("core:default")
                .permission("core:window:default")
                .permission("core:window:allow-close")
                .permission("core:window:allow-minimize")
                .permission("core:window:allow-toggle-maximize")
                .permission("core:window:allow-start-dragging")
                .permission("core:window:allow-set-focus")
                .permission("notification:allow-is-permission-granted")
                .permission("notification:allow-request-permission")
                .permission("notification:allow-notify")
                .permission("core:window:allow-set-focus");

            app.add_capability(capability)?;

            startup_window::startup_window(app.handle().clone()).context("Failed to initialize startup window")?;

            Ok(())
        })
        .build(tauri::generate_context!())
        .context("Failed to build the Tauri application")?;

    Ok(app)
}
