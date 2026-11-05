use std::backtrace::Backtrace;
use std::fmt::Write as _;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct ReportMeta {
    pub app_name: String,
    pub app_version: String,
    pub report_file: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct ErrorReport {
    pub kind: String,
    pub summary: String,
    pub error_text: String,
    pub traceback: String,
}

impl ErrorReport {
    pub fn from_error(
        kind: impl Into<String>,
        summary: impl Into<String>,
        error_text: impl Into<String>,
    ) -> Self {
        Self {
            kind: kind.into(),
            summary: summary.into(),
            error_text: error_text.into(),
            traceback: String::new(),
        }
    }
    pub fn with_traceback(mut self, traceback: impl Into<String>) -> Self {
        self.traceback = traceback.into();
        self
    }

    pub fn from_panic_message(message: impl Into<String>) -> Self {
        let message = message.into();
        Self {
            kind: "Unexpected Panic".into(),
            summary: message.clone(),
            error_text: message,
            traceback: String::new(),
        }
    }

    pub fn from_panic(payload: &(dyn std::any::Any + Send), traceback: impl Into<String>) -> Self {
        Self::from_panic_message(panic_payload_message(payload)).with_traceback(traceback)
    }

    pub fn from_panic_now(payload: &(dyn std::any::Any + Send)) -> Self {
        Self::from_panic(
            payload,
            Backtrace::force_capture().to_string(),
        )
    }

    pub fn to_plain_text(&self, meta: &ReportMeta) -> String {
        let mut out = String::new();

        let _ = writeln!(out, "============== Luma error report ==============");
        let _ = writeln!(out, "Kind:       {}", self.kind);
        let _ = writeln!(out, "App:        {} v{}", meta.app_name, meta.app_version);
        let _ = writeln!(out, "Platform:   {}", plain_platform());
        let _ = writeln!(out, "Timestamp:  {}", utc_timestamp(SystemTime::now()));
        if let Some(file) = &meta.report_file {
            let _ = writeln!(out, "Report log: {}", file.display());
        }
        let _ = writeln!(out, "=====================================================");

        let _ = writeln!(out, "\nError:\n{}", self.error_text);
        if !self.traceback.is_empty() {
            let _ = writeln!(out, "\nTraceback:\n{}", self.traceback);
        }

        out
    }
}
pub fn panic_payload_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else if let Some(s) = payload.downcast_ref::<&str>() {
        (*s).to_string()
    } else {
        "Fatal Exception Occurred".to_string()
    }
}
pub type PanicUiHandler = fn(ErrorReport);

static PANIC_UI_HANDLER: Mutex<Option<PanicUiHandler>> = Mutex::new(None);

pub fn set_panic_ui_handler(handler: PanicUiHandler) {
    if let Ok(mut slot) = PANIC_UI_HANDLER.lock() {
        *slot = Some(handler);
    }
}

fn panic_ui_handler() -> Option<PanicUiHandler> {
    PANIC_UI_HANDLER.lock().ok().and_then(|slot| *slot)
}
pub fn install_panic_logger(meta: ReportMeta) {
    std::panic::set_hook(Box::new(move |info| {
        let message = match info.payload().downcast_ref::<&str>() {
            Some(s) => (*s).to_string(),
            None => match info.payload().downcast_ref::<String>() {
                Some(s) => s.clone(),
                None => "Unknown panic payload".to_string(),
            },
        };

        let mut error_text = message.clone();
        if let Some(loc) = info.location() {
            let _ = write!(
                error_text,
                "\nAt: {}:{}:{}",
                loc.file(),
                loc.line(),
                loc.column()
            );
        }

        let mut meta = meta.clone();
        let report =
            ErrorReport::from_panic_message(error_text.clone()).with_traceback(Backtrace::force_capture().to_string());

        if let Some(path) = persist(&report, &mut meta) {
            eprintln!("Luma panic reported to: {}", path.display());
        }
        eprintln!("Luma panic: {error_text}");

        if let Some(panic_ui) = panic_ui_handler() {
            panic_ui(report);
        }
    }));
}
pub fn crash_report_dir() -> PathBuf {
    let base = dirs::home_dir()
        .map(|home| home.join(".luma"))
        .unwrap_or_else(std::env::temp_dir);

    let dir = base.join("crash-reports");
    let _ = fs::create_dir_all(&dir);
    dir
}
pub fn persist(report: &ErrorReport, meta: &mut ReportMeta) -> Option<PathBuf> {
    let path = crash_report_dir().join(format!(
        "report-{}.txt",
        file_timestamp(SystemTime::now())
    ));

    fs::write(&path, report.to_plain_text(meta)).ok()?;
    meta.report_file = Some(path.clone());
    Some(path)
}
pub fn utc_timestamp(t: SystemTime) -> String {
    let (y, mo, d, h, mi, s) = civil_utc(t);
    format!("{y:04}-{mo:02}-{d:02} {h:02}:{mi:02}:{s:02} UTC")
}
pub fn file_timestamp(t: SystemTime) -> String {
    let (y, mo, d, h, mi, s) = civil_utc(t);
    format!("{y:04}{mo:02}{d:02}-{h:02}{mi:02}{s:02}")
}

fn civil_utc(t: SystemTime) -> (i64, u32, u32, u32, u32, u32) {
    let secs = t
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    let days = secs.div_euclid(86_400);
    let secs_of_day = secs.rem_euclid(86_400);

    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = mp + if mp < 10 { 3 } else { -9 };
    let y = y + i64::from(m <= 2);

    (
        y,
        m as u32,
        d as u32,
        (secs_of_day / 3600) as u32,
        ((secs_of_day % 3600) / 60) as u32,
        (secs_of_day % 60) as u32,
    )
}

fn plain_platform() -> String {
    format!(
        "{} / {} ({})",
        std::env::consts::OS,
        std::env::consts::ARCH,
        std::env::consts::FAMILY
    )
}
pub fn html_escape(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            other => out.push(other),
        }
    }
    out
}
pub fn render_error_page(report: &ErrorReport, meta: &ReportMeta) -> String {
    let details = html_escape(&report.to_plain_text(meta));
    let kind = html_escape(&report.kind);
    let summary = html_escape(&report.summary);
    let reported_line = match &meta.report_file {
        Some(file) => format!(
            "A copy of this report was also saved to <code>{}</code>.",
            html_escape(&file.display().to_string())
        ),
        None => "The report could not be saved to disk.".to_string(),
    };
    let footer_meta = html_escape(&format!(
        "{} v{} · {} · {}",
        meta.app_name,
        meta.app_version,
        plain_platform(),
        utc_timestamp(SystemTime::now())
    ));

    include_str!("error_page.html")
        .replace("@@TITLE@@", &html_escape(&format!(
            "{} — {}",
            meta.app_name, report.kind
        )))
        .replace("@@KIND@@", &kind)
        .replace("@@SUMMARY@@", &summary)
        .replace("@@DETAILS@@", &details)
        .replace("@@REPORTED_LINE@@", &reported_line)
        .replace("@@FOOTER_META@@", &footer_meta)
}
