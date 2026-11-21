//! The `extension.json` manifest format and helpers for resolving its paths.
//!
//! Every extension is a directory containing an `extension.json` file:
//!
//! ```json
//! {
//!   "name": "example-extension",
//!   "description": "Example native + WASM extension",
//!   "version": "0.1.0",
//!   "type": "native",
//!   "entry": {
//!     "x86_64-pc-windows-msvc": "native/x86_64-pc-windows-msvc/example_extension.dll"
//!   },
//!   "dependencies": [
//!     { "name": "other-extension", "version": "^1.0.0" }
//!   ]
//! }
//! ```
//!
//! Native entries are keyed by Rust target triple so CI can ship one archive for
//! every supported OS/architecture. WASM extensions use a single path instead:
//!
//! ```json
//! { "name": "example", "version": "1.0.0", "type": "wasm", "entry": "example.wasm" }
//! ```

use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result, bail};
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};

/// Name of the manifest file at the root of every extension directory.
pub const MANIFEST_FILE_NAME: &str = "extension.json";

/// How an extension is executed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExtensionKind {
    /// A platform-specific dynamic library implementing the C ABI in [`crate::abi`].
    Native,
    /// A WASM module implementing the exports documented in [`crate::guest`].
    Wasm,
}

impl std::fmt::Display for ExtensionKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Native => f.write_str("native"),
            Self::Wasm => f.write_str("wasm"),
        }
    }
}

/// Entry point(s) for an extension. The shape must match the manifest `type`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ExtensionEntry {
    /// A single WASM module path, relative to the extension root.
    Wasm(PathBuf),
    /// Native library paths keyed by Rust target triple (e.g.
    /// `x86_64-pc-windows-msvc`).
    Native(HashMap<String, PathBuf>),
}

/// A declaration that another extension must be loaded first.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtensionDependency {
    /// The `name` field of the dependency's manifest.
    pub name: String,
    /// Semver requirement, defaults to any version.
    #[serde(default = "any_version")]
    pub version: VersionReq,
    /// Optional dependencies are skipped (with a warning) when unavailable.
    #[serde(default)]
    pub optional: bool,
}

fn any_version() -> VersionReq {
    VersionReq::STAR
}

/// Parsed and validated contents of `extension.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtensionManifest {
    /// Unique, case-sensitive extension identifier.
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub version: Version,
    #[serde(rename = "type")]
    pub kind: ExtensionKind,
    pub entry: ExtensionEntry,
    #[serde(default)]
    pub dependencies: Vec<ExtensionDependency>,
    #[serde(default)]
    pub authors: Vec<String>,
    pub license: Option<String>,
}

/// A manifest together with the directory it was discovered in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionDescriptor {
    pub manifest: ExtensionManifest,
    /// Directory containing `extension.json`.
    pub root: PathBuf,
    /// Full path to `extension.json`.
    pub manifest_path: PathBuf,
}

impl ExtensionManifest {
    /// Parse a manifest from a JSON string and validate it.
    pub fn from_str(json: &str) -> Result<Self> {
        let manifest: Self =
            serde_json::from_str(json).context("Failed to parse extension.json")?;
        manifest.validate()?;
        Ok(manifest)
    }

    /// Read and validate the manifest at `path`.
    pub fn from_path(path: &Path) -> Result<Self> {
        let json = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read {}", path.display()))?;
        Self::from_str(&json).with_context(|| format!("Invalid manifest: {}", path.display()))
    }

