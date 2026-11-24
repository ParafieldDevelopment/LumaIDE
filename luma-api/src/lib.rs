use crate::configuration::ConfigurationManager;
use std::panic::{self, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock, RwLock};

pub mod configuration;
pub mod utils;
pub mod workspaces;

pub type Shared<T> = Arc<T>;

/// Fire registered lifecycle callbacks without letting one bad listener stop
/// workspace loading.
pub fn notify_listeners(callbacks: &RwLock<Vec<Box<dyn Fn() + Send + Sync>>>) {
    let Ok(listeners) = callbacks.read() else {
        log::warn!("Workspace listener lock was poisoned");
        return;
    };

    for listener in listeners.iter() {
        if let Err(payload) = panic::catch_unwind(AssertUnwindSafe(|| listener())) {
            let message = payload
                .downcast_ref::<String>()
                .map(String::as_str)
                .or_else(|| payload.downcast_ref::<&str>().copied())
                .unwrap_or("unknown panic");
            log::warn!("Workspace load listener panicked: {message}");
        }
    }
}

pub trait CoreAPI: Send + Sync {
    fn get_data_dir(&self) -> &PathBuf;

    fn get_configuration_dir(&self) -> &PathBuf;

    fn get_workspaces_dir(&self) -> &PathBuf;

    fn boot(&self) -> anyhow::Result<()>;

    fn on_workspace_load(&self, callback: Box<dyn Fn() + Send + Sync>);

    /// Called by [`crate::workspaces::Workspace::load`] after the workspace
    /// configuration has been loaded. Implementations should notify listeners
    /// registered with [`Self::on_workspace_load`].
    fn notify_workspace_loaded(&self) {}

    fn open_workspace(&self, workspace_path: &Path) -> anyhow::Result<()>;

    fn get_ide_configuration(&self) -> &Shared<RwLock<ConfigurationManager>>;

    fn get_workspace_configuration(&self) -> &Shared<RwLock<ConfigurationManager>>;
}

pub trait ExtensionsCoreAPI: Send + Sync {
    fn boot(&self, core_api: &Shared<dyn CoreAPI>) -> anyhow::Result<()>;

    /// Names of extensions that are currently loaded.
    fn loaded_extension_names(&self) -> Vec<String> {
        Vec::new()
    }

    /// `name: error` descriptions of extensions that failed to load.
    fn extension_failures(&self) -> Vec<String> {
        Vec::new()
    }
}

pub static CORE_API: OnceLock<Shared<dyn CoreAPI>> = OnceLock::new();
pub static EXTENSIONS_API: OnceLock<Shared<dyn ExtensionsCoreAPI>> = OnceLock::new();
