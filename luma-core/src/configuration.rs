use std::{
    collections::HashMap,
    fs,
    path::Path,
    sync::Arc,
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

    // Integer values.
    Int(i64),
    Integer(i64),
    I64(i64),
    Int64(i64),

    // Floating point values (64-bit).
    Float64(f64),
    Float(f64),
    Double(f64),
    F64(f64),

    // Boolean values (kept for backwards compatibility).
    Boolean(bool),
    Bool(bool),

    // Nested configuration (a table of values).
    SubConfig(ConfigurationRegistry),
    SubConfiguration(ConfigurationRegistry),
    SubConfigs(ConfigurationRegistry),
    Configuration(ConfigurationRegistry),
    Config(ConfigurationRegistry),

    // Hash maps / tables of values.
    HashMap(HashMap<String, ConfigurationValue>),
    Map(HashMap<String, ConfigurationValue>),
    Dict(HashMap<String, ConfigurationValue>),
    Table(HashMap<String, ConfigurationValue>),
    Object(HashMap<String, ConfigurationValue>),

    // Lists / arrays of values.
    List(Vec<ConfigurationValue>),
    Array(Vec<ConfigurationValue>),
    Vec(Vec<ConfigurationValue>),
    Sequence(Vec<ConfigurationValue>),
}

impl ConfigurationValue {
    pub fn has_same_type(&self, other: &Self) -> bool {
        matches!(
            (self, other),
            (Self::String(_), Self::String(_))
                | (Self::Int(_), Self::Int(_))
                | (Self::Integer(_), Self::Integer(_))
                | (Self::I64(_), Self::I64(_))
                | (Self::Int64(_), Self::Int64(_))
                | (Self::Float64(_), Self::Float64(_))
                | (Self::Float(_), Self::Float(_))
                | (Self::Double(_), Self::Double(_))
                | (Self::F64(_), Self::F64(_))
                | (Self::Boolean(_), Self::Boolean(_))
                | (Self::Bool(_), Self::Bool(_))
                | (Self::SubConfig(_), Self::SubConfig(_))
                | (Self::SubConfiguration(_), Self::SubConfiguration(_))
                | (Self::SubConfigs(_), Self::SubConfigs(_))
                | (Self::Configuration(_), Self::Configuration(_))
                | (Self::Config(_), Self::Config(_))
                | (Self::HashMap(_), Self::HashMap(_))
                | (Self::Map(_), Self::Map(_))
                | (Self::Dict(_), Self::Dict(_))
                | (Self::Table(_), Self::Table(_))
                | (Self::Object(_), Self::Object(_))
                | (Self::List(_), Self::List(_))
                | (Self::Array(_), Self::Array(_))
                | (Self::Vec(_), Self::Vec(_))
                | (Self::Sequence(_), Self::Sequence(_))
                // Allow cross-alias comparisons between equivalent kinds.
                | (Self::Int(_), Self::Integer(_))
                | (Self::Int(_), Self::I64(_))
                | (Self::Int(_), Self::Int64(_))
                | (Self::Integer(_), Self::Int(_))
                | (Self::Integer(_), Self::I64(_))
                | (Self::Integer(_), Self::Int64(_))
                | (Self::I64(_), Self::Int(_))
                | (Self::I64(_), Self::Integer(_))
                | (Self::I64(_), Self::Int64(_))
                | (Self::Int64(_), Self::Int(_))
                | (Self::Int64(_), Self::Integer(_))
                | (Self::Int64(_), Self::I64(_))
                | (Self::Float64(_), Self::Float(_))
                | (Self::Float64(_), Self::Double(_))
                | (Self::Float64(_), Self::F64(_))
                | (Self::Float(_), Self::Float64(_))
                | (Self::Float(_), Self::Double(_))
                | (Self::Float(_), Self::F64(_))
                | (Self::Double(_), Self::Float64(_))
                | (Self::Double(_), Self::Float(_))
                | (Self::Double(_), Self::F64(_))
                | (Self::F64(_), Self::Float64(_))
                | (Self::F64(_), Self::Float(_))
                | (Self::F64(_), Self::Double(_))
                | (Self::Boolean(_), Self::Bool(_))
                | (Self::Bool(_), Self::Boolean(_))
                | (Self::SubConfig(_), Self::SubConfiguration(_))
                | (Self::SubConfig(_), Self::SubConfigs(_))
                | (Self::SubConfig(_), Self::Configuration(_))
                | (Self::SubConfig(_), Self::Config(_))
                | (Self::SubConfiguration(_), Self::SubConfig(_))
                | (Self::SubConfiguration(_), Self::SubConfigs(_))
                | (Self::SubConfiguration(_), Self::Configuration(_))
                | (Self::SubConfiguration(_), Self::Config(_))
                | (Self::SubConfigs(_), Self::SubConfig(_))
                | (Self::SubConfigs(_), Self::SubConfiguration(_))
                | (Self::SubConfigs(_), Self::Configuration(_))
                | (Self::SubConfigs(_), Self::Config(_))
                | (Self::Configuration(_), Self::SubConfig(_))
                | (Self::Configuration(_), Self::SubConfiguration(_))
                | (Self::Configuration(_), Self::SubConfigs(_))
                | (Self::Configuration(_), Self::Config(_))
                | (Self::Config(_), Self::SubConfig(_))
                | (Self::Config(_), Self::SubConfiguration(_))
                | (Self::Config(_), Self::SubConfigs(_))
                | (Self::Config(_), Self::Configuration(_))
                | (Self::HashMap(_), Self::Map(_))
                | (Self::HashMap(_), Self::Dict(_))
                | (Self::HashMap(_), Self::Table(_))
                | (Self::HashMap(_), Self::Object(_))
                | (Self::Map(_), Self::HashMap(_))
                | (Self::Map(_), Self::Dict(_))
                | (Self::Map(_), Self::Table(_))
                | (Self::Map(_), Self::Object(_))
                | (Self::Dict(_), Self::HashMap(_))
                | (Self::Dict(_), Self::Map(_))
                | (Self::Dict(_), Self::Table(_))
                | (Self::Dict(_), Self::Object(_))
                | (Self::Table(_), Self::HashMap(_))
                | (Self::Table(_), Self::Map(_))
                | (Self::Table(_), Self::Dict(_))
                | (Self::Table(_), Self::Object(_))
                | (Self::Object(_), Self::HashMap(_))
                | (Self::Object(_), Self::Map(_))
                | (Self::Object(_), Self::Dict(_))
                | (Self::Object(_), Self::Table(_))
                | (Self::List(_), Self::Array(_))
                | (Self::List(_), Self::Vec(_))
                | (Self::List(_), Self::Sequence(_))
                | (Self::Array(_), Self::List(_))
                | (Self::Array(_), Self::Vec(_))
                | (Self::Array(_), Self::Sequence(_))
                | (Self::Vec(_), Self::List(_))
                | (Self::Vec(_), Self::Array(_))
                | (Self::Vec(_), Self::Sequence(_))
                | (Self::Sequence(_), Self::List(_))
                | (Self::Sequence(_), Self::Array(_))
                | (Self::Sequence(_), Self::Vec(_))
        )
    }
}

