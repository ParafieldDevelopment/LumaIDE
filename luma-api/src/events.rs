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

pub const FILE_OPENED_EVENT: &str = "luma:file-opened";
pub const FILE_CLOSED_EVENT: &str = "luma:file-closed";
pub const FILE_ACTIVATED_EVENT: &str = "luma:file-activated";
pub const FILE_SAVED_EVENT: &str = "luma:file-saved";
pub const FILE_CREATED_EVENT: &str = "luma:file-created";
pub const FILE_DELETED_EVENT: &str = "luma:file-deleted";
pub const DIRECTORY_OPENED_EVENT: &str = "luma:directory-opened";
pub const WORKSPACE_CLOSED_EVENT: &str = "luma:workspace-closed";
pub const TERMINAL_OPENED_EVENT: &str = "luma:terminal-opened";
pub const TERMINAL_CLOSED_EVENT: &str = "luma:terminal-closed";

/// Prefix for the bus event fired by frontend commands declared in an
/// extension manifest. The full name is
/// `{prefix}{extension}.{command-id}` and the payload contains
/// `{ extension, command, workspace }`.
pub const EXTENSION_UI_COMMAND_EVENT_PREFIX: &str = "luma:extension-command:";

/// Fired when the editor opens a file in a tab.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileOpenedEvent {
    pub path: String,
    pub window: String,
}

impl Event for FileOpenedEvent {
    fn name() -> &'static str {
        FILE_OPENED_EVENT
    }
}

/// Fired when the editor closes a file tab.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileClosedEvent {
    pub path: String,
    pub window: String,
}

impl Event for FileClosedEvent {
    fn name() -> &'static str {
        FILE_CLOSED_EVENT
    }
}

/// Fired when the active editor tab changes. `path` is `None` when no file is
/// active.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileActivatedEvent {
    pub path: Option<String>,
    pub window: String,
}

impl Event for FileActivatedEvent {
    fn name() -> &'static str {
        FILE_ACTIVATED_EVENT
    }
}

/// Fired after a file is saved through the IDE. `hash` is the SHA-256 of the
/// bytes written to disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSavedEvent {
    pub path: String,
    pub hash: String,
    pub window: String,
}

impl Event for FileSavedEvent {
    fn name() -> &'static str {
        FILE_SAVED_EVENT
    }
}

/// Fired after Luma creates a file or directory. `is_dir` distinguishes them.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileCreatedEvent {
    pub path: String,
    pub is_dir: bool,
    pub window: String,
}

impl Event for FileCreatedEvent {
    fn name() -> &'static str {
        FILE_CREATED_EVENT
    }
}

/// Fired after Luma deletes a file or directory. `is_dir` distinguishes them.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDeletedEvent {
    pub path: String,
    pub is_dir: bool,
    pub window: String,
}

impl Event for FileDeletedEvent {
    fn name() -> &'static str {
        FILE_DELETED_EVENT
    }
}

/// Fired when the project explorer reads a directory. This also covers
/// expanding a node in the explorer tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectoryOpenedEvent {
    pub path: String,
    pub window: String,
}

impl Event for DirectoryOpenedEvent {
    fn name() -> &'static str {
        DIRECTORY_OPENED_EVENT
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

/// Fired when an IDE window for a workspace is destroyed, after the watcher is
/// stopped and before the workspace is removed from the store.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceClosedEvent {
    pub workspace: Workspace,
    pub window: String,
}

impl Event for WorkspaceClosedEvent {
    fn name() -> &'static str {
        WORKSPACE_CLOSED_EVENT
    }
}

/// Fired after a terminal PTY session is created.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalOpenedEvent {
    pub id: String,
    pub shell: String,
    pub cwd: String,
}

impl Event for TerminalOpenedEvent {
    fn name() -> &'static str {
        TERMINAL_OPENED_EVENT
    }
}

/// Fired when a terminal PTY session is closed or terminated.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalClosedEvent {
    pub id: String,
    pub shell: String,
    pub cwd: String,
}

impl Event for TerminalClosedEvent {
    fn name() -> &'static str {
        TERMINAL_CLOSED_EVENT
    }
}
