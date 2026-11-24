use anyhow::{Context, Result as AnyResult};
use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};
use tauri::ipc::InvokeError;
use uuid::Uuid;

use crate::json;
use luma_api::workspaces::unix_timestamp;

const METADATA_FILE: &str = "project.json";

/// Stable on-disk descriptor stored in `<workspace>/.luma/project.json`.
///
/// Transient UI state (git branch, logo, last-opened time) deliberately lives
/// in `WorkspaceFace` and the recent list instead.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceMetadata {
    /// Serialized as `project_uuid` to stay compatible with existing
    /// `<workspace>/.luma/project.json` files.
    #[serde(rename = "project_uuid", alias = "id")]
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub created_at: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template: Option<String>,
}

pub fn path(root: &Path) -> PathBuf {
    root.join(".luma").join(METADATA_FILE)
}

pub fn load_or_initialize(
    root: &Path,
    default_title: &str,
    existing_id: Option<String>,
) -> Result<WorkspaceMetadata, InvokeError> {
    let path = path(root);
    if path.exists() {
        let contents = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read {}", path.display()))
            .map_err(InvokeError::from_anyhow)?;
        let metadata: WorkspaceMetadata = serde_json::from_str(&contents)
            .with_context(|| format!("Invalid workspace metadata at {}", path.display()))
            .map_err(InvokeError::from_anyhow)?;

        Uuid::parse_str(&metadata.id)
            .context("Workspace metadata contains an invalid id")
            .map_err(InvokeError::from_anyhow)?;
        if metadata.title.trim().is_empty() {
            return Err(invoke_error("Workspace metadata has an empty title"));
        }
        return Ok(metadata);
    }

    let metadata = WorkspaceMetadata {
        id: existing_id
            .filter(|id| !id.trim().is_empty())
            .unwrap_or_else(|| Uuid::new_v4().to_string()),
        title: default_title.to_string(),
        created_at: unix_timestamp(),
        project_type: None,
        runtime: None,
        template: None,
    };
    write(root, &metadata)?;
    Ok(metadata)
}

pub fn write(root: &Path, metadata: &WorkspaceMetadata) -> Result<(), InvokeError> {
    json::write(&path(root), metadata).map_err(InvokeError::from_anyhow)
}

/// Validate an optional wizard id. The frontend owns the set of valid ids;
/// the backend only enforces the format.
pub fn validate_choice_id(
    label: &str,
    value: Option<String>,
) -> Result<Option<String>, InvokeError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    if value.len() > 64
        || !value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
    {
        return Err(invoke_error(format!("Invalid {label}")));
    }
    Ok(Some(value.to_string()))
}

/// Ensure a template file path is relative and cannot escape the workspace.
pub fn validate_template_file_path(path_str: &str) -> AnyResult<PathBuf> {
    let path = Path::new(path_str);
    if path.is_absolute() {
        anyhow::bail!("Template file path must be relative: {path_str}");
    }

    for component in path.components() {
        match component {
            Component::Normal(name) => {
                let name = name.to_string_lossy();
                if name.contains('\0') {
                    anyhow::bail!("Template filename cannot contain null bytes: {path_str}");
                }
                #[cfg(windows)]
                if name.chars().any(|ch| "<>:\"|?*".contains(ch)) {
                    anyhow::bail!("Template filename contains invalid characters: {path_str}");
                }
            }
            Component::CurDir => continue,
            _ => anyhow::bail!("Invalid path traversal in template file: {path_str}"),
        }
    }

    Ok(path.to_path_buf())
}

fn invoke_error(message: impl std::fmt::Display) -> InvokeError {
    InvokeError::from_anyhow(anyhow::anyhow!("{message}"))
}