    /// Check names, versions, dependencies and entry paths.
    pub fn validate(&self) -> Result<()> {
        if !is_valid_name(&self.name) {
            bail!(
                "Invalid extension name '{}': use 1-128 letters, digits, '.', '_' or '-'",
                self.name
            );
        }

        let mut seen = std::collections::HashSet::new();
        for dependency in &self.dependencies {
            if !is_valid_name(&dependency.name) {
                bail!("Invalid dependency name '{}'", dependency.name);
            }
            if dependency.name == self.name {
                bail!("Extension '{}' cannot depend on itself", self.name);
            }
            if !seen.insert(&dependency.name) {
                bail!(
                    "Duplicate dependency '{}' in extension '{}'",
                    dependency.name,
                    self.name
                );
            }
        }

        match (&self.kind, &self.entry) {
            (ExtensionKind::Wasm, ExtensionEntry::Wasm(path)) => {
                validate_relative_path(path, "wasm entry")?;
            }
            (ExtensionKind::Native, ExtensionEntry::Native(entries)) => {
                if entries.is_empty() {
                    bail!("Extension '{}' has no native entry points", self.name);
                }
                for (target, path) in entries {
                    if target.trim().is_empty() {
                        bail!("Extension '{}' has an empty target triple", self.name);
                    }
                    validate_relative_path(path, &format!("native entry for '{target}'"))?;
                }
            }
            (ExtensionKind::Wasm, ExtensionEntry::Native(_)) => {
                bail!(
                    "Extension '{}' is type 'wasm' but has a native entry map",
                    self.name
                );
            }
            (ExtensionKind::Native, ExtensionEntry::Wasm(_)) => {
                bail!(
                    "Extension '{}' is type 'native' but has a single wasm entry",
                    self.name
                );
            }
        }

        Ok(())
    }

    /// The WASM module path, or `None` for native extensions.
    pub fn wasm_path(&self) -> Option<&Path> {
        match &self.entry {
            ExtensionEntry::Wasm(path) => Some(path),
            ExtensionEntry::Native(_) => None,
        }
    }

    /// Version requirement for a dependency, if declared.
    pub fn dependency_requirement(&self, name: &str) -> Option<&VersionReq> {
        self.dependencies
            .iter()
            .find(|dependency| dependency.name == name)
            .map(|dependency| &dependency.version)
    }
}

impl ExtensionDescriptor {
    /// Load a descriptor from a path to `extension.json`.
    pub fn from_manifest_path(path: &Path) -> Result<Self> {
        let manifest = ExtensionManifest::from_path(path)?;
        let root = path.parent().unwrap_or(Path::new(".")).to_path_buf();
        let canonical_root = root
            .canonicalize()
            .with_context(|| format!("Failed to resolve extension root for {}", path.display()))?;
        let canonical_manifest = path
            .canonicalize()
            .with_context(|| format!("Failed to resolve manifest path {}", path.display()))?;
        if !canonical_manifest.starts_with(&canonical_root) {
            bail!(
                "Extension manifest {} points outside its root",
                path.display()
            );
        }
        validate_entry_containment(&canonical_root, &manifest)?;
        Ok(Self {
            manifest,
            root,
            manifest_path: path.to_path_buf(),
        })
    }

    /// Absolute path of the WASM module, or `None` for native extensions.
    pub fn wasm_module(&self) -> Option<PathBuf> {
        match &self.manifest.entry {
            ExtensionEntry::Wasm(path) => Some(self.root.join(path)),
            ExtensionEntry::Native(_) => None,
        }
    }

    /// Absolute path of the native library built for `target`, or `None`.
    pub fn native_library(&self, target: &str) -> Option<PathBuf> {
        match &self.manifest.entry {
            ExtensionEntry::Native(entries) => entries.get(target).map(|path| self.root.join(path)),
            ExtensionEntry::Wasm(_) => None,
        }
    }
}

/// Entry files (and the directories leading to them) must not be symlinks that
/// escape the extension root. Missing files are left for the loader to report.
fn validate_entry_containment(root: &Path, manifest: &ExtensionManifest) -> Result<()> {
    let mut paths: Vec<&Path> = Vec::new();
    match &manifest.entry {
        ExtensionEntry::Wasm(path) => paths.push(path),
        ExtensionEntry::Native(entries) => paths.extend(entries.values().map(PathBuf::as_path)),
    }
    for relative in paths {
        let candidate = root.join(relative);
        let Ok(canonical) = candidate.canonicalize() else {
            continue;
        };
        if !canonical.starts_with(root) {
            bail!(
                "Extension entry '{}' points outside the extension root",
                relative.display()
            );
        }
    }
    Ok(())
}

