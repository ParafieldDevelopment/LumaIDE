use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

use crate::workspace::workspace_for_window;

pub fn workspace_root(label: &str) -> Result<PathBuf> {
    let workspace = workspace_for_window(label).context("No active workspace")?;
    workspace
        .root()
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

pub fn resolve_within_workspace(label: &str, path: &Path) -> Result<PathBuf> {
    let root = workspace_root(label)?;
    let path = if path.is_relative() {
        root.join(path)
    } else {
        path.to_path_buf()
    };
    let canonical = path.canonicalize().context("Failed to resolve path")?;
    if !is_within(&root, &canonical) {
        anyhow::bail!("Path is outside the active workspace: {}", path.display());
    }
    Ok(canonical)
}

pub fn resolve_target_within_workspace(label: &str, path: &Path) -> Result<PathBuf> {
    let root = workspace_root(label)?;
    let path = if path.is_relative() {
        root.join(path)
    } else {
        path.to_path_buf()
    };
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(root.as_path());
    let canonical_parent = parent
        .canonicalize()
        .context("Parent folder does not exist")?;
    if !is_within(&root, &canonical_parent) {
        anyhow::bail!("Path is outside the active workspace: {}", path.display());
    }
    let filename = path.file_name().context("Invalid filename")?;
    Ok(canonical_parent.join(filename))
}
