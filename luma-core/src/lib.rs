pub mod projects;

use anyhow::{Context, Result};
use std::fs::create_dir_all;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use log::{info, error};
use luma_api::{CoreAPI, CORE_API};

pub const APP_DATA_DIR_NAME: &str = "Luma";
pub fn data_dir() -> Result<PathBuf> {
    dirs::data_dir()
        .context("Could not resolve the application data directory")
        .map(|base| base.join(APP_DATA_DIR_NAME))
}

struct Core {
    luma_dir: PathBuf,
}
impl Core {
    fn new(luma_dir: PathBuf) -> Self {
        Self { luma_dir }
    }

    pub fn init(luma_dir: PathBuf) -> anyhow::Result< Arc<Mutex<Box<dyn CoreAPI>>> > {
        let api: Arc<Mutex<Box<dyn CoreAPI>>> = Arc::new(
            Mutex::new(
                Box::new(Self::new(luma_dir))
            )
        );

        CORE_API
            .set(Arc::clone(&api))
            .map_err(|_| anyhow::anyhow!("Core API already initialized"))?;

        Ok(api)
    }
}
impl CoreAPI for Core {
    fn get_data_dir(&self) -> &PathBuf {
        &self.luma_dir
    }
}
pub fn init() -> Result< Arc<Mutex<Box<dyn CoreAPI>>> > {
    let luma_dir = data_dir()?;

    create_dir_all(&luma_dir).context("Failed to create the Luma data directory")?;

    let core_api = Core::init(luma_dir)?;

    info!("Ready!");

    Ok(( core_api ))
}