fn is_valid_name(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(first) if first.is_ascii_alphanumeric() => {}
        _ => return false,
    }
    name.len() <= 128 && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

fn validate_relative_path(path: &Path, what: &str) -> Result<()> {
    if path.as_os_str().is_empty() {
        bail!("{what} path is empty");
    }
    if !path
        .components()
        .all(|component| matches!(component, Component::Normal(_)))
    {
        bail!(
            "{what} path '{}' must be relative to the extension root",
            path.display()
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(json: &str) -> Result<ExtensionManifest> {
        ExtensionManifest::from_str(json)
    }

    #[test]
    fn parses_native_manifest() {
        let parsed = manifest(
            r#"{
                "name": "example-extension",
                "description": "An example",
                "version": "1.2.3-alpha.1",
                "type": "native",
                "entry": { "x86_64-pc-windows-msvc": "example_extension.dll" },
                "dependencies": [{ "name": "other", "version": "^2.0.0", "optional": true }]
            }"#,
        )
        .unwrap();
        assert_eq!(parsed.kind, ExtensionKind::Native);
        assert_eq!(parsed.version.to_string(), "1.2.3-alpha.1");
        assert!(parsed.dependencies[0].optional);
        assert_eq!(
            parsed.dependencies[0].version,
            VersionReq::parse("^2.0.0").unwrap()
        );
    }

    #[test]
    fn parses_wasm_manifest() {
        let parsed =
            manifest(r#"{"name":"wasm-ext","version":"0.1.0","type":"wasm","entry":"ext.wasm"}"#)
                .unwrap();
        assert_eq!(parsed.kind, ExtensionKind::Wasm);
        assert_eq!(parsed.wasm_path().unwrap(), Path::new("ext.wasm"));
    }

    #[test]
    fn rejects_mismatched_or_unsafe_entries() {
        assert!(
            manifest(r#"{"name":"a","version":"1.0.0","type":"native","entry":"a.wasm"}"#).is_err()
        );
        assert!(
            manifest(r#"{"name":"a","version":"1.0.0","type":"wasm","entry":{"x":"a.wasm"}}"#)
                .is_err()
        );
        assert!(
            manifest(r#"{"name":"a","version":"1.0.0","type":"wasm","entry":"../a.wasm"}"#)
                .is_err()
        );
        assert!(manifest(r#"{"name":"a","version":"1.0.0","type":"native","entry":{}}"#).is_err());
    }

    #[test]
    fn rejects_duplicate_and_self_dependencies() {
        assert!(
            manifest(
                r#"{"name":"a","version":"1.0.0","type":"wasm","entry":"a.wasm",
                "dependencies":[{"name":"b"},{"name":"b"}]}"#
            )
            .is_err()
        );
        assert!(
            manifest(
                r#"{"name":"a","version":"1.0.0","type":"wasm","entry":"a.wasm",
                "dependencies":[{"name":"a"}]}"#
            )
            .is_err()
        );
    }

    #[test]
    fn descriptor_resolves_paths_within_root() {
        let dir = tempfile::tempdir().unwrap();
        let manifest_path = dir.path().join(MANIFEST_FILE_NAME);
        std::fs::write(
            &manifest_path,
            r#"{"name":"a","version":"1.0.0","type":"native",
                "entry":{"x86_64-unknown-linux-gnu":"liba.so"}}"#,
        )
        .unwrap();
        let descriptor = ExtensionDescriptor::from_manifest_path(&manifest_path).unwrap();
        assert_eq!(
            descriptor
                .native_library("x86_64-unknown-linux-gnu")
                .unwrap(),
            dir.path().join("liba.so")
        );
        assert!(descriptor.native_library("aarch64-apple-darwin").is_none());
    }
}
