
pub mod projects;
pub mod configuration;

pub use luma_api::{CoreAPI, CORE_API, Shared};

use anyhow::{Context, Result};
use log::info;
use std::fs::create_dir_all;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use dirs::home_dir;
use crate::configuration::ConfigurationManager;

pub const APP_DATA_DIR_NAME: &str = "Luma";
pub const CONFIGURATION_FILE_NAME: &str = "configuration.toml";

pub fn data_dir() -> Result<PathBuf> {
    dirs::data_dir()
        .context("Could not resolve the application data directory")
        .map(|base| base.join(APP_DATA_DIR_NAME))
}

pub fn workspaces_dir() -> Result<PathBuf> {
    if let Some(home) = home_dir() {
        Ok(home.join("projects"))
    } else {
        Err(anyhow::anyhow!("Could not resolve the home directory"))
    }
}

struct Core {
    luma_dir: PathBuf,
    workspaces_dir: PathBuf,
    // IDE-wide (global) configuration manager.
    ide_configuration_manager: Shared<RwLock<ConfigurationManager>>,
    // WorkspaceUi-specific configuration manager.
    workspace_configuration_manager: Shared<RwLock<ConfigurationManager>>,
    workspace_loaded_callbacks: RwLock<Vec<Box<dyn Fn() + Send + Sync>>>,
}

impl Core {
    fn new(luma_dir: PathBuf, workspaces_dir: PathBuf) -> Self {
        let ide_configuration_manager = Arc::new(
            RwLock::new(ConfigurationManager::new())
        );
        let workspace_configuration_manager = Arc::new(
            RwLock::new(ConfigurationManager::new())
        );

        Self {
            luma_dir,
            workspaces_dir,
            ide_configuration_manager,
            workspace_configuration_manager,
            workspace_loaded_callbacks: RwLock::new(Vec::new()),
        }
    }

    pub fn init(luma_dir: PathBuf, workspaces_dir: PathBuf) -> Result<Shared<dyn CoreAPI>> {
        let api: Shared<dyn CoreAPI> = Arc::new(
            Self::new(luma_dir, workspaces_dir)
        );

        CORE_API
            .set(Arc::clone(&api))
            .map_err(|_| anyhow::anyhow!("Core API already initialized"))?;

        Ok(api)
    }

    fn initialize_configuration(&self) -> Result<()> {
        let configuration_path = self.luma_dir.join(CONFIGURATION_FILE_NAME);

        let mut manager = self.ide_configuration_manager
            .write()
            .map_err(|e| anyhow::anyhow!(
                "Failed to acquire configuration lock: {e}"
            ))?;

        // Register the built-in owner if it is not already present.
        let _ = manager.register_owner("luma");

        // TODO: add IDE configuration

        // TODO: add Extensions configuration

        // Load persisted global configuration from LUMA_DIR/configuration.toml.
        manager.load_global(&configuration_path)
            .context("Failed to load global configuration")?;

        Ok(())
    }

    pub fn open_workspace(&self, workspace_path: &Path) -> Result<()> {
        let config_path = if workspace_path.is_dir() {
            workspace_path.join(".luma").join("workspace.toml")
        } else {
            workspace_path.to_path_buf()
        };

        let mut manager = self.workspace_configuration_manager
            .write()
            .map_err(|e| anyhow::anyhow!(
                "Failed to acquire configuration lock: {e}"
            ))?;

        manager.load_workspace(&config_path)
            .context("Failed to load workspace configuration")?;

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

    fn on_workspace_loaded(&self, callback: Box<dyn Fn() + Send + Sync>) {
        if let Ok(mut callbacks) = self.workspace_loaded_callbacks.write() {
            callbacks.push(callback);
        }
    }

    fn open_workspace(&self, workspace_path: &Path) -> Result<()> {
        Core::open_workspace(self, workspace_path)
    }
}

pub fn init() -> Result<Shared<dyn CoreAPI>> {
    let luma_dir = data_dir()?;
    let projects_dir = workspaces_dir()?;

    create_dir_all(&luma_dir)
        .context("Failed to create the Luma data directory")?;

    let core_api = Core::init(luma_dir, projects_dir)
        .context("Failed to initialize Luma API")?;

    core_api.boot().context("Failed to boot Luma API")?;

    info!("Ready!");

    Ok(core_api)
}
