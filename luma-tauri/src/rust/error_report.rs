use luma_diagnostics::{ErrorReport, ReportMeta};

pub fn report_meta() -> ReportMeta {
    ReportMeta {
        app_name: env!("CARGO_PKG_NAME").to_string(),
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        report_file: None,
    }
}

pub fn build_error_report(kind: &str, summary: String, error_text: String) -> ErrorReport {
    ErrorReport::from_error(kind, summary, error_text)
        .with_traceback(std::backtrace::Backtrace::force_capture().to_string())
}

/// Handle a fatal error without any Tauri runtime involvement: the report is
/// persisted to disk and the detached `luma-crash-handler` subprocess shows a
/// plain native dialog, offering to open the saved report.
pub fn raise(report: &ErrorReport) {
    let mut meta = report_meta();
    let saved = luma_diagnostics::persist(report, &mut meta);
    if let Some(path) = &saved {
        log::error!("Luma error report saved to {}", path.display());
    } else {
        log::error!("Failed to write the Luma error report file");
    }

    if let Err(err) = luma_diagnostics::spawn_crash_dialog(saved.as_deref()) {
        log::error!("Failed to launch the crash handler: {err:#}");
        eprintln!("Luma error:\n{}", report.to_plain_text(&meta));
    }
}
