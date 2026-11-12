use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock, RwLock};
use crate::configuration::ConfigurationManager;

pub mod configuration;

pub type Shared<T> = Arc<T>;

pub trait CoreAPI: Send + Sync {
    fn get_data_dir(&self) -> &PathBuf;

    fn get_configuration_file_name(&self) -> &PathBuf;

    fn boot(&self) -> anyhow::Result<()>;

    fn on_workspace_loaded(&self, callback: Box<dyn Fn() + Send + Sync>);

    fn open_workspace(&self, workspace_path: &Path) -> anyhow::Result<()>;
    
    fn get_ide_configuration(&self) -> &Shared<RwLock<ConfigurationManager>>;
}

pub trait ExtensionsCoreAPI: Send + Sync {
    fn boot(&self, core_api: &Shared<dyn CoreAPI>) -> anyhow::Result<()>;
}

pub static CORE_API: OnceLock<Shared<dyn CoreAPI>> = OnceLock::new();
pub static EXTENSIONS_API: OnceLock<Shared<dyn ExtensionsCoreAPI>> = OnceLock::new();