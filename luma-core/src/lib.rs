
pub mod projects;
pub mod configuration;

pub use luma_api::{CoreAPI, CORE_API, Shared};

use anyhow::{Context, Result};
use log::info;
use std::fs::create_dir_all;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use crate::configuration::ConfigurationManager;

pub const APP_DATA_DIR_NAME: &str = "Luma";
pub const CONFIGURATION_FILE_NAME: &str = "configuration.toml";

pub fn data_dir() -> Result<PathBuf> {
    dirs::data_dir()
        .context("Could not resolve the application data directory")
        .map(|base| base.join(APP_DATA_DIR_NAME))
}

struct Core {
    luma_dir: PathBuf,
    configuration_manager: Shared<RwLock<ConfigurationManager>>,
}

impl Core {
    fn new(luma_dir: PathBuf) -> Self {
        let configuration_manager = Arc::new(
            RwLock::new(ConfigurationManager::new())
        );

        Self {
            luma_dir,
            configuration_manager,
        }
    }

    pub fn init(luma_dir: PathBuf) -> Result<Shared<dyn CoreAPI>> {
        let api: Shared<dyn CoreAPI> = Arc::new(
            Self::new(luma_dir)
        );

        CORE_API
            .set(Arc::clone(&api))
            .map_err(|_| anyhow::anyhow!("Core API already initialized"))?;

        Ok(api)
    }

    fn initialize_configuration(&self) -> Result<()> {
        let configuration_path = self.luma_dir.join(CONFIGURATION_FILE_NAME);

        let mut manager = self.configuration_manager
            .write()
            .map_err(|e| anyhow::anyhow!(
                "Failed to acquire configuration lock: {e}"
            ))?;

        // Register the built-in owner if it is not already present.
        let _ = manager.register_owner("luma");

        // Load persisted global configuration from LUMA_DIR/configuration.toml.
        manager.load_global(&configuration_path)
            .context("Failed to load global configuration")?;

        Ok(())
    }

    pub fn open_project(&self, project_id: &str) -> Result<()> {
        let candidate_paths = [
            self.luma_dir.join("projects").join(project_id).join(CONFIGURATION_FILE_NAME),
            self.luma_dir.join("projects").join(format!("{}.toml", project_id)),
            self.luma_dir.join(project_id).join(CONFIGURATION_FILE_NAME),
            self.luma_dir.join(format!("{}.toml", project_id)),
        ];

        let path = candidate_paths
            .iter()
            .find(|p| p.exists())
            .cloned()
            .unwrap_or_else(|| {
                self.luma_dir
                    .join("projects")
                    .join(project_id)
                    .join(CONFIGURATION_FILE_NAME)
            });

        let mut manager = self.configuration_manager
            .write()
            .map_err(|e| anyhow::anyhow!(
                "Failed to acquire configuration lock: {e}"
            ))?;

        manager.load_project(project_id, &path)
            .context("Failed to load project configuration")?;

        Ok(())
    }
}

impl CoreAPI for Core {
    fn get_data_dir(&self) -> &PathBuf {
        &self.luma_dir
    }

    fn boot(&self) -> Result<()> {
        self.initialize_configuration().context("Failed to initialize configuration")?;

        Ok(())
    }

    fn open_project(&self, project_id: &str) -> Result<()> {
        Core::open_project(self, project_id)
    }
}

pub fn init() -> Result<Shared<dyn CoreAPI>> {
    let luma_dir = data_dir()?;

    create_dir_all(&luma_dir)
        .context("Failed to create the Luma data directory")?;

    let core_api = Core::init(luma_dir)
        .context("Failed to initialize Luma API")?;

    core_api.boot().context("Failed to boot Luma API")?;

    info!("Ready!");

    Ok(core_api)
}
