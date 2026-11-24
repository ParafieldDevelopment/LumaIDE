//! Workspace-scoped file commands.

mod paths;
mod tree;

pub mod watch;

use anyhow::{Context, Result as AnyResult};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use tauri::WebviewWindow;
use tauri::ipc::InvokeError;

use self::tree::Sortable;

#[derive(Debug, Serialize, Deserialize)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
}

impl Sortable for FileEntry {
    fn name(&self) -> &str {
        &self.name
    }

    fn is_dir(&self) -> bool {
        self.is_dir
    }
}

/// Contents plus a content hash, used to detect edits made outside Luma.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileSnapshot {
    pub contents: String,
    pub hash: String,
}

/// Result of a save. `saved: false` means disk changed since `expectedHash`;
/// `diskContents` is the text currently on disk so the UI can offer a reload.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WriteFileResult {
    pub saved: bool,
    pub hash: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disk_contents: Option<String>,
}

fn hash_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

#[tauri::command]
pub async fn read_file(window: WebviewWindow, path: String) -> Result<String, InvokeError> {
    let canonical = scoped_path(window.label(), &path).map_err(InvokeError::from_anyhow)?;
    if !canonical.is_file() {
        return Err(invoke_error(format!("Not a file: {path}")));
    }
    std::fs::read_to_string(&canonical)
        .with_context(|| format!("Failed to read {path}"))
        .map_err(InvokeError::from_anyhow)
}

#[tauri::command]
pub async fn read_file_snapshot(
    window: WebviewWindow,
    path: String,
) -> Result<FileSnapshot, InvokeError> {
    let canonical = scoped_path(window.label(), &path).map_err(InvokeError::from_anyhow)?;
    if !canonical.is_file() {
        return Err(invoke_error(format!("Not a file: {path}")));
    }
    let bytes = std::fs::read(&canonical)
        .with_context(|| format!("Failed to read {path}"))
        .map_err(InvokeError::from_anyhow)?;
    Ok(FileSnapshot {
        contents: String::from_utf8_lossy(&bytes).into_owned(),
        hash: hash_bytes(&bytes),
    })
}

#[tauri::command]
pub async fn write_file(
    window: WebviewWindow,
    path: String,
    contents: String,
    expected_hash: Option<String>,
) -> Result<WriteFileResult, InvokeError> {
    let target = PathBuf::from(&path);
    let parent = target
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(target.as_path());
    paths::resolve_within_workspace(window.label(), parent).map_err(InvokeError::from_anyhow)?;

    // Compare with the bytes on disk before replacing anything.
    let current_bytes = if target.exists() {
        Some(
            std::fs::read(&target)
                .with_context(|| format!("Failed to read {path}"))
                .map_err(InvokeError::from_anyhow)?,
        )
    } else {
        None
    };
    let current_hash = current_bytes.as_deref().map(hash_bytes).unwrap_or_default();

    if let Some(expected) = &expected_hash
        && expected != &current_hash
    {
        return Ok(WriteFileResult {
            saved: false,
            hash: current_hash,
            disk_contents: current_bytes
                .map(|bytes| String::from_utf8_lossy(&bytes).into_owned()),
        });
    }

    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)
            .context("Failed to create parent directory")
            .map_err(InvokeError::from_anyhow)?;
    }

    watch::suppress_self_write(&target);
    let temporary = target.with_extension("tmp");
    std::fs::write(&temporary, &contents)
        .context("Failed to write temporary file")
        .map_err(InvokeError::from_anyhow)?;
    if let Err(error) = std::fs::rename(&temporary, &target) {
        let _ = std::fs::remove_file(&temporary);
        return Err(InvokeError::from_anyhow(
            anyhow::Error::from(error).context(format!("Failed to replace {path}")),
        ));
    }
    Ok(WriteFileResult {
        saved: true,
        hash: hash_bytes(contents.as_bytes()),
        disk_contents: None,
    })
}

