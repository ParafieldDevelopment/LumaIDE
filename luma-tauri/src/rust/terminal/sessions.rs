use portable_pty::{Child, CommandBuilder, MasterPty, PtySize, native_pty_system};
use serde::Serialize;
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Instant;
use tauri::ipc::InvokeError;
use uuid::Uuid;

use luma_api::event_bus;
use luma_api::events::{TerminalClosedEvent, TerminalOpenedEvent};

use super::{checked_dir, invoke_error, output};
use crate::paths;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalSessionInfo {
    pub id: String,
    pub shell: String,
    pub cwd: String,
    pub exited: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalReadResult {
    pub output: String,
    pub next_offset: usize,
    pub exited: bool,
}

struct TerminalSession {
    exited: bool,
    closed_notified: bool,
    shell: String,
    cwd: String,
    created_at: Instant,
    writer: Box<dyn Write + Send>,
    output: Arc<Mutex<output::TerminalOutputBuffer>>,
    child: Mutex<Box<dyn Child + Send>>,
    master: Box<dyn MasterPty + Send>,
}

static SESSIONS: OnceLock<Mutex<HashMap<String, TerminalSession>>> = OnceLock::new();

fn sessions() -> &'static Mutex<HashMap<String, TerminalSession>> {
    SESSIONS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn child_exited(session: &TerminalSession) -> Result<bool, InvokeError> {
    session
        .child
        .lock()
        .map_err(invoke_error)?
        .try_wait()
        .map(|status| status.is_some())
        .map_err(|error| invoke_error(format!("Failed checking shell state: {error}")))
}

fn notify_opened(id: &str, shell: &str, cwd: &str) {
    event_bus().fire(TerminalOpenedEvent {
        id: id.to_string(),
        shell: shell.to_string(),
        cwd: cwd.to_string(),
    });
}

/// Fire `TerminalClosedEvent` at most once per session.
fn notify_closed(session: &mut TerminalSession, id: &str) {
    if session.closed_notified {
        return;
    }
    session.closed_notified = true;
    event_bus().fire(TerminalClosedEvent {
        id: id.to_string(),
        shell: session.shell.clone(),
        cwd: session.cwd.clone(),
    });
}

fn mark_exited(session: &mut TerminalSession, id: &str) {
    session.exited = true;
    notify_closed(session, id);
}

#[tauri::command]
pub async fn terminal_open_session(cwd: String) -> Result<TerminalSessionInfo, InvokeError> {
    let dir = checked_dir(&cwd)?;
    let pair = native_pty_system()
        .openpty(PtySize {
            rows: 30,
            cols: 120,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|error| invoke_error(format!("Failed to create terminal: {error}")))?;

    #[cfg(windows)]
    let (shell, args) = ("powershell.exe", vec!["-NoLogo".to_string()]);
    #[cfg(not(windows))]
    let (shell, args) = (
        std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string()),
        vec!["-i".to_string()],
    );

    let cwd_string = paths::display_path(&dir);
    let mut command = CommandBuilder::new(shell);
    command.args(args);
    command.cwd(&dir);
    command.env("TERM", "xterm-256color");

    let child = pair
        .slave
        .spawn_command(command)
        .map_err(|error| invoke_error(format!("Could not start shell: {error}")))?;
    drop(pair.slave);

    let reader = pair
        .master
        .try_clone_reader()
        .map_err(|error| invoke_error(format!("Could not read terminal: {error}")))?;
    let output = Arc::new(Mutex::new(output::TerminalOutputBuffer::default()));
    let _reader_thread = output::drain_terminal(reader, Arc::clone(&output));
    let writer = pair
        .master
        .take_writer()
        .map_err(|error| invoke_error(format!("Could not write to terminal: {error}")))?;

    let id = Uuid::new_v4().to_string();
    let shell_name = Path::new(shell)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(shell)
        .to_string();
    let info = TerminalSessionInfo {
        id: id.clone(),
        shell: shell_name.clone(),
        cwd: cwd_string.clone(),
        exited: false,
    };

    sessions().lock().map_err(invoke_error)?.insert(
        id.clone(),
        TerminalSession {
            exited: false,
            closed_notified: false,
            shell: shell_name,
            cwd: cwd_string,
            created_at: Instant::now(),
            writer,
            output,
            child: Mutex::new(child),
            master: pair.master,
        },
    );
    notify_opened(&info.id, &info.shell, &info.cwd);

    Ok(info)
}

#[tauri::command]
pub async fn terminal_write(session_id: String, input: String) -> Result<(), InvokeError> {
    let mut map = sessions().lock().map_err(invoke_error)?;
    let session = map
        .get_mut(&session_id)
        .ok_or_else(|| invoke_error("Terminal session not found"))?;
    if session.exited {
        return Err(invoke_error("Terminal session has exited"));
    }
    if child_exited(session)? {
        mark_exited(session, &session_id);
        return Err(invoke_error("Terminal session has exited"));
    }

    if let Err(first_error) = session
        .writer
        .write_all(input.as_bytes())
        .and_then(|_| session.writer.flush())
    {
        // ConPTY can invalidate the writer when an interactive child exits.
        if child_exited(session)? {
            mark_exited(session, &session_id);
            return Err(invoke_error("Terminal session has exited"));
        }

        session.writer = session.master.take_writer().map_err(|error| {
            invoke_error(format!("Could not reconnect terminal input: {error}"))
        })?;
        if let Err(retry_error) = session
            .writer
            .write_all(input.as_bytes())
            .and_then(|_| session.writer.flush())
        {
            if child_exited(session)? {
                mark_exited(session, &session_id);
                return Err(invoke_error("Terminal session has exited"));
            }
            return Err(invoke_error(format!(
                "Terminal input temporarily unavailable: {first_error}; retry failed: {retry_error}"
            )));
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn terminal_list_sessions() -> Result<Vec<TerminalSessionInfo>, InvokeError> {
    let mut map = sessions().lock().map_err(invoke_error)?;
    let mut ordered = Vec::with_capacity(map.len());

    for (id, session) in map.iter_mut() {
        if !session.exited {
            session.exited = child_exited(session)?;
            if session.exited {
                notify_closed(session, id);
            }
        }
        ordered.push((
            session.created_at,
            TerminalSessionInfo {
                id: id.clone(),
                shell: session.shell.clone(),
                cwd: session.cwd.clone(),
                exited: session.exited,
            },
        ));
    }

    ordered.sort_by_key(|(created_at, _)| *created_at);
    Ok(ordered.into_iter().map(|(_, session)| session).collect())
}

#[tauri::command]
pub async fn terminal_resize(session_id: String, cols: u16, rows: u16) -> Result<(), InvokeError> {
    let map = sessions().lock().map_err(invoke_error)?;
    let session = map
        .get(&session_id)
        .ok_or_else(|| invoke_error("Terminal session not found"))?;
    session
        .master
        .resize(PtySize {
            rows: rows.max(2),
            cols: cols.max(2),
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|error| invoke_error(format!("Terminal resize failed: {error}")))
}

#[tauri::command]
pub async fn terminal_read(
    session_id: String,
    offset: usize,
) -> Result<TerminalReadResult, InvokeError> {
    let mut map = sessions().lock().map_err(invoke_error)?;
    let session = map
        .get_mut(&session_id)
        .ok_or_else(|| invoke_error("Terminal session not found"))?;

    let (output, next_offset) = {
        let buffer = session.output.lock().map_err(invoke_error)?;
        let start = offset
            .saturating_sub(buffer.base_offset)
            .min(buffer.bytes.len());
        (
            String::from_utf8_lossy(&buffer.bytes[start..]).into_owned(),
            buffer.next_offset,
        )
    };

    let exited = session.exited || child_exited(session)?;
    if exited && !session.exited {
        notify_closed(session, &session_id);
    }
    session.exited = exited;
    Ok(TerminalReadResult {
        output,
        next_offset,
        exited,
    })
}

#[tauri::command]
pub async fn terminal_close_session(session_id: String) -> Result<(), InvokeError> {
    let mut map = sessions().lock().map_err(invoke_error)?;
    if let Some(mut session) = map.remove(&session_id) {
        notify_closed(&mut session, &session_id);
        terminate(session);
    }
    Ok(())
}

pub fn terminal_close_all_sessions() {
    let Ok(mut map) = sessions().lock() else {
        return;
    };
    let active = std::mem::take(&mut *map);
    drop(map);
    for (id, mut session) in active {
        notify_closed(&mut session, &id);
        terminate(session);
    }
}

/// Resolve a `cd` target relative to the terminal's working directory.
#[tauri::command]
pub async fn terminal_change_dir(cwd: String, target: String) -> Result<String, InvokeError> {
    let base = checked_dir(&cwd)?;
    let target = target.trim();
    let candidate = if target == "~" || target.starts_with("~/") || target.starts_with("~\\") {
        let home = std::env::var_os("USERPROFILE")
            .or_else(|| std::env::var_os("HOME"))
            .ok_or_else(|| invoke_error("Home directory not found"))?;
        PathBuf::from(home).join(target[1..].trim_start_matches(['/', '\\']))
    } else {
        base.join(target)
    };

    let resolved = candidate
        .canonicalize()
        .map_err(|_| invoke_error(format!("Cannot find path '{target}'")))?;
    if !resolved.is_dir() {
        return Err(invoke_error(format!("'{target}' is not a directory")));
    }
    Ok(paths::clean_path(resolved).to_string_lossy().into_owned())
}

fn terminate(session: TerminalSession) {
    if let Ok(mut child) = session.child.lock() {
        let _ = child.kill();
        let _ = child.wait();
    }
}
