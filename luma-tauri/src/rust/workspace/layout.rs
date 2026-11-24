use serde_json::Value;
use std::path::{Path, PathBuf};
use tauri::ipc::InvokeError;

use crate::{json, paths};

const LAYOUT_FILE: &str = "tool-windows.json";

#[tauri::command]
pub async fn load_tool_window_layout(
    workspace_path: String,
) -> Result<Option<String>, InvokeError> {
    let root = root(&workspace_path)?;
    match std::fs::read_to_string(layout_path(&root)) {
        Ok(contents) => {
            serde_json::from_str::<Value>(&contents).map_err(|error| {
                invoke_error(format!("Invalid tool-window layout JSON: {error}"))
            })?;
            Ok(Some(contents))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(invoke_error(format!(
            "Failed to read tool-window layout: {error}"
        ))),
    }
}

#[tauri::command]
pub async fn save_tool_window_layout(
    workspace_path: String,
    layout: String,
) -> Result<(), InvokeError> {
    let root = root(&workspace_path)?;
    let value: Value = serde_json::from_str(&layout)
        .map_err(|error| invoke_error(format!("Invalid tool-window layout JSON: {error}")))?;
    if value.get("version").and_then(Value::as_u64).is_none() {
        return Err(invoke_error("Unsupported tool-window layout version"));
    }

    json::write_text(
        &layout_path(&root),
        &serde_json::to_string_pretty(&value).unwrap(),
    )
    .map_err(InvokeError::from_anyhow)
}

fn root(workspace_path: &str) -> Result<PathBuf, InvokeError> {
    paths::canonical_directory(workspace_path, "Workspace folder does not exist")
        .map_err(InvokeError::from_anyhow)
}

fn layout_path(root: &Path) -> PathBuf {
    root.join(".luma").join(LAYOUT_FILE)
}

fn invoke_error(message: impl std::fmt::Display) -> InvokeError {
    InvokeError::from_anyhow(anyhow::anyhow!("{message}"))
}