impl PartialEq for ConfigurationValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::String(a), Self::String(b)) => a == b,
            (Self::Int(a), Self::Int(b))
            | (Self::Int(a), Self::Integer(b))
            | (Self::Int(a), Self::I64(b))
            | (Self::Int(a), Self::Int64(b))
            | (Self::Integer(a), Self::Int(b))
            | (Self::Integer(a), Self::Integer(b))
            | (Self::Integer(a), Self::I64(b))
            | (Self::Integer(a), Self::Int64(b))
            | (Self::I64(a), Self::Int(b))
            | (Self::I64(a), Self::Integer(b))
            | (Self::I64(a), Self::I64(b))
            | (Self::I64(a), Self::Int64(b))
            | (Self::Int64(a), Self::Int(b))
            | (Self::Int64(a), Self::Integer(b))
            | (Self::Int64(a), Self::I64(b))
            | (Self::Int64(a), Self::Int64(b)) => a == b,
            (Self::Float64(a), Self::Float64(b))
            | (Self::Float64(a), Self::Float(b))
            | (Self::Float64(a), Self::Double(b))
            | (Self::Float64(a), Self::F64(b))
            | (Self::Float(a), Self::Float64(b))
            | (Self::Float(a), Self::Float(b))
            | (Self::Float(a), Self::Double(b))
            | (Self::Float(a), Self::F64(b))
            | (Self::Double(a), Self::Float64(b))
            | (Self::Double(a), Self::Float(b))
            | (Self::Double(a), Self::Double(b))
            | (Self::Double(a), Self::F64(b))
            | (Self::F64(a), Self::Float64(b))
            | (Self::F64(a), Self::Float(b))
            | (Self::F64(a), Self::Double(b))
            | (Self::F64(a), Self::F64(b)) => a == b,
            (Self::Boolean(a), Self::Boolean(b))
            | (Self::Boolean(a), Self::Bool(b))
            | (Self::Bool(a), Self::Boolean(b))
            | (Self::Bool(a), Self::Bool(b)) => a == b,
            (Self::SubConfig(a), Self::SubConfig(b))
            | (Self::SubConfig(a), Self::SubConfiguration(b))
            | (Self::SubConfig(a), Self::SubConfigs(b))
            | (Self::SubConfig(a), Self::Configuration(b))
            | (Self::SubConfig(a), Self::Config(b))
            | (Self::SubConfiguration(a), Self::SubConfig(b))
            | (Self::SubConfiguration(a), Self::SubConfiguration(b))
            | (Self::SubConfiguration(a), Self::SubConfigs(b))
            | (Self::SubConfiguration(a), Self::Configuration(b))
            | (Self::SubConfiguration(a), Self::Config(b))
            | (Self::SubConfigs(a), Self::SubConfig(b))
            | (Self::SubConfigs(a), Self::SubConfiguration(b))
            | (Self::SubConfigs(a), Self::SubConfigs(b))
            | (Self::SubConfigs(a), Self::Configuration(b))
            | (Self::SubConfigs(a), Self::Config(b))
            | (Self::Configuration(a), Self::SubConfig(b))
            | (Self::Configuration(a), Self::SubConfiguration(b))
            | (Self::Configuration(a), Self::SubConfigs(b))
            | (Self::Configuration(a), Self::Configuration(b))
            | (Self::Configuration(a), Self::Config(b))
            | (Self::Config(a), Self::SubConfig(b))
            | (Self::Config(a), Self::SubConfiguration(b))
            | (Self::Config(a), Self::SubConfigs(b))
            | (Self::Config(a), Self::Configuration(b))
            | (Self::Config(a), Self::Config(b)) => a == b,
            // Table-like variants are interchangeable.
            (Self::HashMap(a), Self::HashMap(b))
            | (Self::HashMap(a), Self::Map(b))
            | (Self::HashMap(a), Self::Dict(b))
            | (Self::HashMap(a), Self::Table(b))
            | (Self::HashMap(a), Self::Object(b))
            | (Self::Map(a), Self::HashMap(b))
            | (Self::Map(a), Self::Map(b))
            | (Self::Map(a), Self::Dict(b))
            | (Self::Map(a), Self::Table(b))
            | (Self::Map(a), Self::Object(b))
            | (Self::Dict(a), Self::HashMap(b))
            | (Self::Dict(a), Self::Map(b))
            | (Self::Dict(a), Self::Dict(b))
            | (Self::Dict(a), Self::Table(b))
            | (Self::Dict(a), Self::Object(b))
            | (Self::Table(a), Self::HashMap(b))
            | (Self::Table(a), Self::Map(b))
            | (Self::Table(a), Self::Dict(b))
            | (Self::Table(a), Self::Table(b))
            | (Self::Table(a), Self::Object(b))
            | (Self::Object(a), Self::HashMap(b))
            | (Self::Object(a), Self::Map(b))
            | (Self::Object(a), Self::Dict(b))
            | (Self::Object(a), Self::Table(b))
            | (Self::Object(a), Self::Object(b)) => {
                // Compare as maps of ConfigurationValue.
                if a.len() != b.len() { return false; }
                a.iter().all(|(k,v)| b.get(k).map(|ov| v == ov).unwrap_or(false))
            }
            // Cross table/map/subconfig equality.
            (Self::SubConfig(a), Self::HashMap(b))
            | (Self::SubConfig(a), Self::Map(b))
            | (Self::SubConfig(a), Self::Dict(b))
            | (Self::SubConfig(a), Self::Table(b))
            | (Self::SubConfig(a), Self::Object(b))
            | (Self::SubConfiguration(a), Self::HashMap(b))
            | (Self::SubConfiguration(a), Self::Map(b))
            | (Self::SubConfiguration(a), Self::Dict(b))
            | (Self::SubConfiguration(a), Self::Table(b))
            | (Self::SubConfiguration(a), Self::Object(b))
            | (Self::SubConfigs(a), Self::HashMap(b))
            | (Self::SubConfigs(a), Self::Map(b))
            | (Self::SubConfigs(a), Self::Dict(b))
            | (Self::SubConfigs(a), Self::Table(b))
            | (Self::SubConfigs(a), Self::Object(b))
            | (Self::Configuration(a), Self::HashMap(b))
            | (Self::Configuration(a), Self::Map(b))
            | (Self::Configuration(a), Self::Dict(b))
            | (Self::Configuration(a), Self::Table(b))
            | (Self::Configuration(a), Self::Object(b))
            | (Self::Config(a), Self::HashMap(b))
            | (Self::Config(a), Self::Map(b))
            | (Self::Config(a), Self::Dict(b))
            | (Self::Config(a), Self::Table(b))
            | (Self::Config(a), Self::Object(b)) => {
                let am = &a.values;
                if am.len() != b.len() { return false; }
                b.iter().all(|(k,v)| am.get(k).map(|ov| ov == v).unwrap_or(false))
            }
            (Self::HashMap(a), Self::SubConfig(b))
            | (Self::HashMap(a), Self::SubConfiguration(b))
            | (Self::HashMap(a), Self::SubConfigs(b))
            | (Self::HashMap(a), Self::Configuration(b))
            | (Self::HashMap(a), Self::Config(b))
            | (Self::Map(a), Self::SubConfig(b))
            | (Self::Map(a), Self::SubConfiguration(b))
            | (Self::Map(a), Self::SubConfigs(b))
            | (Self::Map(a), Self::Configuration(b))
            | (Self::Map(a), Self::Config(b))
            | (Self::Dict(a), Self::SubConfig(b))
            | (Self::Dict(a), Self::SubConfiguration(b))
            | (Self::Dict(a), Self::SubConfigs(b))
            | (Self::Dict(a), Self::Configuration(b))
            | (Self::Dict(a), Self::Config(b))
            | (Self::Table(a), Self::SubConfig(b))
            | (Self::Table(a), Self::SubConfiguration(b))
            | (Self::Table(a), Self::SubConfigs(b))
            | (Self::Table(a), Self::Configuration(b))
            | (Self::Table(a), Self::Config(b))
            | (Self::Object(a), Self::SubConfig(b))
            | (Self::Object(a), Self::SubConfiguration(b))
            | (Self::Object(a), Self::SubConfigs(b))
            | (Self::Object(a), Self::Configuration(b))
            | (Self::Object(a), Self::Config(b)) => {
                let bm = &b.values;
                if a.len() != bm.len() { return false; }
                a.iter().all(|(k,v)| bm.get(k).map(|ov| v == ov).unwrap_or(false))
            }
            (Self::List(a), Self::List(b))
            | (Self::List(a), Self::Array(b))
            | (Self::List(a), Self::Vec(b))
            | (Self::List(a), Self::Sequence(b))
            | (Self::Array(a), Self::List(b))
            | (Self::Array(a), Self::Array(b))
            | (Self::Array(a), Self::Vec(b))
            | (Self::Array(a), Self::Sequence(b))
            | (Self::Vec(a), Self::List(b))
            | (Self::Vec(a), Self::Array(b))
            | (Self::Vec(a), Self::Vec(b))
            | (Self::Vec(a), Self::Sequence(b))
            | (Self::Sequence(a), Self::List(b))
            | (Self::Sequence(a), Self::Array(b))
            | (Self::Sequence(a), Self::Vec(b))
            | (Self::Sequence(a), Self::Sequence(b)) => {
                if a.len() != b.len() { return false; }
                a.iter().zip(b.iter()).all(|(x,y)| x == y)
            }
            _ => false,
        }
    }
}

