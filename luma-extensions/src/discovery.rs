//! Finds extensions on disk and unpacks ZIP archives.
//!
//! Extensions live in `LUMA_DIR/extensions` as `.zip` archives. Everything is
//! stored at the root of the archive (including `extension.json`), so the
//! loader unpacks each archive into a cache directory and treats that as the
//! extension root. Unpacked directories with a manifest are also accepted,
//! which is handy when developing an extension locally.

use std::collections::HashSet;
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};
use zip::ZipArchive;

use crate::manifest::{ExtensionDescriptor, ExtensionManifest, MANIFEST_FILE_NAME};

/// Directory inside the extensions folder where archives are unpacked.
pub const CACHE_DIR_NAME: &str = ".cache";
/// Maximum size of an extension archive on disk (256 MiB).
const MAX_ARCHIVE_SIZE: u64 = 256 * 1024 * 1024;
/// Maximum total uncompressed size of one extension archive (256 MiB).
const MAX_UNCOMPRESSED_SIZE: u64 = 256 * 1024 * 1024;

/// A single extension that could not be discovered.
#[derive(Debug, Clone)]
pub struct DiscoveryFailure {
    pub path: PathBuf,
    pub error: String,
}

/// Descriptors found during discovery, plus per-extension failures.
#[derive(Debug, Default)]
pub struct DiscoveryResult {
    pub descriptors: Vec<ExtensionDescriptor>,
    pub failures: Vec<DiscoveryFailure>,
}

/// Scan `roots` for extension ZIPs (and unpacked extension directories).
///
/// `cache_dir` is where ZIP contents are extracted; it must be writable.
pub fn discover_extensions(
    roots: impl IntoIterator<Item = PathBuf>,
    cache_dir: &Path,
) -> Result<Vec<ExtensionDescriptor>> {
    Ok(discover_extensions_with_failures(roots, cache_dir)?.descriptors)
}

/// Like [`discover_extensions`], but reports bad archives/manifests individually
/// instead of aborting the whole scan.
pub fn discover_extensions_with_failures(
    roots: impl IntoIterator<Item = PathBuf>,
    cache_dir: &Path,
) -> Result<DiscoveryResult> {
    let mut result = DiscoveryResult::default();

    for root in roots {
        if !root.exists() {
            continue;
        }
        let manifest = root.join(MANIFEST_FILE_NAME);
        if manifest.is_file() {
            match ExtensionDescriptor::from_manifest_path(&manifest) {
                Ok(descriptor) => result.descriptors.push(descriptor),
                Err(error) => record_failure(&mut result, &manifest, error),
            }
            continue;
        }
        if root.is_file() {
            if root
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("zip"))
                && let Err(error) = extract_archive(&root, cache_dir)
            {
                record_failure(&mut result, &root, error);
            }
            continue;
        }
        if !root.is_dir() {
            continue;
        }

        let entries = match std::fs::read_dir(&root) {
            Ok(entries) => entries,
            Err(error) => {
                record_failure(
                    &mut result,
                    &root,
                    format!(
                        "Failed to scan extension directory {}: {error:#}",
                        root.display()
                    ),
                );
                continue;
            }
        };
        let mut entries: Vec<PathBuf> = entries
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|path| !is_hidden(path))
            .collect();
        entries.sort();

        for path in entries {
            if path.is_file()
                && path
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("zip"))
            {
                match extract_archive(&path, cache_dir) {
                    Ok(descriptor) => result.descriptors.push(descriptor),
                    Err(error) => record_failure(&mut result, &path, error),
                }
            } else if path.is_dir() && path.join(MANIFEST_FILE_NAME).is_file() {
                // Unpacked extension (development layout).
                let manifest = path.join(MANIFEST_FILE_NAME);
                match ExtensionDescriptor::from_manifest_path(&manifest) {
                    Ok(descriptor) => result.descriptors.push(descriptor),
                    Err(error) => record_failure(&mut result, &manifest, error),
                }
            }
        }
    }

    result
        .descriptors
        .sort_by(|a, b| a.manifest.name.cmp(&b.manifest.name));
    Ok(result)
}

fn record_failure(result: &mut DiscoveryResult, path: &Path, error: impl std::fmt::Display) {
    result.failures.push(DiscoveryFailure {
        path: path.to_path_buf(),
        error: format!("{error:#}"),
    });
}

/// Extension search paths: `<LUMA_DIR>/extensions` plus any paths in
/// `LUMA_EXTENSIONS_PATH` (platform path separator between entries).
pub fn default_extension_roots(data_dir: &Path) -> Vec<PathBuf> {
    let mut roots = vec![data_dir.join("extensions")];
    if let Some(paths) = std::env::var_os("LUMA_EXTENSIONS_PATH") {
        roots.extend(std::env::split_paths(&paths));
    }
    roots
}

