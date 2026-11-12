use std::{
    collections::HashMap,
    fs,
    path::Path,
};

use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};

// ============================================================
// Configuration Values
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ConfigurationValue {
    String(String),
    Boolean(bool),
    Float(f64),
    SubConfig(ConfigurationRegistry),
}

impl ConfigurationValue {
    fn has_same_type(&self, other: &Self) -> bool {
        matches!(
            (self, other),
            (Self::String(_), Self::String(_))
                | (Self::Boolean(_), Self::Boolean(_))
                | (Self::Float(_), Self::Float(_))
                | (Self::SubConfig(_), Self::SubConfig(_))
        )
    }
}

// ============================================================
// Configuration Registry
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigurationRegistry {
    // Flatten makes registry entries serialize as ordinary TOML keys.
    #[serde(default, flatten)]
    values: HashMap<String, ConfigurationValue>,

    // Runtime-only state. Never persisted.
    #[serde(skip)]
    frozen: bool,
}

impl Default for ConfigurationRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigurationRegistry {
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
            frozen: false,
        }
    }

    pub fn freeze(&mut self) {
        self.frozen = true;

        for value in self.values.values_mut() {
            if let ConfigurationValue::SubConfig(registry) = value {
                registry.freeze();
            }
        }
    }

    pub fn is_frozen(&self) -> bool {
        self.frozen
    }

    fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    fn insert_path(
        &mut self,
        path: &[String],
        value: ConfigurationValue,
    ) -> Result<()> {
        if self.frozen {
            bail!("Configuration registry is frozen");
        }

        let (key, remaining) = path
            .split_first()
            .ok_or_else(|| anyhow!("Configuration path cannot be empty"))?;

        if remaining.is_empty() {
            self.values.insert(key.clone(), value);
            return Ok(());
        }

        let entry = self.values
            .entry(key.clone())
            .or_insert_with(|| {
                ConfigurationValue::SubConfig(
                    ConfigurationRegistry::new()
                )
            });

        match entry {
            ConfigurationValue::SubConfig(registry) => {
                registry.insert_path(remaining, value)
            }

            _ => bail!(
                "'{}' is not a sub-configuration",
                key
            ),
        }
    }

    fn get_path(
        &self,
        path: &[String],
    ) -> Option<&ConfigurationValue> {
        let (key, remaining) = path.split_first()?;

        let value = self.values.get(key)?;

        if remaining.is_empty() {
            return Some(value);
        }

        match value {
            ConfigurationValue::SubConfig(registry) => {
                registry.get_path(remaining)
            }

            _ => None,
        }
    }

    fn remove_path(
        &mut self,
        path: &[String],
    ) -> Result<bool> {
        let (key, remaining) = path
            .split_first()
            .ok_or_else(|| anyhow!("Configuration path cannot be empty"))?;

        if remaining.is_empty() {
            return Ok(self.values.remove(key).is_some());
        }

        let mut remove_parent = false;

        let removed = match self.values.get_mut(key) {
            Some(ConfigurationValue::SubConfig(registry)) => {
                let removed = registry.remove_path(remaining)?;
                remove_parent = registry.is_empty();
                removed
            }

            _ => false,
        };

        if remove_parent {
            self.values.remove(key);
        }

        Ok(removed)
    }
}

// ============================================================
// Configuration Identifiers
// ============================================================

#[derive(Debug, Clone)]
pub struct ConfigurationId {
    pub owner: String,
    pub path: Vec<String>,
}

impl ConfigurationId {
    pub fn parse(identifier: &str) -> Result<Self> {
        let (owner, path) = identifier
            .split_once(':')
            .ok_or_else(|| anyhow!("Missing configuration owner"))?;

        if owner.is_empty() || path.is_empty() {
            bail!("Invalid configuration identifier");
        }

        if owner.contains('/') || owner.contains(':') {
            bail!("Invalid configuration owner");
        }

        let path: Vec<String> = path
            .split('/')
            .map(String::from)
            .collect();

        if path.iter().any(|segment| {
            segment.is_empty()
                || segment.contains(':')
                || segment.contains('/')
        }) {
            bail!("Invalid configuration path");
        }

        Ok(Self {
            owner: owner.to_string(),
            path,
        })
    }
}

// ============================================================
// Configuration Scope
// ============================================================

#[derive(Debug, Clone, Copy)]
pub enum ConfigurationScope<'a> {
    Global,
    Project(&'a str),
}

