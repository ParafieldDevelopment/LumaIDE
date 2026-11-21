//! Example Luma extension. The same implementation is exported as a native
//! DLL/cdylib and as a WASM module (via the `guest` helper module).

use anyhow::Result;
use luma_api::{CoreAPI, ExtensionsCoreAPI, Shared};
use luma_extensions::{LumaExtension, log};

#[derive(Default)]
struct ExampleExtension;

impl LumaExtension for ExampleExtension {
    fn name(&self) -> &str {
        "example-extension"
    }

    fn initialize(
        &mut self,
        core_api: &Shared<dyn CoreAPI>,
        extensions_core: &Shared<dyn ExtensionsCoreAPI>,
    ) -> Result<()> {
        log(3, "Example extension initializing");
        log(
            3,
            &format!("Luma data directory: {}", core_api.get_data_dir().display()),
        );
        log(
            3,
            &format!(
                "Already loaded extensions: {:?}",
                extensions_core.loaded_extension_names()
            ),
        );
        Ok(())
    }

    fn shutdown(&mut self) -> Result<()> {
        log(3, "Example extension shutting down");
        Ok(())
    }
}

#[cfg(not(target_arch = "wasm32"))]
luma_extensions::declare_luma_extension!(ExampleExtension);

#[cfg(target_arch = "wasm32")]
luma_extensions::declare_wasm_extension!(ExampleExtension);
