use anyhow::{Context, Result};
use serde::Serialize;
use std::path::Path;

/// Atomically write a serialized value using a temporary file and rename.
pub fn write<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let contents = serde_json::to_string_pretty(value).context("Failed to serialize JSON")?;
    write_text(path, &contents)
}

/// Atomically write raw JSON text.
pub fn write_text(path: &Path, contents: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create {}", parent.display()))?;
    }

    let temporary = path.with_extension("tmp");
    std::fs::write(&temporary, contents)
        .with_context(|| format!("Failed to write {}", temporary.display()))?;

    if let Err(error) = std::fs::rename(&temporary, path) {
        let _ = std::fs::remove_file(&temporary);
        return Err(
            anyhow::Error::from(error).context(format!("Failed to replace {}", path.display()))
        );
    }

    Ok(())
}
