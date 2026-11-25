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
use luma_api::events::EXTENSION_UI_COMMAND_EVENT_PREFIX;
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

/// Optional frontend UI shipped by an extension.
///
/// The entry is an HTML file loaded into a sandboxed iframe. Native and WASM
/// extensions do not need to know about it: the host reads these declarations
/// directly from `extension.json` and the Vue app registers them on mount.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrontendManifest {
    /// HTML file, relative to the extension root.
    pub entry: Option<PathBuf>,
    #[serde(default, alias = "contributes")]
    pub contributions: FrontendContributions,
}

/// UI items an extension contributes to the IDE shell.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrontendContributions {
    #[serde(default)]
    pub commands: Vec<FrontendCommandSpec>,
    #[serde(default)]
    pub menus: Vec<FrontendMenuSpec>,
    #[serde(default)]
    pub tool_windows: Vec<FrontendToolWindowSpec>,
    /// Bus event names the iframe is allowed to receive and send.
    #[serde(default)]
    pub events: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrontendCommandSpec {
    /// Short id, e.g. `openPanel`. The host namespaces it with the extension name.
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub keys: Option<String>,
    /// SVG path data for a 16px icon.
    #[serde(default)]
    pub icon: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrontendMenuSpec {
    /// Menu location, e.g. `titlebar/main`.
    pub location: String,
    pub command_id: String,
    #[serde(default)]
    pub group: Option<String>,
    #[serde(default)]
    pub order: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrontendToolWindowSpec {
    /// Short id, e.g. `panel`. The host namespaces it with the extension name.
    pub id: String,
    pub title: String,
    /// One of `left-top`, `left-bottom`, `right-top`, `right-bottom`.
    pub position: String,
    /// Optional icon asset path, relative to the extension root.
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub order: Option<u32>,
    #[serde(default)]
    pub hotkey: Option<String>,
    #[serde(default)]
    pub default_open: bool,
}

/// A resolved command sent to the frontend. `event` is fired on the bus when
/// the action/menu item is invoked, so native/WASM code can handle it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrontendCommand {
    pub id: String,
    pub label: String,
    pub category: Option<String>,
    pub keys: Option<String>,
    pub icon: Option<String>,
    pub event: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrontendMenu {
    pub location: String,
    pub command_id: String,
    pub group: Option<String>,
    pub order: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrontendToolWindow {
    pub id: String,
    pub title: String,
    pub position: String,
    pub icon: Option<String>,
    pub order: Option<u32>,
    pub hotkey: Option<String>,
    pub default_open: bool,
}

/// Validated UI contribution for one loaded extension.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrontendContribution {
    pub extension: String,
    /// Path of the iframe entry relative to the extension root.
    pub entry: Option<String>,
    pub commands: Vec<FrontendCommand>,
    pub menus: Vec<FrontendMenu>,
    pub tool_windows: Vec<FrontendToolWindow>,
    pub events: Vec<String>,
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
    #[serde(default)]
    pub frontend: Option<FrontendManifest>,
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
    // Kept as an inherent method for compatibility with existing callers.
    #[allow(clippy::should_implement_trait)]
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

        self.validate_frontend()?;
        Ok(())
    }

    fn validate_frontend(&self) -> Result<()> {
        let Some(frontend) = &self.frontend else {
            return Ok(());
        };
        if let Some(entry) = &frontend.entry {
            validate_relative_path(entry, "frontend entry")?;
        }

        let mut command_ids = std::collections::HashSet::new();
        for command in &frontend.contributions.commands {
            if !is_valid_name(&command.id) {
                bail!(
                    "Invalid frontend command id '{}' in extension '{}'",
                    command.id,
                    self.name
                );
            }
            if command.label.trim().is_empty() {
                bail!(
                    "Frontend command '{}' has an empty label",
                    command.id
                );
            }
            command_ids.insert(command.id.clone());
        }

        for menu in &frontend.contributions.menus {
            if menu.location.trim().is_empty() {
                bail!(
                    "Frontend menu in extension '{}' has an empty location",
                    self.name
                );
            }
            if !command_ids.contains(&menu.command_id) {
                bail!(
                    "Frontend menu '{}' references unknown command '{}'",
                    menu.location,
                    menu.command_id
                );
            }
        }

        for tool in &frontend.contributions.tool_windows {
            if !is_valid_name(&tool.id) {
                bail!(
                    "Invalid tool window id '{}' in extension '{}'",
                    tool.id,
                    self.name
                );
            }
            if tool.title.trim().is_empty() {
                bail!("Tool window '{}' has an empty title", tool.id);
            }
            if !matches!(
                tool.position.as_str(),
                "left-top" | "left-bottom" | "right-top" | "right-bottom"
            ) {
                bail!(
                    "Tool window '{}' has invalid position '{}'",
                    tool.id,
                    tool.position
                );
            }
            if let Some(icon) = &tool.icon {
                validate_relative_path(Path::new(&icon.replace('\\', "/")), "tool window icon")?;
            }
        }

        for event in &frontend.contributions.events {
            if event.trim().is_empty() {
                bail!(
                    "Extension '{}' declared an empty frontend event name",
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

    /// Resolve and validate a frontend asset path inside this extension.
    pub fn asset_path(&self, relative: &str) -> Result<PathBuf> {
        let normalized = relative.replace('\\', "/");
        let path = Path::new(&normalized);
        validate_relative_path(path, "extension asset")?;
        let root = self
            .root
            .canonicalize()
            .with_context(|| format!("Failed to resolve extension root {}", self.root.display()))?;
        let canonical = path_canonical(&root, path)?;
        if !canonical.starts_with(&root) {
            bail!(
                "Extension asset '{relative}' points outside the extension root"
            );
        }
        Ok(canonical)
    }

    /// Build the frontend contribution for this extension, if it declares one.
    ///
    /// The manifest has already been validated; this only resolves asset
    /// paths and namespaces ids so extensions cannot collide with each other.
    pub fn frontend_contribution(&self) -> Option<FrontendContribution> {
        let frontend = self.manifest.frontend.as_ref()?;
        let entry = frontend
            .entry
            .as_ref()
            .and_then(|path| path.to_str())
            .filter(|path| self.asset_path(path).is_ok())
            .map(str::to_owned);

        let mut events = frontend.contributions.events.clone();
        let commands = frontend
            .contributions
            .commands
            .iter()
            .map(|command| {
                let id = format!("{}.{}", self.manifest.name, command.id);
                let event = format!("{EXTENSION_UI_COMMAND_EVENT_PREFIX}{id}");
                if !events.contains(&event) {
                    events.push(event.clone());
                }
                FrontendCommand {
                    id,
                    label: command.label.clone(),
                    category: command.category.clone(),
                    keys: command.keys.clone(),
                    icon: command.icon.clone(),
                    event,
                }
            })
            .collect();

        let menus = frontend
            .contributions
            .menus
            .iter()
            .map(|menu| FrontendMenu {
                location: menu.location.clone(),
                command_id: format!("{}.{}", self.manifest.name, menu.command_id),
                group: menu.group.clone(),
                order: menu.order,
            })
            .collect();

        let tool_windows = frontend
            .contributions
            .tool_windows
            .iter()
            .map(|tool| FrontendToolWindow {
                id: format!("{}.{}", self.manifest.name, tool.id),
                title: tool.title.clone(),
                position: tool.position.clone(),
                icon: tool.icon.clone(),
                order: tool.order,
                hotkey: tool.hotkey.clone(),
                default_open: tool.default_open,
            })
            .collect();

        Some(FrontendContribution {
            extension: self.manifest.name.clone(),
            entry,
            commands,
            menus,
            tool_windows,
            events,
        })
    }
}

fn path_canonical(root: &Path, relative: &Path) -> Result<PathBuf> {
    root.join(relative)
        .canonicalize()
        .with_context(|| format!("Extension asset '{}' does not exist", relative.display()))
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

    #[test]
    fn parses_and_resolves_frontend_contributions() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("frontend")).unwrap();
        std::fs::write(dir.path().join("frontend/index.html"), "<html></html>").unwrap();
        std::fs::write(dir.path().join("frontend/icon.svg"), "<svg></svg>").unwrap();
        let manifest_path = dir.path().join(MANIFEST_FILE_NAME);
        std::fs::write(
            &manifest_path,
            r#"{
                "name": "my-ext",
                "version": "1.0.0",
                "type": "wasm",
                "entry": "my-ext.wasm",
                "frontend": {
                    "entry": "frontend/index.html",
                    "contributions": {
                        "commands": [{ "id": "openPanel", "label": "Open Panel", "keys": "Alt+P" }],
                        "menus": [{ "location": "titlebar/main", "commandId": "openPanel" }],
                        "toolWindows": [{
                            "id": "panel",
                            "title": "My Panel",
                            "position": "right-bottom",
                            "icon": "frontend/icon.svg"
                        }],
                        "events": ["luma:file-opened"]
                    }
                }
            }"#,
        )
        .unwrap();

        let descriptor = ExtensionDescriptor::from_manifest_path(&manifest_path).unwrap();
        let contribution = descriptor.frontend_contribution().unwrap();
        assert_eq!(contribution.extension, "my-ext");
        assert_eq!(contribution.entry.as_deref(), Some("frontend/index.html"));
        assert_eq!(contribution.commands[0].id, "my-ext.openPanel");
        assert_eq!(
            contribution.commands[0].event,
            "luma:extension-command:my-ext.openPanel"
        );
        assert_eq!(contribution.menus[0].command_id, "my-ext.openPanel");
        assert_eq!(contribution.tool_windows[0].id, "my-ext.panel");
        assert!(contribution.events.contains(&contribution.commands[0].event));
        assert_eq!(
            descriptor.asset_path("frontend/index.html").unwrap(),
            dir.path().join("frontend/index.html").canonicalize().unwrap()
        );
    }

    #[test]
    fn rejects_invalid_frontend_contributions() {
        let base = r#"{"name":"a","version":"1.0.0","type":"wasm","entry":"a.wasm","frontend":{"entry":"frontend/index.html","contributions":"#;
        assert!(manifest(&(base.to_owned() + r#"{"commands":[{"id":"../bad","label":"Bad"}]}}}"#)).is_err());
        assert!(manifest(&(base.to_owned() + r#"{"menus":[{"location":"titlebar/main","commandId":"missing"}]}}}"#)).is_err());
        assert!(manifest(&(base.to_owned() + r#"{"toolWindows":[{"id":"p","title":"Panel","position":"top"}]}}}"#)).is_err());
        assert!(manifest(&(base.to_owned() + r#"{"events":[""]}}}"#)).is_err());
    }
}
