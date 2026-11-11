use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};

pub trait CoreAPI: Send + Sync {
    fn get_data_dir(&self) -> &PathBuf;
}

pub trait ExtensionsCoreAPI: Send + Sync {
    fn boot(&self);
}
pub static CORE_API: OnceLock< Arc<Mutex<Box<dyn CoreAPI>>> > = OnceLock::new();
pub static EXTENSIONS_API: OnceLock< Arc<Mutex<Box<dyn ExtensionsCoreAPI>>> > = OnceLock::new();
