use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

pub type Shared<T> = Arc<T>;

pub trait CoreAPI: Send + Sync {
    fn get_data_dir(&self) -> &PathBuf;

    fn boot(&self) -> anyhow::Result<()>;

    fn open_project(&self, project_id: &str) -> anyhow::Result<()>;
}

pub trait ExtensionsCoreAPI: Send + Sync {
    fn boot(&self, core_api: &Shared<dyn CoreAPI>) -> anyhow::Result<()>;
}

pub static CORE_API: OnceLock<Shared<dyn CoreAPI>> = OnceLock::new();
pub static EXTENSIONS_API: OnceLock<Shared<dyn ExtensionsCoreAPI>> = OnceLock::new();