use std::sync::Mutex;

use anyhow::{Context, Result};
use luma_diagnostics::{ErrorReport, ReportMeta};
use tauri::{AppHandle, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};
pub const REPORT_SCHEME: &str = "lumaerror";
pub const REPORT_WINDOW_LABEL: &str = "luma-error";
const REPORT_URL: &str = "lumaerror://localhost/report.html";

pub fn report_meta() -> ReportMeta {
    ReportMeta {
        app_name: env!("CARGO_PKG_NAME").to_string(),
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        report_file: None,
    }
}

static REPORT_PAGE: Mutex<Option<String>> = Mutex::new(None);
static APP_HANDLE: Mutex<Option<AppHandle>> = Mutex::new(None);
pub fn set_app_handle(handle: AppHandle) {
    if let Ok(mut slot) = APP_HANDLE.lock() {
        *slot = Some(handle);
    }
}
pub fn app_handle() -> Option<AppHandle> {
    APP_HANDLE.lock().ok().and_then(|slot| slot.clone())
}
pub fn build_error_report(kind: &str, summary: String, error_text: String) -> ErrorReport {
    ErrorReport::from_error(kind, summary, error_text)
        .with_traceback(std::backtrace::Backtrace::force_capture().to_string())
}
pub fn protocol_handler_response(
    _request: &tauri::http::Request<Vec<u8>>,
) -> tauri::http::Response<Vec<u8>> {    let page = REPORT_PAGE.lock().ok().and_then(|slot| slot.clone());

    match page {
        Some(page) => tauri::http::Response::builder()
            .header(tauri::http::header::CONTENT_TYPE, "text/html; charset=utf-8")
            .header(tauri::http::header::CACHE_CONTROL, "no-store")
            .body(page.into_bytes())
            .expect("static error page response"),
        None => tauri::http::Response::builder()
            .status(tauri::http::StatusCode::NOT_FOUND)
            .header(
                tauri::http::header::CONTENT_TYPE,
                "text/plain; charset=utf-8",
            )
            .body(b"no error report is currently active".to_vec())
            .expect("static error page response"),
    }
}
pub fn persist_report(report: &ErrorReport, meta: &mut ReportMeta) {
    if let Some(path) = luma_diagnostics::persist(report, meta) {
        log::error!("Luma error report saved to {}", path.display());
    } else {
        log::error!("Failed to write the Luma error report file");
    }
}
fn publish_page(report: &ErrorReport, meta: &ReportMeta) {
    if let Ok(mut slot) = REPORT_PAGE.lock() {
        *slot = Some(luma_diagnostics::render_error_page(report, meta));
    }
}
pub fn show_report_window(app: &AppHandle, report: &ErrorReport, mut meta: ReportMeta) -> Result<()> {
    persist_report(report, &mut meta);
    publish_page(report, &meta);

    let url = tauri::Url::parse(REPORT_URL)
        .context("Failed to parse the error report URL")?;

    WebviewWindowBuilder::new(app, REPORT_WINDOW_LABEL, WebviewUrl::CustomProtocol(url))
        .title(format!("{} — {}", meta.app_name, report.kind))
        .inner_size(780.0, 620.0)
        .min_inner_size(460.0, 320.0)
        .center()
        .resizable(true)
        .visible(false)
        .on_page_load(|window, _| {
            if let Err(err) = window.show() {
                log::error!("Failed to show the error report window: {err:#}");
            }
            if let Err(err) = window.set_focus() {
                log::error!("Failed to focus the error report window: {err:#}");
            }
        })
        .build()
        .context("Failed to create the error report window")?;

    Ok(())
}

pub fn show_native_dialog(app: &AppHandle, report: &ErrorReport, mut meta: ReportMeta) -> Mechanism {
    persist_report(report, &mut meta);

    let shown = app
        .dialog()
        .message(report.to_plain_text(&meta))
        .title(format!("{} — {}", meta.app_name, report.kind))
        .kind(MessageDialogKind::Error)
        .blocking_show();

    if shown {
        Mechanism::Dialog
    } else {
        Mechanism::FileOnly
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mechanism {
    Window, 
    Dialog,
    FileOnly,
}

pub fn present(
    app: Option<&AppHandle>,
    report: &ErrorReport,
    meta: ReportMeta,
) -> Mechanism {
    match app {
        Some(app) => match show_report_window(app, report, meta.clone()) {
            Ok(()) => Mechanism::Window,
            Err(err) => {
                log::error!("Failed to open the error report window: {err:#}");
                show_native_dialog(app, report, meta)
            }
        },
        None => {
            let mut meta = meta;
            luma_diagnostics::persist(report, &mut meta);
            eprintln!("Luma error:\n{}", report.to_plain_text(&meta));
            Mechanism::FileOnly
        }
    }
}
