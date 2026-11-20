use crate::base_invokes::active_workspace;
use anyhow::{Context, Result as AnyResult};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tauri::ipc::InvokeError;

#[derive(Debug, Serialize, Deserialize)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DirectoryTree {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub children: Option<Vec<DirectoryTree>>,
}

fn workspace_root() -> AnyResult<PathBuf> {
    let guard = active_workspace()
        .read()
        .map_err(|error| anyhow::anyhow!("Failed to read active workspace lock: {error}"))?;
    let project = guard
        .as_ref()
        .context("No active workspace")?;
    PathBuf::from(&project.path)
        .canonicalize()
        .context("Active workspace path does not exist")
}

fn is_within(root: &Path, path: &Path) -> bool {
    match path.strip_prefix(root) {
        Ok(relative) => {
            let relative = relative.to_string_lossy();
            relative.is_empty() || !relative.starts_with("..")
        }
        Err(_) => false,
    }
}

fn resolve_within_workspace(path: &Path) -> AnyResult<PathBuf> {
    let root = workspace_root()?;
    let canonical = path.canonicalize().context("Failed to resolve path")?;
    if !is_within(&root, &canonical) {
        anyhow::bail!("Path is outside the active workspace: {}", path.display());
    }
    Ok(canonical)
}

#[tauri::command]
pub async fn read_file(path: String) -> Result<String, InvokeError> {
    let target = PathBuf::from(&path);
    let canonical = resolve_within_workspace(&target).map_err(InvokeError::from_anyhow)?;
    if !canonical.is_file() {
        return Err(InvokeError::from_anyhow(anyhow::anyhow!(
            "Not a file: {}",
            path
        )));
    }
    std::fs::read_to_string(&canonical)
        .with_context(|| format!("Failed to read {}", path))
        .map_err(InvokeError::from_anyhow)
}

#[tauri::command]
pub async fn write_file(path: String, contents: String) -> Result<(), InvokeError> {
    let target = PathBuf::from(&path);
    let parent = target
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(target.as_path());
    resolve_within_workspace(parent).map_err(InvokeError::from_anyhow)?;

    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)
            .context("Failed to create parent directory")
            .map_err(InvokeError::from_anyhow)?;
    }

    let temporary = target.with_extension("tmp");
    std::fs::write(&temporary, &contents)
        .context("Failed to write temporary file")
        .map_err(InvokeError::from_anyhow)?;
    if let Err(error) = std::fs::rename(&temporary, &target) {
        let _ = std::fs::remove_file(&temporary);
        return Err(InvokeError::from_anyhow(
            anyhow::Error::from(error).context(format!("Failed to replace {}", path)),
        ));
    }
    Ok(())
}

#[tauri::command]
pub async fn read_directory(path: String) -> Result<Vec<FileEntry>, InvokeError> {
    let target = PathBuf::from(&path);
    let canonical = resolve_within_workspace(&target).map_err(InvokeError::from_anyhow)?;
    if !canonical.is_dir() {
        return Err(InvokeError::from_anyhow(anyhow::anyhow!(
            "Not a directory: {}",
            path
        )));
    }

    let mut entries = Vec::new();
    for entry in std::fs::read_dir(&canonical)
        .with_context(|| format!("Failed to read directory {}", path))
        .map_err(InvokeError::from_anyhow)?
    {
        let entry = entry
            .context("Failed to read directory entry")
            .map_err(InvokeError::from_anyhow)?;
        let entry_path = entry.path();
        let name = entry
            .file_name()
            .to_string_lossy()
            .into_owned();
        entries.push(FileEntry {
            name,
            path: entry_path.to_string_lossy().into_owned(),
            is_dir: entry_path.is_dir(),
        });
    }

    entries.sort_by(|left, right| {
        let order = right.is_dir.cmp(&left.is_dir);
        if order != std::cmp::Ordering::Equal {
            return order;
        }
        left.name
            .to_lowercase()
            .cmp(&right.name.to_lowercase())
    });

    Ok(entries)
}

#[tauri::command]
pub async fn read_directory_tree(path: String) -> Result<DirectoryTree, InvokeError> {
    let target = PathBuf::from(&path);
    let canonical = resolve_within_workspace(&target).map_err(InvokeError::from_anyhow)?;
    if !canonical.is_dir() {
        return Err(InvokeError::from_anyhow(anyhow::anyhow!(
            "Not a directory: {}",
            path
        )));
    }
    build_directory_tree(&canonical).map_err(InvokeError::from_anyhow)
}

fn build_directory_tree(path: &Path) -> AnyResult<DirectoryTree> {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("")
        .to_string();
    let mut node = DirectoryTree {
        name,
        path: path.to_string_lossy().into_owned(),
        is_dir: true,
        children: Some(Vec::new()),
    };

    if let Ok(entries) = std::fs::read_dir(path) {
        let mut children: Vec<DirectoryTree> = Vec::new();
        for entry in entries.flatten() {
            let entry_path = entry.path();
            if entry_path.is_dir() {
                children.push(build_directory_tree(&entry_path)?);
            } else {
                children.push(DirectoryTree {
                    name: entry_path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or("")
                        .to_string(),
                    path: entry_path.to_string_lossy().into_owned(),
                    is_dir: false,
                    children: None,
                });
            }
        }
        children.sort_by(|left, right| {
            let order = right.is_dir.cmp(&left.is_dir);
            if order != std::cmp::Ordering::Equal {
                return order;
            }
            left.name
                .to_lowercase()
                .cmp(&right.name.to_lowercase())
        });
        node.children = Some(children);
    }

    Ok(node)
}