impl PartialEq for ConfigurationRegistry {
    fn eq(&self, other: &Self) -> bool {
        self.values == other.values
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
            match value {
                ConfigurationValue::SubConfig(registry)
                | ConfigurationValue::SubConfiguration(registry)
                | ConfigurationValue::SubConfigs(registry)
                | ConfigurationValue::Configuration(registry)
                | ConfigurationValue::Config(registry) => {
                    registry.freeze();
                }
                _ => {}
            }
        }
    }

    pub fn is_frozen(&self) -> bool {
        self.frozen
    }

    fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub fn insert_path(
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
            ConfigurationValue::SubConfig(registry)
            | ConfigurationValue::SubConfiguration(registry)
            | ConfigurationValue::SubConfigs(registry)
            | ConfigurationValue::Configuration(registry)
            | ConfigurationValue::Config(registry) => {
                registry.insert_path(remaining, value)
            }

            _ => bail!(
                "'{}' is not a sub-configuration",
                key
            ),
        }
    }

    pub fn get_path(
        &self,
        path: &[String],
    ) -> Option<&ConfigurationValue> {
        let (key, remaining) = path.split_first()?;

        let value = self.values.get(key)?;

        if remaining.is_empty() {
            return Some(value);
        }

        match value {
            ConfigurationValue::SubConfig(registry)
            | ConfigurationValue::SubConfiguration(registry)
            | ConfigurationValue::SubConfigs(registry)
            | ConfigurationValue::Configuration(registry)
            | ConfigurationValue::Config(registry) => {
                registry.get_path(remaining)
            }

            _ => None,
        }
    }

    pub fn remove_path(
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
            Some(ConfigurationValue::SubConfig(registry))
            | Some(ConfigurationValue::SubConfiguration(registry))
            | Some(ConfigurationValue::SubConfigs(registry))
            | Some(ConfigurationValue::Configuration(registry))
            | Some(ConfigurationValue::Config(registry)) => {
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

    pub fn values(&self) -> &HashMap<String, ConfigurationValue> {
        &self.values
    }

    pub fn values_mut(&mut self) -> &mut HashMap<String, ConfigurationValue> {
        &mut self.values
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
// Callbacks
// ============================================================

pub type ConfigurationChangeCallback =
    Arc<dyn Fn(&str, ConfigurationValue) + Send + Sync>;

pub type ConfigurationCallback = ConfigurationChangeCallback;

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

    // Change listeners.
    callbacks: Vec<ConfigurationChangeCallback>,
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
            callbacks: Vec::new(),
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
    // Change callbacks
    // --------------------------------------------------------

    pub fn on_change<F>(&mut self, callback: F)
    where
        F: Fn(&str, ConfigurationValue) + Send + Sync + 'static,
    {
        self.callbacks.push(Arc::new(callback));
    }

    pub fn set_on_change<F>(&mut self, callback: F)
    where
        F: Fn(&str, ConfigurationValue) + Send + Sync + 'static,
    {
        self.on_change(callback);
    }

    pub fn set_change_callback<F>(&mut self, callback: F)
    where
        F: Fn(&str, ConfigurationValue) + Send + Sync + 'static,
    {
        self.on_change(callback);
    }

    pub fn register_change_callback<F>(&mut self, callback: F)
    where
        F: Fn(&str, ConfigurationValue) + Send + Sync + 'static,
    {
        self.on_change(callback);
    }

    pub fn add_change_listener<F>(&mut self, callback: F)
    where
        F: Fn(&str, ConfigurationValue) + Send + Sync + 'static,
    {
        self.on_change(callback);
    }

    pub fn register_listener<F>(&mut self, callback: F)
    where
        F: Fn(&str, ConfigurationValue) + Send + Sync + 'static,
    {
        self.on_change(callback);
    }

    pub fn on_update<F>(&mut self, callback: F)
    where
        F: Fn(&str, ConfigurationValue) + Send + Sync + 'static,
    {
        self.on_change(callback);
    }

    pub fn set_update_callback<F>(&mut self, callback: F)
    where
        F: Fn(&str, ConfigurationValue) + Send + Sync + 'static,
    {
        self.on_change(callback);
    }

    pub fn on_configuration_changed<F>(&mut self, callback: F)
    where
        F: Fn(&str, ConfigurationValue) + Send + Sync + 'static,
    {
        self.on_change(callback);
    }

    fn notify_change(&self, identifier: &str, value: ConfigurationValue) {
        for callback in &self.callbacks {
            callback(identifier, value.clone());
        }
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

                registry.insert_path(&id.path, value.clone())?;
            }

            ConfigurationScope::Project(project_id) => {
                let project = self.project_overrides
                    .entry(project_id.to_string())
                    .or_default();

                let registry = project
                    .entry(id.owner.clone())
                    .or_default();

                registry.insert_path(&id.path, value.clone())?;
            }
        }

        self.notify_change(identifier, value);

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
            // Also validate alias subconfig variants.
            let default_sub = match default_value {
                ConfigurationValue::SubConfig(r)
                | ConfigurationValue::SubConfiguration(r)
                | ConfigurationValue::SubConfigs(r)
                | ConfigurationValue::Configuration(r)
                | ConfigurationValue::Config(r) => Some(r),
                _ => None,
            };
            let override_sub = match override_value {
                ConfigurationValue::SubConfig(r)
                | ConfigurationValue::SubConfiguration(r)
                | ConfigurationValue::SubConfigs(r)
                | ConfigurationValue::Configuration(r)
                | ConfigurationValue::Config(r) => Some(r),
                _ => None,
            };
            if let (Some(ds), Some(os)) = (default_sub, override_sub) {
                Self::validate_registry(
                    ds,
                    os,
                    &format!("{}/{}", path, key),
                )?;
            }
        }

        Ok(())
    }
}