//! Manual smoke test for the extension loader.
//!
//! ```sh
//! cargo run -p luma-extensions --example load_from_dir -- ./extensions-dir
//! ```
//!
//! The directory may contain extension ZIPs and/or unpacked extension
//! directories. It prints what loaded and what failed.

use std::path::PathBuf;

use anyhow::Result;
use luma_extensions::ExtensionManager;

fn main() -> Result<()> {
    let roots: Vec<PathBuf> = std::env::args().skip(1).map(PathBuf::from).collect();
    if roots.is_empty() {
        anyhow::bail!("Usage: load_from_dir <extensions-directory> [...]");
    }

    let core_api = luma_core::init()?;
    let cache_dir = roots[0].join(".cache");
    let mut manager = ExtensionManager::new();
    manager.load_all(&core_api, roots, &cache_dir)?;

    for extension in manager.loaded() {
        println!(
            "LOADED  {} v{} ({:?})",
            extension.name(),
            extension.version(),
            extension.kind()
        );
    }
    for failure in manager.failures() {
        println!("FAILED  {}: {}", failure.name, failure.error);
    }

    manager.unload_all()?;
    Ok(())
}