/// Extract a ZIP archive into `cache_dir` and load its root manifest.
///
/// The cache key is a SHA-256 digest of the archive bytes, so replacing a ZIP
/// with a new version automatically produces a fresh extraction directory.
pub fn extract_archive(archive_path: &Path, cache_dir: &Path) -> Result<ExtensionDescriptor> {
    let mut file = File::open(archive_path).with_context(|| {
        format!(
            "Failed to open extension archive {}",
            archive_path.display()
        )
    })?;
    let hash = archive_digest(&mut file, archive_path)?;
    file.seek(SeekFrom::Start(0))
        .with_context(|| format!("Failed to rewind {}", archive_path.display()))?;
    let mut archive = ZipArchive::new(BufReader::new(file))
        .with_context(|| format!("Invalid ZIP archive {}", archive_path.display()))?;

    if archive.has_duplicate_names() {
        bail!(
            "Extension archive {} contains duplicate entry names",
            archive_path.display()
        );
    }

    let manifest_bytes = {
        let mut file = archive.by_name(MANIFEST_FILE_NAME).with_context(|| {
            format!(
                "Extension archive {} has no root {MANIFEST_FILE_NAME}",
                archive_path.display()
            )
        })?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)
            .context("Failed to read manifest from archive")?;
        bytes
    };
    let manifest_json =
        String::from_utf8(manifest_bytes).context("extension.json must be UTF-8")?;
    let manifest = ExtensionManifest::from_str(&manifest_json)
        .with_context(|| format!("Invalid manifest in {}", archive_path.display()))?;

    let target = cache_dir.join(format!("{}-{}", manifest.name, hash));
    let extracted_manifest = target.join(MANIFEST_FILE_NAME);
    let cache_is_valid = !std::fs::symlink_metadata(&target)
        .is_ok_and(|metadata| metadata.file_type().is_symlink())
        && std::fs::symlink_metadata(&extracted_manifest)
            .is_ok_and(|metadata| metadata.is_file() && !metadata.file_type().is_symlink());
    if !cache_is_valid {
        write_archive(&mut archive, &target)?;
    }

    ExtensionDescriptor::from_manifest_path(&extracted_manifest).with_context(|| {
        format!(
            "Failed to load extracted extension from {}",
            archive_path.display()
        )
    })
}

/// Hash the archive in chunks so a large file is never loaded wholly into
/// memory, while enforcing an input size limit.
fn archive_digest(file: &mut File, path: &Path) -> Result<String> {
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    let mut total = 0u64;

    loop {
        let read = file
            .read(&mut buffer)
            .with_context(|| format!("Failed to read extension archive {}", path.display()))?;
        if read == 0 {
            break;
        }
        total = total
            .checked_add(read as u64)
            .context("Extension archive size overflow")?;
        if total > MAX_ARCHIVE_SIZE {
            bail!("Extension archive exceeds the {MAX_ARCHIVE_SIZE} byte limit");
        }
        hasher.update(&buffer[..read]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

fn write_archive<R: Read + std::io::Seek>(
    archive: &mut ZipArchive<R>,
    target: &Path,
) -> Result<()> {
    // Unpack into a temporary directory first: an interrupted extraction must
    // never leave a half-written extension that looks valid on the next boot.
    let staging = target.parent().unwrap_or(Path::new(".")).join(format!(
        ".staging-{}",
        target.file_name().unwrap_or_default().to_string_lossy()
    ));
    remove_path(&staging).with_context(|| {
        format!(
            "Failed to clean stale extraction directory {}",
            staging.display()
        )
    })?;
    std::fs::create_dir_all(&staging)
        .with_context(|| format!("Failed to create {}", staging.display()))?;

    let mut total = 0u64;
    let mut written = HashSet::new();
    for index in 0..archive.len() {
        let entry = archive
            .by_index(index)
            .context("Failed to read ZIP entry")?;
        let name = entry.name().context("Invalid ZIP entry name")?.to_string();
        let relative = safe_entry_path(&name)?;
        if !written.insert(relative.clone()) {
            bail!("Extension archive has duplicate path '{name}'");
        }
        // Reject Unix symlinks, including symlinks flagged as directories: they
        // could point outside the extraction root.
        if let Some(mode) = entry.unix_mode()
            && (mode & 0o170000) == 0o120000
        {
            bail!("Extension archive contains a symlink: '{name}'");
        }
        if entry.is_dir() {
            std::fs::create_dir_all(staging.join(relative))
                .with_context(|| format!("Failed to create ZIP directory '{name}'"))?;
            continue;
        }

        let destination = staging.join(relative);
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create parent directory for '{name}'"))?;
        }
        let mut content = std::fs::File::create(&destination)
            .with_context(|| format!("Failed to create ZIP entry '{name}'"))?;
        // Bound the actual decompressed bytes, not just the size in the ZIP
        // header, so a lying header cannot bypass the limit.
        let remaining = MAX_UNCOMPRESSED_SIZE.saturating_sub(total);
        let mut limited = entry.take(remaining.saturating_add(1));
        let copied = std::io::copy(&mut limited, &mut content)
            .with_context(|| format!("Failed to write ZIP entry '{name}'"))?;
        total = total
            .checked_add(copied)
            .context("Extension archive size overflow")?;
        if total > MAX_UNCOMPRESSED_SIZE {
            bail!("Extension archive exceeds the {MAX_UNCOMPRESSED_SIZE} byte limit");
        }
    }

    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create {}", parent.display()))?;
    }
    remove_path(target).with_context(|| format!("Failed to replace {}", target.display()))?;
    std::fs::rename(&staging, target).with_context(|| {
        format!(
            "Failed to move extracted extension from {} to {}",
            staging.display(),
            target.display()
        )
    })?;
    Ok(())
}

/// Remove a file, directory or symlink without following symlinks.
fn remove_path(path: &Path) -> Result<()> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) => {
            let result = if metadata.file_type().is_symlink() || !metadata.is_dir() {
                std::fs::remove_file(path)
            } else {
                std::fs::remove_dir_all(path)
            };
            result.with_context(|| format!("Failed to remove {}", path.display()))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).with_context(|| format!("Failed to inspect {}", path.display())),
    }
}

