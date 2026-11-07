#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod startup_window;
mod error_report;

use std::panic::{self, AssertUnwindSafe};
use std::process::ExitCode;
use anyhow::{Context, Result};
use luma_diagnostics::ErrorReport;
use tauri::{Manager, RunEvent, WindowEvent};
use tauri::ipc::{CapabilityBuilder};
use tauri::webview::Color;
const STARTUP_COLOR: Color = Color {
    0: 6,
    1: 7,
    2: 10,
    3: 158,
};

fn run_setup() -> Result<()> {
    luma_core::init().context("Failed to initialize Luma core")
}

fn main() -> ExitCode {
    luma_diagnostics::install_panic_logger(error_report::report_meta());

    let mut pending_report: Option<ErrorReport> = match panic::catch_unwind(AssertUnwindSafe(run_setup)) {
        Ok(Ok(())) => {
            log::info!("Luma initialization completed");
            None
        }
        Ok(Err(error)) => Some(error_report::build_error_report(
            "Initialization Failure",
            "Luma IDE failed to initialize.".to_string(),
            format!("{error:#}"),
        )),
        Err(payload) => Some(ErrorReport::from_panic_now(&payload)),
    };

    let bootstrapped_with_failure = pending_report.is_some();

    let app = match build_tauri_app() {
        Ok(app) => app,
        Err(error) => {
            let report = error_report::build_error_report(
                "Startup Failure",
                "Luma could not start: the Tauri runtime failed to initialize.".to_string(),
                format!("{error:#}"),
            );
            error_report::present(None, &report, error_report::report_meta());
            return ExitCode::FAILURE;
        }
    };

    let run_outcome = panic::catch_unwind(AssertUnwindSafe(move || {
        let app = app;
        app.run_return(move |handle, event| match &event {
            RunEvent::Ready => {
                if let Some(report) = pending_report.take() {
                    let mechanism =
                        error_report::present(Some(handle), &report, error_report::report_meta());
                    if mechanism == error_report::Mechanism::FileOnly {
                        handle.exit(1);
                    }
                }
            }

            RunEvent::WindowEvent { label, event: WindowEvent::Destroyed, .. }
                if label == error_report::REPORT_WINDOW_LABEL =>
            {
                handle.exit(1);
            }

            _ => {}
        })
    }));

    match run_outcome {
        Ok(code) => ExitCode::from(if bootstrapped_with_failure {
            1
        } else {
            code.min(u8::MAX as i32) as u8
        }),

        Err(payload) => {
            let report = ErrorReport::from_panic_now(&payload);
            error_report::present(
                error_report::app_handle().as_ref(),
                &report,
                error_report::report_meta(),
            );
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
        .plugin(tauri_plugin_single_instance::init(|_app, _args, _cwd| {}))
        .plugin(
            tauri_plugin_window_state::Builder::new()
                // DECORATIONS would re-apply a saved `decorations: true` on top of
                // windows built with `.decorations(false)` (e.g. the startup window).
                .with_state_flags(StateFlags::all() & !StateFlags::DECORATIONS)
                // The plugin joins this onto `app_config_dir()`, but `Path::join`
                // replaces the base when given an absolute path, so passing the
                // full path stores window state in Roaming/Luma instead of the
                // bundle-identifier directory.
                .with_filename(window_state_path.to_string_lossy().into_owned())
                .build(),
        )
        .invoke_handler(
            tauri::generate_handler![]
        )
        .plugin(
            tauri_plugin_log::Builder::new()
                .targets([tauri_plugin_log::Target::new(
                    tauri_plugin_log::TargetKind::Stdout,
                )])
                .level(log::LevelFilter::Info)
                .build(),
        )
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .register_uri_scheme_protocol(error_report::REPORT_SCHEME, |_ctx, request| {
            error_report::protocol_handler_response(&request)
        })
        .setup(|app| {
            log::info!("Tauri ready");

            let capability = CapabilityBuilder::new("startup")
                .window("*")
                .permission("core:default")
                .permission("core:window:default")
                .permission("core:window:allow-close")
                .permission("core:window:allow-minimize")
                .permission("core:window:allow-toggle-maximize")
                .permission("core:window:allow-start-dragging");

            app.add_capability(capability)?;

            error_report::set_app_handle(app.handle().clone());

            startup_window::startup_window(app.handle().clone()).context("Failed to initialize startup window")?;

            Ok(())
        })
        .build(tauri::generate_context!())
        .context("Failed to build the Tauri application")?;

    Ok(app)
}