#[tauri::command]
pub async fn create_file(window: WebviewWindow, path: String) -> Result<String, InvokeError> {
    let resolved =
        paths::resolve_target_within_workspace(window.label(), PathBuf::from(&path).as_path())
            .map_err(InvokeError::from_anyhow)?;
    if resolved.exists() {
        return Err(invoke_error(format!("File already exists: {path}")));
    }
    watch::suppress_self_write(&resolved);
    std::fs::write(&resolved, "")
        .with_context(|| format!("Failed to create file {path}"))
        .map_err(InvokeError::from_anyhow)?;
    Ok(resolved.to_string_lossy().into_owned())
}

#[tauri::command]
pub async fn create_directory(window: WebviewWindow, path: String) -> Result<String, InvokeError> {
    let resolved =
        paths::resolve_target_within_workspace(window.label(), PathBuf::from(&path).as_path())
            .map_err(InvokeError::from_anyhow)?;
    if resolved.exists() {
        return Err(invoke_error(format!("Directory already exists: {path}")));
    }
    watch::suppress_self_write(&resolved);
    std::fs::create_dir_all(&resolved)
        .with_context(|| format!("Failed to create directory {path}"))
        .map_err(InvokeError::from_anyhow)?;
    Ok(resolved.to_string_lossy().into_owned())
}

#[tauri::command]
pub async fn delete_entry(window: WebviewWindow, path: String) -> Result<(), InvokeError> {
    let canonical = scoped_path(window.label(), &path).map_err(InvokeError::from_anyhow)?;
    let root = paths::workspace_root(window.label()).map_err(InvokeError::from_anyhow)?;
    if canonical == root {
        return Err(invoke_error("Cannot delete the workspace root"));
    }

    watch::suppress_self_write(&canonical);
    if canonical.is_dir() {
        std::fs::remove_dir_all(&canonical)
            .with_context(|| format!("Failed to delete directory {path}"))
            .map_err(InvokeError::from_anyhow)
    } else {
        std::fs::remove_file(&canonical)
            .with_context(|| format!("Failed to delete file {path}"))
            .map_err(InvokeError::from_anyhow)
    }
}

#[tauri::command]
pub async fn read_directory(
    window: WebviewWindow,
    path: String,
) -> Result<Vec<FileEntry>, InvokeError> {
    let canonical = scoped_path(window.label(), &path).map_err(InvokeError::from_anyhow)?;
    if !canonical.is_dir() {
        return Err(invoke_error(format!("Not a directory: {path}")));
    }

    let mut entries = Vec::new();
    for entry in std::fs::read_dir(&canonical)
        .with_context(|| format!("Failed to read directory {path}"))
        .map_err(InvokeError::from_anyhow)?
    {
        let entry = entry
            .context("Failed to read directory entry")
            .map_err(InvokeError::from_anyhow)?;
        let entry_path = entry.path();
        entries.push(FileEntry {
            name: entry.file_name().to_string_lossy().into_owned(),
            path: entry_path.to_string_lossy().into_owned(),
            is_dir: entry_path.is_dir(),
        });
    }
    tree::sort(&mut entries);
    Ok(entries)
}

#[tauri::command]
pub async fn read_directory_tree(
    window: WebviewWindow,
    path: String,
) -> Result<tree::DirectoryTree, InvokeError> {
    let canonical = scoped_path(window.label(), &path).map_err(InvokeError::from_anyhow)?;
    if !canonical.is_dir() {
        return Err(invoke_error(format!("Not a directory: {path}")));
    }
    tree::build(&canonical).map_err(InvokeError::from_anyhow)
}

fn scoped_path(label: &str, path: &str) -> AnyResult<PathBuf> {
    paths::resolve_within_workspace(label, PathBuf::from(path).as_path())
}

fn invoke_error(message: impl std::fmt::Display) -> InvokeError {
    InvokeError::from_anyhow(anyhow::anyhow!("{message}"))
}
