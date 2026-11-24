use luma_api::workspaces::Workspace;
use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

/// Loaded workspaces, keyed by IDE window label (`main`, `main-2`, ...).
#[derive(Default)]
pub struct WorkspaceStore {
    workspaces: RwLock<HashMap<String, Workspace>>,
}

static STORE: OnceLock<WorkspaceStore> = OnceLock::new();

pub fn store() -> &'static WorkspaceStore {
    STORE.get_or_init(WorkspaceStore::default)
}

impl WorkspaceStore {
    pub fn get(&self, label: &str) -> Option<Workspace> {
        self.workspaces.read().ok()?.get(label).cloned()
    }

    pub fn set(&self, label: &str, workspace: Workspace) {
        if let Ok(mut workspaces) = self.workspaces.write() {
            workspaces.insert(label.to_string(), workspace);
        }
    }

    pub fn remove(&self, label: &str) {
        if let Ok(mut workspaces) = self.workspaces.write() {
            workspaces.remove(label);
        }
    }
}

pub fn workspace_for_window(label: &str) -> Option<Workspace> {
    store().get(label)
}