// ============================================================
// Configuration Manager
// ============================================================

pub struct ConfigurationManager {
    // Registered defaults, organized by owner.
    defaults: HashMap<String, ConfigurationRegistry>,

    // User-wide configuration overrides.
    global_overrides: HashMap<String, ConfigurationRegistry>,

    // Project-specific overrides.
    // The key is the project's stable identifier.
    project_overrides:
        HashMap<String, HashMap<String, ConfigurationRegistry>>,

    // Prevents registration after initialization.
    frozen: bool,
}

impl Default for ConfigurationManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigurationManager {
    pub fn new() -> Self {
        Self {
            defaults: HashMap::new(),
            global_overrides: HashMap::new(),
            project_overrides: HashMap::new(),
            frozen: false,
        }
    }

    // --------------------------------------------------------
    // Registration
    // --------------------------------------------------------

    pub fn register_owner(&mut self, owner: &str) -> Result<()> {
        if self.frozen {
            bail!("Configuration manager is frozen");
        }

        if owner.is_empty()
            || owner.contains(':')
            || owner.contains('/')
        {
            bail!("Invalid configuration owner");
        }

        self.defaults
            .entry(owner.to_string())
            .or_default();

        Ok(())
    }

    pub fn register_default(
        &mut self,
        identifier: &str,
        value: ConfigurationValue,
    ) -> Result<()> {
        if self.frozen {
            bail!("Configuration manager is frozen");
        }

        let id = ConfigurationId::parse(identifier)?;

        let registry = self.defaults
            .get_mut(&id.owner)
            .ok_or_else(|| {
                anyhow!("Unknown configuration owner: {}", id.owner)
            })?;

        registry.insert_path(&id.path, value)
            .with_context(|| {
                format!("Failed to register '{}'", identifier)
            })
    }

    pub fn freeze(&mut self) {
        for registry in self.defaults.values_mut() {
            registry.freeze();
        }

        self.frozen = true;
    }

    pub fn is_frozen(&self) -> bool {
        self.frozen
    }

    // --------------------------------------------------------
    // Retrieval
    // --------------------------------------------------------

    pub fn get(
        &self,
        identifier: &str,
        project_id: Option<&str>,
    ) -> Result<ConfigurationValue> {
        let id = ConfigurationId::parse(identifier)?;

        // 1. Project override
        if let Some(project_id) = project_id {
            if let Some(project) =
                self.project_overrides.get(project_id)
            {
                if let Some(registry) = project.get(&id.owner) {
                    if let Some(value) = registry.get_path(&id.path) {
                        return Ok(value.clone());
                    }
                }
            }
        }

        // 2. Global override
        if let Some(registry) =
            self.global_overrides.get(&id.owner)
        {
            if let Some(value) = registry.get_path(&id.path) {
                return Ok(value.clone());
            }
        }

        // 3. Registered default
        let registry = self.defaults
            .get(&id.owner)
            .ok_or_else(|| {
                anyhow!("Unknown configuration owner: {}", id.owner)
            })?;

        registry.get_path(&id.path)
            .cloned()
            .ok_or_else(|| {
                anyhow!("Unknown configuration: {}", identifier)
            })
    }

    // --------------------------------------------------------
    // Modification
    // --------------------------------------------------------

    pub fn set(
        &mut self,
        identifier: &str,
        value: ConfigurationValue,
        scope: ConfigurationScope<'_>,
    ) -> Result<()> {
        let id = ConfigurationId::parse(identifier)?;

        // Ensure the configuration exists and has the correct type.
        let default_registry = self.defaults
            .get(&id.owner)
            .ok_or_else(|| {
                anyhow!("Unknown configuration owner: {}", id.owner)
            })?;

        let default_value = default_registry
            .get_path(&id.path)
            .ok_or_else(|| {
                anyhow!("Unknown configuration: {}", identifier)
            })?;

        if !default_value.has_same_type(&value) {
            bail!(
                "Incorrect value type for configuration '{}'",
                identifier
            );
        }

        match scope {
            ConfigurationScope::Global => {
                let registry = self.global_overrides
                    .entry(id.owner.clone())
                    .or_default();

                registry.insert_path(&id.path, value)?;
            }

            ConfigurationScope::Project(project_id) => {
                let project = self.project_overrides
                    .entry(project_id.to_string())
                    .or_default();

                let registry = project
                    .entry(id.owner.clone())
                    .or_default();

                registry.insert_path(&id.path, value)?;
            }
        }

        Ok(())
    }

