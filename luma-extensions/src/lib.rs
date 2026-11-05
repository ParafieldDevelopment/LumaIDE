use log::info;
use anyhow::{Context, Result};
pub fn init() -> Result<(), ()> {
    info!("Luma Extensions initialized");

    Ok(())
}