/// Reject absolute paths and `..` traversal (zip-slip) and normalise separators.
fn safe_entry_path(name: &str) -> Result<PathBuf> {
    let normalised = name.replace('\\', "/");
    let path = Path::new(&normalised);
    let mut result = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => result.push(part),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                bail!("Unsafe path in extension archive: '{name}'");
            }
        }
    }
    if result.as_os_str().is_empty() {
        bail!("Empty path in extension archive");
    }
    Ok(result)
}

fn is_hidden(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with('.'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn make_zip(entries: &[(&str, &str)]) -> Vec<u8> {
        let mut buffer = Vec::new();
        {
            let mut writer = zip::ZipWriter::new(std::io::Cursor::new(&mut buffer));
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated);
            for (name, content) in entries {
                writer.start_file(*name, options).unwrap();
                writer.write_all(content.as_bytes()).unwrap();
            }
            writer.finish().unwrap();
        }
        buffer
    }

    #[test]
    fn extracts_zip_and_loads_manifest() {
        let dir = tempfile::tempdir().unwrap();
        let archive = dir.path().join("test.zip");
        std::fs::write(
            &archive,
            make_zip(&[
                (
                    MANIFEST_FILE_NAME,
                    r#"{"name":"zipped","version":"1.0.0","type":"wasm","entry":"mod.wasm"}"#,
                ),
                ("mod.wasm", "not really wasm"),
            ]),
        )
        .unwrap();

        let descriptor = extract_archive(&archive, &dir.path().join(CACHE_DIR_NAME)).unwrap();
        assert_eq!(descriptor.manifest.name, "zipped");
        assert_eq!(
            descriptor.wasm_module().unwrap(),
            descriptor.root.join("mod.wasm")
        );
        assert!(descriptor.root.join("mod.wasm").is_file());
    }

    #[test]
    fn rejects_zip_slip() {
        let name = "../outside.txt";
        assert!(safe_entry_path(name).is_err());
        assert!(safe_entry_path("/absolute.txt").is_err());
        assert!(safe_entry_path("nested/ok.txt").is_ok());
    }

    #[test]
    fn discovery_reads_zips_and_unpacked_directories() {
        let dir = tempfile::tempdir().unwrap();
        let extensions = dir.path().join("extensions");
        std::fs::create_dir(&extensions).unwrap();
        std::fs::write(
            extensions.join("zipped.zip"),
            make_zip(&[(
                MANIFEST_FILE_NAME,
                r#"{"name":"zipped","version":"1.0.0","type":"wasm","entry":"z.wasm"}"#,
            )]),
        )
        .unwrap();
        let unpacked = extensions.join("unpacked");
        std::fs::create_dir(&unpacked).unwrap();
        std::fs::write(
            unpacked.join(MANIFEST_FILE_NAME),
            r#"{"name":"unpacked","version":"1.0.0","type":"wasm","entry":"u.wasm"}"#,
        )
        .unwrap();

        let found =
            discover_extensions([extensions.clone()], &extensions.join(CACHE_DIR_NAME)).unwrap();
        let names: Vec<_> = found
            .iter()
            .map(|item| item.manifest.name.as_str())
            .collect();
        assert_eq!(names, vec!["unpacked", "zipped"]);
    }

    #[test]
    fn bad_archive_does_not_abort_discovery() {
        let dir = tempfile::tempdir().unwrap();
        let extensions = dir.path().join("extensions");
        std::fs::create_dir(&extensions).unwrap();
        std::fs::write(extensions.join("bad.zip"), b"not a zip").unwrap();
        std::fs::write(
            extensions.join("good.zip"),
            make_zip(&[(
                MANIFEST_FILE_NAME,
                r#"{"name":"good","version":"1.0.0","type":"wasm","entry":"g.wasm"}"#,
            )]),
        )
        .unwrap();

        let found = discover_extensions_with_failures(
            [extensions.clone()],
            &extensions.join(CACHE_DIR_NAME),
        )
        .unwrap();
        assert_eq!(found.descriptors.len(), 1);
        assert_eq!(found.descriptors[0].manifest.name, "good");
        assert_eq!(found.failures.len(), 1);
        assert!(found.failures[0].path.ends_with("bad.zip"));
    }
}
