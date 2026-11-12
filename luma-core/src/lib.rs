
pub mod projects;

pub use luma_api::{CoreAPI, CORE_API, Shared};

use anyhow::{Context, Result};
use dirs::home_dir;
use log::info;
use luma_api::configuration::{ConfigurationManager, Identifier};
use std::fs::create_dir_all;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

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
    luma_configuration_path: PathBuf,
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

        let luma_configuration_path = luma_dir.join(CONFIGURATION_FILE_NAME);

        Self {
            luma_dir,
            workspaces_dir,
            luma_configuration_path,
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
        let mut ide_manager = self.ide_configuration_manager
            .write()
            .map_err(|e| anyhow::anyhow!(
                "Failed to acquire configuration lock: {e}"
            ))?;

        let mut workspace_manager = self.workspace_configuration_manager
            .write()
            .map_err(|e| anyhow::anyhow!(
                "Failed to acquire configuration lock: {e}"
            ))?;

        // Register the built-in owner if it is not already present.
        let registries_ide = ide_manager.registries_mut();
        registries_ide.register_owner("luma")?;

        let registries_wp = workspace_manager.registries_mut();
        registries_wp.register_owner("luma")?;



        // TODO: add IDE configuration

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

    fn get_configuration_dir(&self) -> &PathBuf {
        &self.luma_configuration_path
    }

    fn get_workspaces_dir(&self) -> &PathBuf {
        &self.workspaces_dir
    }

    fn boot(&self) -> Result<()> {
        self.initialize_configuration().context("Failed to initialize configuration")?;

        Ok(())
    }

    fn on_workspace_load(&self, callback: Box<dyn Fn() + Send + Sync>) {
        if let Ok(mut callbacks) = self.workspace_loaded_callbacks.write() {
            callbacks.push(callback);
        }
    }

    fn open_workspace(&self, workspace_path: &Path) -> Result<()> {
        Core::open_workspace(self, workspace_path)
    }

    fn get_ide_configuration(&self) -> &Shared<RwLock<ConfigurationManager>> {
        &self.ide_configuration_manager
    }

    fn get_workspace_configuration(&self) -> &Shared<RwLock<ConfigurationManager>> {
        &self.workspace_configuration_manager
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
