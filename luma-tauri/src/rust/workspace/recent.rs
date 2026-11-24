use anyhow::Context;
use luma_api::workspaces::{RecentProjects, WorkspaceFace};
use std::path::{Path, PathBuf};
use tauri::ipc::InvokeError;

use crate::{json, paths};
use uuid::Uuid;

const RECENT_FILE: &str = "recent_workspaces.json";

pub fn path() -> Result<PathBuf, InvokeError> {
    luma_core::data_dir()
        .context("Failed to resolve the Luma data directory")
        .map(|dir| dir.join(RECENT_FILE))
        .map_err(InvokeError::from_anyhow)
}

pub fn load() -> Result<RecentProjects, InvokeError> {
    load_from(&path()?)
}

fn load_from(path: &Path) -> Result<RecentProjects, InvokeError> {
    match std::fs::read_to_string(path) {
        Ok(contents) => serde_json::from_str(&contents)
            .with_context(|| format!("Failed to parse recent workspaces from {}", path.display()))
            .map_err(InvokeError::from_anyhow),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(RecentProjects::default()),
        Err(error) => Err(InvokeError::from_anyhow(
            anyhow::Error::from(error).context(format!(
                "Failed to read recent workspaces from {}",
                path.display()
            )),
        )),
    }
}

/// Insert or replace a face in the recent list, stamping it as just opened.
pub fn record(face: WorkspaceFace) -> Result<WorkspaceFace, InvokeError> {
    let mut face = face.mark_opened();
    face.path = paths::clean_path_string(&face.path);

    let path = path()?;
    let mut recent = load_from(&path)?;

    if face.id.trim().is_empty() {
        face.id = existing_for_path(&recent, &face.path)
            .map(|(id, _)| id)
            .unwrap_or_else(|| Uuid::new_v4().to_string());
    }

    recent
        .map
        .retain(|_, entry| !paths::same_path(&entry.path, &face.path));
    recent.map.insert(face.id.clone(), face.clone());
    json::write(&path, &recent).map_err(InvokeError::from_anyhow)?;
    Ok(face)
}

pub fn remove(project_uuid: &str) -> Result<(), InvokeError> {
    let path = path()?;
    let mut recent = load_from(&path)?;
    if recent.map.remove(project_uuid).is_some() {
        json::write(&path, &recent).map_err(InvokeError::from_anyhow)?;
    }
    Ok(())
}

pub fn existing_for_path(recent: &RecentProjects, path: &str) -> Option<(String, WorkspaceFace)> {
    recent
        .map
        .iter()
        .find(|(_, face)| paths::same_path(&face.path, path))
        .map(|(id, face)| (id.clone(), face.clone()))
}

/// Remove entries whose workspace directory no longer exists.
pub fn prune_missing(recent: &mut RecentProjects) -> Vec<String> {
    let missing: Vec<String> = recent
        .map
        .iter()
        .filter(|(_, face)| !Path::new(&face.path).is_dir())
        .map(|(id, _)| id.clone())
        .collect();

    for id in &missing {
        recent.map.remove(id);
    }
    missing
}

/// Recompute derived UI fields before returning a face to the frontend.
pub fn refresh(id: &str, face: &WorkspaceFace) -> Option<WorkspaceFace> {
    let root = PathBuf::from(paths::clean_path_string(&face.path));
    if !root.is_dir() {
        return None;
    }

    let mut refreshed = face.clone();
    if refreshed.id.trim().is_empty() {
        refreshed.id = id.to_string();
    }
    refreshed.path = paths::display_path(&root);
    refreshed.git_branch = detect_git_branch(&root);
    refreshed.logo_path = resolve_logo(refreshed.logo_path.clone(), &root);
    Some(refreshed)
}

pub fn logo_for_path(recent: &RecentProjects, root: &Path) -> Option<String> {
    let path = paths::display_path(root);
    let stored = existing_for_path(recent, &path).and_then(|(_, face)| face.logo_path);
    resolve_logo(stored, root)
}

/// Detect the branch from `.git/HEAD`, falling back to a detached HEAD hash.
pub fn detect_git_branch(root: &Path) -> Option<String> {
    let head = root.join(".git").join("HEAD");
    let contents = std::fs::read_to_string(head).ok()?;
    let contents = contents.trim();

    if let Some(branch) = contents.strip_prefix("ref: refs/heads/") {
        return Some(branch.to_string());
    }
    if contents.len() >= 7 {
        return Some(contents[..7].to_string());
    }
    None
}

fn resolve_logo(stored: Option<String>, root: &Path) -> Option<String> {
    if let Some(logo) = stored.filter(|value| !value.trim().is_empty()) {
        let logo = PathBuf::from(logo);
        if logo.is_absolute() {
            return Some(paths::display_path(&logo));
        }
        if let Ok(data_dir) = luma_core::data_dir() {
            return Some(paths::display_path(&data_dir.join(logo)));
        }
    }
    luma_api::utils::get_workspace_icon(root)
}
