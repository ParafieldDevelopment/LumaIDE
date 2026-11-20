//! Process helpers for IDE tool windows and interactive terminal sessions.

use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use serde::Serialize;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use tauri::ipc::InvokeError;
use uuid::Uuid;

const COMMAND_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(60);
const MAX_OUTPUT_BYTES: usize = 2 * 1024 * 1024;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandOutput { pub code: Option<i32>, pub stdout: String, pub stderr: String, pub timed_out: bool }

fn invoke_error(message: impl std::fmt::Display) -> InvokeError {
    InvokeError::from_anyhow(anyhow::anyhow!("{}", message))
}
fn checked_dir(cwd: &str) -> Result<PathBuf, InvokeError> {
    let dir = PathBuf::from(cwd);
    if !dir.is_dir() { return Err(invoke_error(format!("Directory does not exist: {cwd}"))); }
    Ok(dir)
}

fn drain<R: Read + Send + 'static>(mut reader: R) -> (Arc<Mutex<Vec<u8>>>, thread::JoinHandle<()>) {
    let buffer = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&buffer);
    let handle = thread::spawn(move || {
        let mut chunk = [0u8; 4096];
        loop {
            match reader.read(&mut chunk) {
                Ok(0) | Err(_) => break,
                Ok(n) => if let Ok(mut bytes) = sink.lock() {
                    if bytes.len() < MAX_OUTPUT_BYTES { bytes.extend_from_slice(&chunk[..n]); }
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
fn run_with_timeout(mut command: Command, timeout: std::time::Duration) -> std::io::Result<CommandOutput> {
    command.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    #[cfg(windows)] {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    let mut child = command.spawn()?;
    let (out_buffer, out_thread) = drain(child.stdout.take().expect("stdout is piped"));
    let (err_buffer, err_thread) = drain(child.stderr.take().expect("stderr is piped"));
    let started = std::time::Instant::now();
    let mut timed_out = false;
    let status = loop {
        if let Some(status) = child.try_wait()? { break Some(status); }
        if started.elapsed() >= timeout { let _ = child.kill(); let _ = child.wait(); timed_out = true; break None; }
        thread::sleep(std::time::Duration::from_millis(30));
    };
    if !timed_out { let _ = out_thread.join(); let _ = err_thread.join(); }
    Ok(CommandOutput { code: status.and_then(|s| s.code()), stdout: snapshot(&out_buffer), stderr: snapshot(&err_buffer), timed_out })
}
async fn run_blocking(command: Command) -> Result<CommandOutput, InvokeError> {
    tauri::async_runtime::spawn_blocking(move || run_with_timeout(command, COMMAND_TIMEOUT)).await
        .map_err(|error| invoke_error(format!("Command task failed: {error}")))?
        .map_err(|error| invoke_error(format!("Failed to start process: {error}")))
}

#[tauri::command]
pub async fn git_command(cwd: String, args: Vec<String>) -> Result<CommandOutput, InvokeError> {
    let dir = checked_dir(&cwd)?;
    match args.first() { Some(first) if !first.starts_with('-') => {}, _ => return Err(invoke_error("Invalid git arguments")) }
    let mut command = Command::new("git");
    command.args(&args).current_dir(dir).env("GIT_TERMINAL_PROMPT", "0").env("GIT_OPTIONAL_LOCKS", "0");
    run_blocking(command).await
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalSessionInfo { pub id: String, pub shell: String }
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalReadResult { pub output: String, pub next_offset: usize, pub exited: bool }

struct TerminalSession {
    writer: Box<dyn Write + Send>,
    output: Arc<Mutex<Vec<u8>>>,
    child: Mutex<Box<dyn Child + Send>>,
    master: Box<dyn MasterPty + Send>,
}
static TERMINAL_SESSIONS: OnceLock<Mutex<std::collections::HashMap<String, TerminalSession>>> = OnceLock::new();
fn sessions() -> &'static Mutex<std::collections::HashMap<String, TerminalSession>> {
    TERMINAL_SESSIONS.get_or_init(|| Mutex::new(std::collections::HashMap::new()))
}

#[tauri::command]
pub async fn terminal_open_session(cwd: String) -> Result<TerminalSessionInfo, InvokeError> {
    let dir = checked_dir(&cwd)?;
    let pty_system = native_pty_system();
    let pair = pty_system.openpty(PtySize { rows: 30, cols: 120, pixel_width: 0, pixel_height: 0 })
        .map_err(|e| invoke_error(format!("Failed to create terminal: {e}")))?;
    #[cfg(windows)]
    let (shell, args) = ("powershell.exe", vec!["-NoLogo".to_string()]);
    #[cfg(not(windows))]
    let (shell, args) = (std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string()), vec!["-i".to_string()]);
    let mut cmd = CommandBuilder::new(shell);
    cmd.args(args);
    cmd.cwd(dir);
    cmd.env("TERM", "xterm-256color");
    let child = pair.slave.spawn_command(cmd).map_err(|e| invoke_error(format!("Could not start shell: {e}")))?;
    drop(pair.slave);
    let reader = pair.master.try_clone_reader().map_err(|e| invoke_error(format!("Could not read terminal: {e}")))?;
    let (output, _reader_thread) = drain(reader);
    let writer = pair.master.take_writer().map_err(|e| invoke_error(format!("Could not write to terminal: {e}")))?;
    let id = Uuid::new_v4().to_string();
    let shell_name = Path::new(shell).file_name().and_then(|n| n.to_str()).unwrap_or(shell).to_string();
    sessions().lock().map_err(|e| invoke_error(e))?.insert(id.clone(), TerminalSession {
        writer, output, child: Mutex::new(child), master: pair.master,
    });
    Ok(TerminalSessionInfo { id, shell: shell_name })
}

#[tauri::command]
pub async fn terminal_write(session_id: String, input: String) -> Result<(), InvokeError> {
    let mut map = sessions().lock().map_err(|e| invoke_error(e))?;
    let session = map.get_mut(&session_id).ok_or_else(|| invoke_error("Terminal session not found"))?;
    session.writer.write_all(input.as_bytes()).map_err(|e| invoke_error(format!("Terminal input failed: {e}")))?;
    session.writer.flush().map_err(|e| invoke_error(format!("Terminal flush failed: {e}")))
}

#[tauri::command]
pub async fn terminal_resize(session_id: String, cols: u16, rows: u16) -> Result<(), InvokeError> {
    let map = sessions().lock().map_err(|e| invoke_error(e))?;
    let session = map.get(&session_id).ok_or_else(|| invoke_error("Terminal session not found"))?;
    session.master.resize(PtySize { rows: rows.max(2), cols: cols.max(2), pixel_width: 0, pixel_height: 0 })
        .map_err(|e| invoke_error(format!("Terminal resize failed: {e}")))
}

#[tauri::command]
pub async fn terminal_read(session_id: String, offset: usize) -> Result<TerminalReadResult, InvokeError> {
    let map = sessions().lock().map_err(|e| invoke_error(e))?;
    let session = map.get(&session_id).ok_or_else(|| invoke_error("Terminal session not found"))?;
    let bytes = session.output.lock().map_err(|e| invoke_error(e))?;
    let start = offset.min(bytes.len());
    let output = String::from_utf8_lossy(&bytes[start..]).into_owned();
    let exited = session.child.lock().map_err(|e| invoke_error(e))?.try_wait().map_err(|e| invoke_error(format!("Failed checking shell state: {e}")))?.is_some();
    Ok(TerminalReadResult { output, next_offset: bytes.len(), exited })
}

#[tauri::command]
pub async fn terminal_close_session(session_id: String) -> Result<(), InvokeError> {
    let mut map = sessions().lock().map_err(|e| invoke_error(e))?;
    if let Some(session) = map.remove(&session_id) {
        if let Ok(mut child) = session.child.lock() { let _ = child.kill(); let _ = child.wait(); }
    }
    Ok(())
}

/// Resolve a cd target relative to cwd. Retained for the older terminal UI helper.
#[tauri::command]
pub async fn terminal_change_dir(cwd: String, target: String) -> Result<String, InvokeError> {
    let base = checked_dir(&cwd)?;
    let target = target.trim();
    let candidate = if target == "~" || target.starts_with("~/") || target.starts_with("~\\") {
        let home = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")).ok_or_else(|| invoke_error("Home directory not found"))?;
        PathBuf::from(home).join(target[1..].trim_start_matches(|c| c == '/' || c == '\\'))
    } else { base.join(target) };
    let resolved = candidate.canonicalize().map_err(|_| invoke_error(format!("Cannot find path '{target}'")))?;
    if !resolved.is_dir() { return Err(invoke_error(format!("'{target}' is not a directory"))); }
    Ok(crate::base_invokes::clean_path(resolved).to_string_lossy().into_owned())
}
