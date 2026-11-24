//! Process helpers for tool windows and interactive terminal sessions.

pub mod git;
pub mod sessions;

mod output;

use std::path::PathBuf;
use tauri::ipc::InvokeError;

fn invoke_error(message: impl std::fmt::Display) -> InvokeError {
    InvokeError::from_anyhow(anyhow::anyhow!("{message}"))
}

fn checked_dir(cwd: &str) -> Result<PathBuf, InvokeError> {
    let dir = PathBuf::from(cwd);
    if !dir.is_dir() {
        return Err(invoke_error(format!("Directory does not exist: {cwd}")));
    }
    Ok(dir)
}
