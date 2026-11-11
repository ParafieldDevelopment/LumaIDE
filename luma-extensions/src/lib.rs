use std::sync::{Arc, Mutex};
use log::info;
use anyhow::{Context, Result};
use luma_api::{ExtensionsCoreAPI, EXTENSIONS_API};

pub struct ExtensionsCore {

}
impl ExtensionsCore {
    fn new() -> Self {
        ExtensionsCore {}
    }

    pub fn init() -> Result< Arc<Mutex<Box<dyn ExtensionsCoreAPI>>> > {
        let api: Arc<Mutex<Box<dyn ExtensionsCoreAPI>>> = Arc::new(
            Mutex::new(
                Box::new(
                    Self::new()
                )
            )
        );

        EXTENSIONS_API.set(Arc::clone(&api))
            .map_err(|_| anyhow::anyhow!("Extensions API already exists!"))?;

        Ok(api)
    }
}

impl ExtensionsCoreAPI for ExtensionsCore {
    fn boot(&self) {
        todo!("build plugin bootup here!")
    }
}
pub fn init() -> Result<()> {
    info!("Luma Extensions initialized");

    ExtensionsCore::init()?;

    Ok(())
}