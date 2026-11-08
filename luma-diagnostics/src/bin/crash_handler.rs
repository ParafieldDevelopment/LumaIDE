#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

//! Standalone Luma crash reporter (binary: `luma-crash-handler`).
//!
//! Runs as its own subprocess (spawned detached from the main app by
//! [`luma_diagnostics::spawn_crash_dialog`]) so it can outlive a crashed
//! parent process. It shows a plain native dialog on each platform
//! (Win32 `MessageBoxW`, macOS `NSAlert` via `rfd`, `zenity`/`kdialog` on
//! Linux); if the user asks to view the saved crash report, the file is
//! opened with the system's default handler.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

const DIALOG_TITLE: &str = "Luma has crashed!";
const DIALOG_MESSAGE: &str =
    "An unexpected error occurred and Luma has to close.\nA crash report has been saved. Would you like to view it?";

fn main() -> ExitCode {
    let report_file = std::env::args().nth(1).map(PathBuf::from);

    match show_crash_dialog(report_file.as_deref()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("luma-crash-handler: {error:#}");
            ExitCode::FAILURE
        }
    }
}

fn handle_report_request(report_file: Option<&Path>) -> Result<(), String> {
    if let Some(file) = report_file {
        open_report(file)?;
    }
    Ok(())
}

/// The report could not be interactively offered; dump everything to stderr.
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn stderr_fallback(report_file: Option<&Path>) {
    eprintln!("{DIALOG_TITLE}\n{DIALOG_MESSAGE}");
    if let Some(file) = report_file {
        eprintln!("Report file: {}", file.display());
    }
}

#[cfg(target_os = "windows")]
fn show_crash_dialog(report_file: Option<&Path>) -> Result<(), String> {
    use windows::core::PCWSTR;
    use windows::Win32::UI::WindowsAndMessaging::{
        MessageBoxW, IDYES, MB_ICONERROR, MB_SETFOREGROUND, MB_TOPMOST, MB_YESNO,
    };

    let message = wide(DIALOG_MESSAGE);
    let title = wide(DIALOG_TITLE);

    let ordering = unsafe {
        MessageBoxW(
            None,
            PCWSTR(message.as_ptr()),
            PCWSTR(title.as_ptr()),
            MB_YESNO | MB_ICONERROR | MB_SETFOREGROUND | MB_TOPMOST,
        )
    };

    if ordering == IDYES {
        handle_report_request(report_file)?;
    }

    Ok(())
}

#[cfg(target_os = "macos")]
fn show_crash_dialog(report_file: Option<&Path>) -> Result<(), String> {
    let answer = rfd::MessageDialog::new()
        .set_title(DIALOG_TITLE)
        .set_description(DIALOG_MESSAGE)
        .set_level(rfd::MessageLevel::Error)
        .set_buttons(rfd::MessageButtons::YesNo)
        .blocking_show();

    if answer == rfd::MessageDialogResult::Yes {
        handle_report_request(report_file)?;
    }

    Ok(())
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn show_crash_dialog(report_file: Option<&Path>) -> Result<(), String> {
    match linux_dialog_answer() {
        Ok(Some(answer)) => {
            if answer {
                handle_report_request(report_file)?;
            }
        }
        Ok(None) => stderr_fallback(report_file),
        Err(err) => return Err(err),
    }

    Ok(())
}

/// Ask `zenity` (GNOME and friends) or `kdialog` (KDE) to render the dialog.
/// Returns `Ok(Some(true))` when the user wants to view the report,
/// `Ok(Some(false))` when they declined, and `Ok(None)` when neither tool is
/// available (the caller should fall back to stderr).
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn linux_dialog_answer() -> Result<Option<bool>, String> {
    use std::process::Command;

    if let Some(success) = ask_tool(
        Command::new("zenity")
            .args(["--question", "--no-markup"])
            .arg("--title")
            .arg(DIALOG_TITLE)
            .arg("--text")
            .arg(DIALOG_MESSAGE),
    ) {
        return Ok(Some(success));
    }

    if let Some(success) = ask_tool(
        Command::new("kdialog")
            .arg("--question")
            .arg(DIALOG_MESSAGE)
            .arg("--title")
            .arg(DIALOG_TITLE),
    ) {
        return Ok(Some(success));
    }

    Ok(None)
}

/// Run a dialog tool that exits with 0 for "yes". Returns `None` when the
/// tool could not be launched at all.
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn ask_tool(command: &mut std::process::Command) -> Option<bool> {
    command.output().ok().map(|output| output.status.success())
}

/// Encode a string as a zero-terminated UTF-16 buffer for the Win32 API.
#[cfg(target_os = "windows")]
fn wide(text: &str) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;

    std::ffi::OsStr::new(text)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

#[cfg(target_os = "windows")]
fn open_report(path: &Path) -> Result<(), String> {
    use windows::core::PCWSTR;
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    let file = wide(&path.as_os_str().to_string_lossy());
    let operation = wide("open");

    // ShellExecuteW returns a pseudo-instance; values above 32 signal success.
    let result = unsafe {
        ShellExecuteW(
            None,
            PCWSTR(operation.as_ptr()),
            PCWSTR(file.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };

    if result.0 as usize > 32 {
        Ok(())
    } else {
        Err(format!("failed to open the crash report {}", path.display()))
    }
}

#[cfg(not(target_os = "windows"))]
fn open_report(path: &Path) -> Result<(), String> {
    use std::process::Command;

    // `open` is the macOS default-handler launcher, `xdg-open` covers the
    // remaining desktop Unixes.
    let opener = if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };

    let status = Command::new(opener)
        .arg(path)
        .status()
        .map_err(|err| format!("failed to launch {opener}: {err}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("failed to open the crash report {}", path.display()))
    }
}
