use anyhow::{bail, Context, Result};
use std::fs::create_dir_all;
pub fn init() -> Result<()> {
    let home_dir = dirs::home_dir().context("Could not determine home directory")?;

    let lumadir = home_dir.join(".luma");

    create_dir_all(&lumadir).context("Failed to create Luma directory")?;

    Ok(())
}