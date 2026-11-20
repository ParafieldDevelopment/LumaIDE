//! Process helpers for the IDE tool windows (Terminal and Git panels).
//!
//! This is NOT a PTY: commands run to completion (or time out) and their
//! output is returned in one piece. Interactive programs are not supported.

use serde::Serialize;
use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tauri::ipc::InvokeError;

const COMMAND_TIMEOUT: Duration = Duration::from_secs(60);
/// Output past this size is discarded (the pipe is still drained).
const MAX_OUTPUT_BYTES: usize = 2 * 1024 * 1024;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandOutput {
    pub code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub timed_out: bool,
}

fn invoke_error(message: impl std::fmt::Display) -> InvokeError {
    InvokeError::from_anyhow(anyhow::anyhow!("{}", message))
}

fn checked_dir(cwd: &str) -> Result<PathBuf, InvokeError> {
    let dir = PathBuf::from(cwd);
    if !dir.is_dir() {
        return Err(invoke_error(format!("Directory does not exist: {cwd}")));
    }
    Ok(dir)
}

/// Reads a pipe on a background thread into a shared buffer.
fn drain<R: Read + Send + 'static>(
    mut reader: R,
) -> (Arc<Mutex<Vec<u8>>>, thread::JoinHandle<()>) {
    let buffer = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&buffer);

    let handle = thread::spawn(move || {
        let mut chunk = [0u8; 4096];
        loop {
            match reader.read(&mut chunk) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    if let Ok(mut bytes) = sink.lock() {
                        if bytes.len() < MAX_OUTPUT_BYTES {
                            bytes.extend_from_slice(&chunk[..n]);
                        }
                    }
                }
            }
        }
    });

    (buffer, handle)
}

fn snapshot(buffer: &Arc<Mutex<Vec<u8>>>) -> String {
    let bytes = buffer.lock().map(|b| b.clone()).unwrap_or_default();
    String::from_utf8_lossy(&bytes).into_owned()
}

fn run_with_timeout(mut command: Command, timeout: Duration) -> std::io::Result<CommandOutput> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // CREATE_NO_WINDOW: don't flash a console window.
        command.creation_flags(0x0800_0000);
    }

    let mut child = command.spawn()?;
    let (out_buffer, out_thread) = drain(child.stdout.take().expect("stdout is piped"));
    let (err_buffer, err_thread) = drain(child.stderr.take().expect("stderr is piped"));

    let started = Instant::now();
    let mut timed_out = false;
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break Some(status);
        }
        if started.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            timed_out = true;
            break None;
        }
        thread::sleep(Duration::from_millis(30));
    };

    // After a timeout, grandchildren may still hold the pipes open, so only
    // wait for the readers when the process ended normally.
    if !timed_out {
        let _ = out_thread.join();
        let _ = err_thread.join();
    }

    Ok(CommandOutput {
        code: status.and_then(|s| s.code()),
        stdout: snapshot(&out_buffer),
        stderr: snapshot(&err_buffer),
        timed_out,
    })
}

async fn run_blocking(command: Command) -> Result<CommandOutput, InvokeError> {
    tauri::async_runtime::spawn_blocking(move || run_with_timeout(command, COMMAND_TIMEOUT))
        .await
        .map_err(|error| invoke_error(format!("Command task failed: {error}")))?
        .map_err(|error| invoke_error(format!("Failed to start process: {error}")))
}

/// Runs `git <args>` in `cwd`. Used by the Git Commit panel.
#[tauri::command]
pub async fn git_command(cwd: String, args: Vec<String>) -> Result<CommandOutput, InvokeError> {
    let dir = checked_dir(&cwd)?;

    // The first argument must be a subcommand. Global options such as
    // `-c core.sshCommand=...` can make git run arbitrary programs.
    match args.first() {
        Some(first) if !first.starts_with('-') => {}
        _ => return Err(invoke_error("Invalid git arguments")),
    }

    let mut command = Command::new("git");
    command
        .args(&args)
        .current_dir(dir)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_OPTIONAL_LOCKS", "0");

    run_blocking(command).await
}

/// Runs a shell command line in `cwd`. Used by the Terminal panel.
#[tauri::command]
pub async fn run_terminal_command(
    cwd: String,
    command: String,
) -> Result<CommandOutput, InvokeError> {
    let dir = checked_dir(&cwd)?;
    if command.trim().is_empty() {
        return Err(invoke_error("Empty command"));
    }

    #[cfg(windows)]
    let mut shell = {
        let mut shell = Command::new("powershell.exe");
        shell
            .args(["-NoLogo", "-NoProfile", "-NonInteractive", "-Command"])
            .arg(format!(
                "[Console]::OutputEncoding=[System.Text.Encoding]::UTF8; {command}"
            ));
        shell
    };

    #[cfg(not(windows))]
    let mut shell = {
        let mut shell = Command::new("sh");
        shell.arg("-c").arg(&command);
        shell
    };

    shell.current_dir(dir);
    run_blocking(shell).await
}

/// Resolves `target` relative to `cwd` and returns the new directory.
/// Used by the Terminal panel to implement `cd`.
#[tauri::command]
pub async fn terminal_change_dir(cwd: String, target: String) -> Result<String, InvokeError> {
    let base = checked_dir(&cwd)?;
    let target = target.trim();

    let candidate = if target == "~" || target.starts_with("~/") || target.starts_with("~\\") {
        let home = std::env::var_os("USERPROFILE")
            .or_else(|| std::env::var_os("HOME"))
            .ok_or_else(|| invoke_error("Home directory not found"))?;
        PathBuf::from(home).join(target[1..].trim_start_matches(|c| c == '/' || c == '\\'))
    } else {
        base.join(target)
    };

    let resolved = candidate
        .canonicalize()
        .map_err(|_| invoke_error(format!("Cannot find path '{target}'")))?;
    if !resolved.is_dir() {
        return Err(invoke_error(format!("'{target}' is not a directory")));
    }

    Ok(crate::base_invokes::clean_path(resolved)
        .to_string_lossy()
        .into_owned())
}
