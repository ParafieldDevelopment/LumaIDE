#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod constants;
mod error_report;
mod events;
mod filesystem;
mod json;
mod paths;
mod terminal;
mod windows;
mod workspace;

use anyhow::{Context, Result};
use luma_diagnostics::ErrorReport;
use std::panic::{self, AssertUnwindSafe};
use std::process::ExitCode;

fn run_setup() -> Result<()> {
    let coreapi = luma_core::init().context("Failed to initialize Luma core")?;

    // Boot extensions before configuration is frozen, but do not hold the
    // write lock while booting: extensions can call back into the
    // configuration manager through the host API.
    luma_extensions::init(&coreapi).context("Failed to initialize Luma extensions")?;

    {
        let mut manager = coreapi.get_ide_configuration().write().unwrap();
        manager.freeze();
    }
    if let Ok(mut ws) = coreapi.get_workspace_configuration().write() {
        ws.freeze();
    }

    coreapi
        .get_ide_configuration()
        .write()
        .unwrap()
        .load_global(coreapi.get_configuration_dir())
        .context("Failed to load global configuration")?;

    Ok(())
}

fn main() -> ExitCode {
    luma_diagnostics::install_panic_logger(error_report::report_meta());

    match panic::catch_unwind(AssertUnwindSafe(run_setup)) {
        Ok(Ok(())) => log::info!("Luma initialization completed"),
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

    let app = match app::build_tauri_app() {
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
        app.run_return(move |_, event| {
            if matches!(event, tauri::RunEvent::Exit) {
                terminal::sessions::terminal_close_all_sessions();
            }
        })
    }));

    match run_outcome {
        Ok(code) => ExitCode::from(code.min(u8::MAX as i32) as u8),
        Err(payload) => {
            error_report::raise(&ErrorReport::from_panic_now(&payload));
            ExitCode::FAILURE
        }
    }
}
