//! Built-in IDE events dispatched on the shared [`crate::event::EventBus`].
//!
//! All events are namespaced with `luma:` and carry a JSON-serializable
//! payload, so native and WASM extensions can listen to them through the
//! extension host API just like Rust code can.

use serde::{Deserialize, Serialize};

use crate::configuration::ConfigurationValue;
use crate::event::Event;
use crate::workspaces::Workspace;

pub const WORKSPACE_OPEN_EVENT: &str = "luma:workspace-open";
pub const WORKSPACE_LOADED_EVENT: &str = "luma:workspace-loaded";
pub const EXTENSION_LOADED_EVENT: &str = "luma:extension-loaded";
pub const EXTENSION_UNLOADED_EVENT: &str = "luma:extension-unloaded";
pub const CONFIGURATION_CHANGED_EVENT: &str = "luma:configuration-changed";
pub const FILES_CHANGED_EVENT: &str = "luma:files-changed";

/// Fired before a workspace is opened. Cancelling it aborts the open flow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceOpenEvent {
    pub workspace: Workspace,
}

impl Event for WorkspaceOpenEvent {
    fn name() -> &'static str {
        WORKSPACE_OPEN_EVENT
    }
}

/// Fired after a workspace's configuration has been loaded and before legacy
/// `on_workspace_load` callbacks run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceLoadedEvent {
    pub workspace: Workspace,
}

impl Event for WorkspaceLoadedEvent {
    fn name() -> &'static str {
        WORKSPACE_LOADED_EVENT
    }
}

/// Fired after an extension has been loaded and initialized. `kind` is
/// `"native"` or `"wasm"`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionLoadedEvent {
    pub name: String,
    pub kind: String,
    pub version: String,
}

impl Event for ExtensionLoadedEvent {
    fn name() -> &'static str {
        EXTENSION_LOADED_EVENT
    }
}

/// Fired after an extension has shut down.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionUnloadedEvent {
    pub name: String,
}

impl Event for ExtensionUnloadedEvent {
    fn name() -> &'static str {
        EXTENSION_UNLOADED_EVENT
    }
}

/// Fired when a configuration value is written or reset. `scope` is
/// `"global"`, `"project"` or `"workspace"`; `old_value`/`new_value` are
/// absent when no value was available.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigurationChangedEvent {
    pub id: String,
    pub scope: String,
    pub old_value: Option<ConfigurationValue>,
    pub new_value: Option<ConfigurationValue>,
}

impl Event for ConfigurationChangedEvent {
    fn name() -> &'static str {
        CONFIGURATION_CHANGED_EVENT
    }
}

/// One change reported by a filesystem watcher. `kind` is `"create"`,
/// `"modify"`, `"remove"` or `"other"`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChangeEvent {
    pub path: String,
    pub kind: String,
    pub is_dir: bool,
}

/// Debounced batch of file changes for one IDE window (`window` is the Tauri
/// window label, `root` the watched workspace root).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilesChangedEvent {
    pub window: String,
    pub root: String,
    pub changes: Vec<FileChangeEvent>,
}

impl Event for FilesChangedEvent {
    fn name() -> &'static str {
        FILES_CHANGED_EVENT
    }
}
