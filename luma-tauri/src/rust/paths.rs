use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// Strip the Windows verbatim prefix (`\\?\C:\...`) added by `canonicalize`.
/// UNC paths are left untouched.
pub fn clean_path(path: PathBuf) -> PathBuf {
    #[cfg(windows)]
    {
        let text = path.to_string_lossy();
        if let Some(rest) = text.strip_prefix(r"\\?\") {
            let bytes = rest.as_bytes();
            if bytes.len() >= 3
                && bytes[0].is_ascii_alphabetic()
                && bytes[1] == b':'
                && bytes[2] == b'\\'
            {
                return PathBuf::from(rest);
            }
        }
    }
    path
}

pub fn clean_path_string(path: &str) -> String {
    clean_path(PathBuf::from(path))
        .to_string_lossy()
        .into_owned()
}

pub fn display_path(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

pub fn same_path(left: &str, right: &str) -> bool {
    let left = Path::new(left)
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(left));
    let right = Path::new(right)
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(right));

    left.to_string_lossy()
        .eq_ignore_ascii_case(&right.to_string_lossy())
}

pub fn canonical_directory(path: &str, context_message: &str) -> Result<PathBuf> {
    let path = PathBuf::from(path)
        .canonicalize()
        .with_context(|| context_message.to_string())?;
    if !path.is_dir() {
        anyhow::bail!("{context_message}: path is not a directory");
    }
    Ok(clean_path(path))
}
