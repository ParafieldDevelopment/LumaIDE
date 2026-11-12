use std::sync::{Arc, Mutex, RwLock};
use log::info;
use anyhow::{Context, Result};
use luma_api::{ExtensionsCoreAPI, EXTENSIONS_API, Shared, CoreAPI};

pub struct ExtensionsCore {

}
impl ExtensionsCore {
    fn new() -> Self {
        ExtensionsCore {}
    }

    pub fn init() -> Result< Shared<dyn ExtensionsCoreAPI> >  {
        let api: Shared< dyn ExtensionsCoreAPI > = Arc::new(
            Self::new()
        );

        EXTENSIONS_API.set(Arc::clone(&api))
            .map_err(|_| anyhow::anyhow!("Extensions API already exists!"))?;

        Ok(api)
    }
}

impl ExtensionsCoreAPI for ExtensionsCore {
    fn boot(&self,core_api: &Shared<dyn CoreAPI>) {
        todo!("build plugin bootup here!")
    }
}
pub fn init(core_api: &Shared<dyn CoreAPI>) -> Result< Shared<dyn ExtensionsCoreAPI> > {
    info!("Luma Extensions initialized");

    let extensions_api = ExtensionsCore::init().context("Failed to initialize Luma Extensions")?;
    extensions_api.boot(core_api);

    Ok(extensions_api)
}