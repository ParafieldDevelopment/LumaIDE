//! The interface implemented by every Luma extension.

use anyhow::Result;
use luma_api::{CoreAPI, ExtensionsCoreAPI, Shared};

/// Implemented by native and WASM extensions alike.
///
/// Native extensions export this implementation through
/// [`declare_luma_extension!`](crate::declare_luma_extension). WASM extensions
/// call [`initialize_extension`](crate::guest::initialize_extension) from their
/// `luma_extension_initialize` export.
pub trait LumaExtension: Send {
    /// Construct the extension. The default implementation uses `Default`.
    fn create(
        _core_api: &Shared<dyn CoreAPI>,
        _extensions_core: &Shared<dyn ExtensionsCoreAPI>,
    ) -> Self
    where
        Self: Sized + Default,
    {
        Self::default()
    }

    /// Human-readable name used in log messages.
    fn name(&self) -> &str;

    /// Called once after the host APIs are available.
    fn initialize(
        &mut self,
        core_api: &Shared<dyn CoreAPI>,
        extensions_core: &Shared<dyn ExtensionsCoreAPI>,
    ) -> Result<()>;

    /// Called before the library/module is unloaded.
    fn shutdown(&mut self) -> Result<()>;
}
