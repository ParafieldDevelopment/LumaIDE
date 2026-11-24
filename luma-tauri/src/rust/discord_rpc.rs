//! Tauri commands for the Discord Rich Presence extension settings.
//!
//! The extension persists its configuration in `discord_rpc.json` inside the
//! Luma data directory. These commands let the settings UI read and toggle the
//! `enabled` flag and display options at runtime; the extension worker re-reads
//! the file on every poll cycle so changes take effect without restarting Luma.

use luma_core::data_dir;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use tauri::ipc::InvokeError;

const CONFIG_FILE: &str = "discord_rpc.json";

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordRpcConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_true")]
    pub show_workspace: bool,
    #[serde(default = "default_true")]
    pub show_project_type: bool,
    #[serde(default = "default_true")]
    pub show_git_branch: bool,
}

impl Default for DiscordRpcConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            show_workspace: true,
            show_project_type: true,
            show_git_branch: true,
        }
    }
}

fn config_path() -> Result<PathBuf, InvokeError> {
    data_dir()
        .map(|dir| dir.join(CONFIG_FILE))
        .map_err(|err| InvokeError::from_anyhow(anyhow::Error::from(err)))
}

/// Read the complete Discord RPC configuration.
#[tauri::command]
pub async fn get_discord_rpc_config() -> Result<DiscordRpcConfig, InvokeError> {
    let path = config_path()?;
    if !path.exists() {
        return Ok(DiscordRpcConfig::default());
    }

    let text = std::fs::read_to_string(&path)
        .map_err(|err| InvokeError::from_anyhow(anyhow::Error::from(err)))?;
    let config: DiscordRpcConfig = serde_json::from_str(&text)
        .unwrap_or_default();
    Ok(config)
}

/// Save the complete Discord RPC configuration, preserving unknown keys like `client_id`.
#[tauri::command]
pub async fn set_discord_rpc_config(config: DiscordRpcConfig) -> Result<(), InvokeError> {
    let path = config_path()?;

    let mut value: Value = if path.exists() {
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_else(|| Value::Object(Default::default()))
    } else {
        Value::Object(Default::default())
    };

    if let Some(object) = value.as_object_mut() {
        object.insert("enabled".to_string(), Value::Bool(config.enabled));
        object.insert("show_workspace".to_string(), Value::Bool(config.show_workspace));
        object.insert("show_project_type".to_string(), Value::Bool(config.show_project_type));
        object.insert("show_git_branch".to_string(), Value::Bool(config.show_git_branch));
    }

    let json = serde_json::to_string_pretty(&value)
        .map_err(|err| InvokeError::from_anyhow(anyhow::Error::from(err)))?;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| InvokeError::from_anyhow(anyhow::Error::from(err)))?;
    }

    std::fs::write(&path, json)
        .map_err(|err| InvokeError::from_anyhow(anyhow::Error::from(err)))?;

    Ok(())
}

/// Read the persisted `enabled` flag. Defaults to `true` when the file is
/// missing or the key is absent.
#[tauri::command]
pub async fn get_discord_rpc_enabled() -> Result<bool, InvokeError> {
    let path = config_path()?;
    if !path.exists() {
        return Ok(true);
    }

    let text = std::fs::read_to_string(&path)
        .map_err(|err| InvokeError::from_anyhow(anyhow::Error::from(err)))?;
    let value: Value = serde_json::from_str(&text)
        .map_err(|err| InvokeError::from_anyhow(anyhow::Error::from(err)))?;
    Ok(value
        .get("enabled")
        .and_then(Value::as_bool)
        .unwrap_or(true))
}

/// Persist the `enabled` flag, preserving any other keys already present in
/// the config file.
#[tauri::command]
pub async fn set_discord_rpc_enabled(enabled: bool) -> Result<(), InvokeError> {
    let path = config_path()?;

    let mut value: Value = if path.exists() {
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_else(|| Value::Object(Default::default()))
    } else {
        Value::Object(Default::default())
    };

    if let Some(object) = value.as_object_mut() {
        object.insert("enabled".to_string(), Value::Bool(enabled));
    } else {
        value = serde_json::json!({ "enabled": enabled });
    }

    let json = serde_json::to_string_pretty(&value)
        .map_err(|err| InvokeError::from_anyhow(anyhow::Error::from(err)))?;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| InvokeError::from_anyhow(anyhow::Error::from(err)))?;
    }

    std::fs::write(&path, json)
        .map_err(|err| InvokeError::from_anyhow(anyhow::Error::from(err)))?;

    Ok(())
}