    pub fn reset(
        &mut self,
        identifier: &str,
        scope: ConfigurationScope<'_>,
    ) -> Result<()> {
        let id = ConfigurationId::parse(identifier)?;

        let registry = match scope {
            ConfigurationScope::Global => {
                self.global_overrides.get_mut(&id.owner)
            }

            ConfigurationScope::Project(project_id) => {
                self.project_overrides
                    .get_mut(project_id)
                    .and_then(|project| project.get_mut(&id.owner))
            }
        };

        if let Some(registry) = registry {
            registry.remove_path(&id.path)?;
        }

        Ok(())
    }

    // --------------------------------------------------------
    // Global Persistence
    // --------------------------------------------------------

    pub fn load_global(&mut self, path: &Path) -> Result<()> {
        if !path.exists() {
            self.global_overrides.clear();
            return Ok(());
        }

        let contents = fs::read_to_string(path)
            .context("Failed to read global configuration")?;

        let overrides: HashMap<String, ConfigurationRegistry> =
            toml::from_str(&contents)
                .context("Failed to parse global configuration")?;

        Self::validate_overrides(&self.defaults, &overrides)?;

        self.global_overrides = overrides;

        Ok(())
    }

    pub fn save_global(&self, path: &Path) -> Result<()> {
        Self::save_to_file(&self.global_overrides, path)
    }

    // --------------------------------------------------------
    // Project Persistence
    // --------------------------------------------------------

    pub fn load_project(
        &mut self,
        project_id: &str,
        path: &Path,
    ) -> Result<()> {
        if !path.exists() {
            self.project_overrides.remove(project_id);
            return Ok(());
        }

        let contents = fs::read_to_string(path)
            .context("Failed to read project configuration")?;

        let overrides: HashMap<String, ConfigurationRegistry> =
            toml::from_str(&contents)
                .context("Failed to parse project configuration")?;

        Self::validate_overrides(&self.defaults, &overrides)?;

        self.project_overrides
            .insert(project_id.to_string(), overrides);

        Ok(())
    }

    pub fn save_project(
        &self,
        project_id: &str,
        path: &Path,
    ) -> Result<()> {
        let empty = HashMap::new();

        let overrides = self.project_overrides
            .get(project_id)
            .unwrap_or(&empty);

        Self::save_to_file(overrides, path)
    }

    // --------------------------------------------------------
    // Persistence Helpers
    // --------------------------------------------------------

    fn save_to_file(
        configuration: &HashMap<String, ConfigurationRegistry>,
        path: &Path,
    ) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .context("Failed to create configuration directory")?;
        }

        let contents = toml::to_string_pretty(configuration)
            .context("Failed to serialize configuration")?;

        fs::write(path, contents)
            .context("Failed to write configuration file")?;

        Ok(())
    }

    // --------------------------------------------------------
    // Validation
    // --------------------------------------------------------

    fn validate_overrides(
        defaults: &HashMap<String, ConfigurationRegistry>,
        overrides: &HashMap<String, ConfigurationRegistry>,
    ) -> Result<()> {
        for (owner, override_registry) in overrides {
            let default_registry = defaults
                .get(owner)
                .ok_or_else(|| {
                    anyhow!("Unknown configuration owner: {}", owner)
                })?;

            Self::validate_registry(
                default_registry,
                override_registry,
                owner,
            )?;
        }

        Ok(())
    }

    fn validate_registry(
        defaults: &ConfigurationRegistry,
        overrides: &ConfigurationRegistry,
        path: &str,
    ) -> Result<()> {
        for (key, override_value) in &overrides.values {
            let default_value = defaults.values
                .get(key)
                .ok_or_else(|| {
                    anyhow!(
                        "Unknown configuration: {}",
                        format!("{}/{}", path, key)
                    )
                })?;

            if !default_value.has_same_type(override_value) {
                bail!(
                    "Incorrect value type for configuration: {}/{}",
                    path,
                    key
                );
            }

            if let (
                ConfigurationValue::SubConfig(default_sub),
                ConfigurationValue::SubConfig(override_sub),
            ) = (default_value, override_value) {
                Self::validate_registry(
                    default_sub,
                    override_sub,
                    &format!("{}/{}", path, key),
                )?;
            }
        }

        Ok(())
    }
}
