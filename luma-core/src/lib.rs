use anyhow::{Context, Result};
use std::fs::create_dir_all;
use std::path::PathBuf;
pub const APP_DATA_DIR_NAME: &str = "Luma";
pub fn data_dir() -> Result<PathBuf> {
    dirs::data_dir()
        .context("Could not resolve the application data directory")
        .map(|base| base.join(APP_DATA_DIR_NAME))
}

pub fn init() -> Result<()> {
    let luma_dir = data_dir()?;

    create_dir_all(&luma_dir).context("Failed to create the Luma data directory")?;

    Ok(())
}
