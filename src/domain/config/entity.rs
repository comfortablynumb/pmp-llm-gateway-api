//! Application configuration domain entities

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::domain::storage::{StorageEntity, StorageKey};

/// Configuration key identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ConfigKey(String);

impl ConfigKey {
    pub fn new(key: impl Into<String>) -> Result<Self, ConfigValidationError> {
        let key = key.into();
        validate_config_key(&key)?;
        Ok(Self(key))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ConfigKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl StorageKey for ConfigKey {
    fn as_str(&self) -> &str {
        &self.0
    }
}

/// Configuration value types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum ConfigValue {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    StringList(Vec<String>),
}

impl ConfigValue {
    pub fn as_string(&self) -> Option<&str> {
        match self {
            ConfigValue::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_integer(&self) -> Option<i64> {
        match self {
            ConfigValue::Integer(i) => Some(*i),
            _ => None,
        }
    }

    pub fn as_float(&self) -> Option<f64> {
        match self {
            ConfigValue::Float(f) => Some(*f),
            _ => None,
        }
    }

    pub fn as_boolean(&self) -> Option<bool> {
        match self {
            ConfigValue::Boolean(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_string_list(&self) -> Option<&[String]> {
        match self {
            ConfigValue::StringList(list) => Some(list),
            _ => None,
        }
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            ConfigValue::String(_) => "string",
            ConfigValue::Integer(_) => "integer",
            ConfigValue::Float(_) => "float",
            ConfigValue::Boolean(_) => "boolean",
            ConfigValue::StringList(_) => "string_list",
        }
    }
}

impl std::fmt::Display for ConfigValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigValue::String(s) => write!(f, "{}", s),
            ConfigValue::Integer(i) => write!(f, "{}", i),
            ConfigValue::Float(fl) => write!(f, "{}", fl),
            ConfigValue::Boolean(b) => write!(f, "{}", b),
            ConfigValue::StringList(list) => write!(f, "{:?}", list),
        }
    }
}

/// Configuration categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigCategory {
    General,
    Persistence,
    Logging,
    Security,
    Cache,
    RateLimit,
}

impl ConfigCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            ConfigCategory::General => "general",
            ConfigCategory::Persistence => "persistence",
            ConfigCategory::Logging => "logging",
            ConfigCategory::Security => "security",
            ConfigCategory::Cache => "cache",
            ConfigCategory::RateLimit => "rate_limit",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "general" => Some(ConfigCategory::General),
            "persistence" => Some(ConfigCategory::Persistence),
            "logging" => Some(ConfigCategory::Logging),
            "security" => Some(ConfigCategory::Security),
            "cache" => Some(ConfigCategory::Cache),
            "rate_limit" => Some(ConfigCategory::RateLimit),
            _ => None,
        }
    }
}

impl std::fmt::Display for ConfigCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Metadata for a configuration entry (stored in metadata JSONB column)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigMetadata {
    pub category: String,
    pub description: String,
    pub value_type: String,
}

impl ConfigMetadata {
    pub fn new(category: ConfigCategory, description: impl Into<String>) -> Self {
        Self {
            category: category.as_str().to_string(),
            description: description.into(),
            value_type: String::new(), // Will be set from value
        }
    }

    pub fn with_value_type(mut self, value_type: &str) -> Self {
        self.value_type = value_type.to_string();
        self
    }

    pub fn category(&self) -> Option<ConfigCategory> {
        ConfigCategory::from_str(&self.category)
    }
}

impl Default for ConfigMetadata {
    fn default() -> Self {
        Self {
            category: "general".to_string(),
            description: String::new(),
            value_type: String::new(),
        }
    }
}

/// A single configuration entry (maps to one row in app_configurations table)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigEntry {
    key: ConfigKey,
    value: ConfigValue,
    metadata: ConfigMetadata,
    #[serde(default = "Utc::now")]
    created_at: DateTime<Utc>,
    #[serde(default = "Utc::now")]
    updated_at: DateTime<Utc>,
}

impl ConfigEntry {
    pub fn new(key: ConfigKey, value: ConfigValue, metadata: ConfigMetadata) -> Self {
        let now = Utc::now();
        Self {
            key,
            value,
            metadata,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn key(&self) -> &ConfigKey {
        &self.key
    }

    pub fn value(&self) -> &ConfigValue {
        &self.value
    }

    pub fn metadata(&self) -> &ConfigMetadata {
        &self.metadata
    }

    pub fn category(&self) -> ConfigCategory {
        self.metadata.category().unwrap_or(ConfigCategory::General)
    }

    pub fn description(&self) -> &str {
        &self.metadata.description
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    pub fn set_value(&mut self, value: ConfigValue) {
        self.value = value;
        self.updated_at = Utc::now();
    }

    pub fn with_timestamps(mut self, created_at: DateTime<Utc>, updated_at: DateTime<Utc>) -> Self {
        self.created_at = created_at;
        self.updated_at = updated_at;
        self
    }
}

impl StorageEntity for ConfigEntry {
    type Key = ConfigKey;

    fn key(&self) -> &Self::Key {
        &self.key
    }
}

/// Application configuration collection (constructed from individual entries)
#[derive(Debug, Clone, Default)]
pub struct AppConfiguration {
    entries: HashMap<String, ConfigEntry>,
}

impl AppConfiguration {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// Create configuration from a list of entries
    pub fn from_entries(entries: Vec<ConfigEntry>) -> Self {
        let mut config = Self::new();

        for entry in entries {
            config
                .entries
                .insert(entry.key().as_str().to_string(), entry);
        }
        config
    }

    pub fn get(&self, key: &str) -> Option<&ConfigEntry> {
        self.entries.get(key)
    }

    pub fn get_value(&self, key: &str) -> Option<&ConfigValue> {
        self.entries.get(key).map(|e| e.value())
    }

    pub fn set(
        &mut self,
        key: ConfigKey,
        value: ConfigValue,
    ) -> Result<(), ConfigValidationError> {
        if let Some(entry) = self.entries.get_mut(key.as_str()) {
            // Validate type matches
            if entry.value().type_name() != value.type_name() {
                return Err(ConfigValidationError::TypeMismatch {
                    key: key.to_string(),
                    expected: entry.value().type_name().to_string(),
                    actual: value.type_name().to_string(),
                });
            }
            entry.set_value(value);
            Ok(())
        } else {
            Err(ConfigValidationError::UnknownKey(key.to_string()))
        }
    }

    pub fn list(&self) -> Vec<&ConfigEntry> {
        let mut entries: Vec<_> = self.entries.values().collect();
        entries.sort_by_key(|e| e.key().as_str());
        entries
    }

    pub fn list_by_category(&self, category: ConfigCategory) -> Vec<&ConfigEntry> {
        let mut entries: Vec<_> = self
            .entries
            .values()
            .filter(|e| e.category() == category)
            .collect();
        entries.sort_by_key(|e| e.key().as_str());
        entries
    }

    pub fn categories(&self) -> Vec<ConfigCategory> {
        vec![
            ConfigCategory::General,
            ConfigCategory::Persistence,
            ConfigCategory::Logging,
            ConfigCategory::Security,
            ConfigCategory::Cache,
            ConfigCategory::RateLimit,
        ]
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    // Convenience getters for persistence settings

    pub fn is_persistence_enabled(&self) -> bool {
        self.get_value("persistence.enabled")
            .and_then(|v| v.as_boolean())
            .unwrap_or(false)
    }

    pub fn enabled_models(&self) -> Vec<String> {
        self.get_value("persistence.enabled_models")
            .and_then(|v| v.as_string_list())
            .map(|s| s.to_vec())
            .unwrap_or_default()
    }

    pub fn enabled_workflows(&self) -> Vec<String> {
        self.get_value("persistence.enabled_workflows")
            .and_then(|v| v.as_string_list())
            .map(|s| s.to_vec())
            .unwrap_or_default()
    }

    pub fn log_retention_days(&self) -> i64 {
        self.get_value("persistence.log_retention_days")
            .and_then(|v| v.as_integer())
            .unwrap_or(30)
    }

    pub fn log_sensitive_data(&self) -> bool {
        self.get_value("persistence.log_sensitive_data")
            .and_then(|v| v.as_boolean())
            .unwrap_or(false)
    }

    pub fn should_log_model(&self, model_id: &str) -> bool {
        if !self.is_persistence_enabled() {
            return false;
        }

        let enabled_models = self.enabled_models();

        if enabled_models.is_empty() {
            return true; // Log all if list is empty
        }

        enabled_models.contains(&model_id.to_string())
    }

    pub fn should_log_workflow(&self, workflow_id: &str) -> bool {
        if !self.is_persistence_enabled() {
            return false;
        }

        let enabled_workflows = self.enabled_workflows();

        if enabled_workflows.is_empty() {
            return true; // Log all if list is empty
        }

        enabled_workflows.contains(&workflow_id.to_string())
    }
}

/// Configuration validation errors
#[derive(Debug, Clone, thiserror::Error)]
pub enum ConfigValidationError {
    #[error("Invalid configuration key: {0}")]
    InvalidKey(String),

    #[error("Unknown configuration key: {0}")]
    UnknownKey(String),

    #[error("Type mismatch for key '{key}': expected {expected}, got {actual}")]
    TypeMismatch {
        key: String,
        expected: String,
        actual: String,
    },

    #[error("Invalid value for key '{key}': {reason}")]
    InvalidValue { key: String, reason: String },
}

fn validate_config_key(key: &str) -> Result<(), ConfigValidationError> {
    if key.is_empty() {
        return Err(ConfigValidationError::InvalidKey(
            "Key cannot be empty".to_string(),
        ));
    }

    if key.len() > 100 {
        return Err(ConfigValidationError::InvalidKey(
            "Key cannot exceed 100 characters".to_string(),
        ));
    }

    // Allow alphanumeric, underscores, and dots (for namespacing)
    if !key
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '.')
    {
        return Err(ConfigValidationError::InvalidKey(
            "Key can only contain alphanumeric characters, underscores, and dots".to_string(),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_key_validation() {
        assert!(ConfigKey::new("valid_key").is_ok());
        assert!(ConfigKey::new("persistence.enabled").is_ok());
        assert!(ConfigKey::new("").is_err());
        assert!(ConfigKey::new("invalid key").is_err());
        assert!(ConfigKey::new("key-with-dash").is_err());
    }

    #[test]
    fn test_config_value_accessors() {
        let string_val = ConfigValue::String("test".to_string());
        assert_eq!(string_val.as_string(), Some("test"));
        assert_eq!(string_val.as_integer(), None);

        let int_val = ConfigValue::Integer(42);
        assert_eq!(int_val.as_integer(), Some(42));
        assert_eq!(int_val.as_string(), None);

        let bool_val = ConfigValue::Boolean(true);
        assert_eq!(bool_val.as_boolean(), Some(true));

        let list_val = ConfigValue::StringList(vec!["a".to_string(), "b".to_string()]);
        assert_eq!(
            list_val.as_string_list(),
            Some(&["a".to_string(), "b".to_string()][..])
        );
    }

    #[test]
    fn test_config_metadata() {
        let metadata = ConfigMetadata::new(ConfigCategory::Persistence, "Test description")
            .with_value_type("boolean");

        assert_eq!(metadata.category(), Some(ConfigCategory::Persistence));
        assert_eq!(metadata.description, "Test description");
        assert_eq!(metadata.value_type, "boolean");
    }

    #[test]
    fn test_config_entry_creation() {
        let key = ConfigKey::new("test.key").unwrap();
        let value = ConfigValue::Boolean(true);
        let metadata = ConfigMetadata::new(ConfigCategory::General, "Test");

        let entry = ConfigEntry::new(key.clone(), value, metadata);

        assert_eq!(entry.key().as_str(), "test.key");
        assert_eq!(entry.value().as_boolean(), Some(true));
        assert_eq!(entry.category(), ConfigCategory::General);
    }

    #[test]
    fn test_app_configuration_from_entries() {
        let entries = vec![
            ConfigEntry::new(
                ConfigKey::new("persistence.enabled").unwrap(),
                ConfigValue::Boolean(true),
                ConfigMetadata::new(ConfigCategory::Persistence, "Enable logging"),
            ),
            ConfigEntry::new(
                ConfigKey::new("persistence.log_retention_days").unwrap(),
                ConfigValue::Integer(30),
                ConfigMetadata::new(ConfigCategory::Persistence, "Retention days"),
            ),
        ];

        let config = AppConfiguration::from_entries(entries);

        assert!(config.is_persistence_enabled());
        assert_eq!(config.log_retention_days(), 30);
    }

    #[test]
    fn test_app_configuration_empty_defaults() {
        let config = AppConfiguration::new();

        // Empty config returns defaults for convenience getters
        assert!(!config.is_persistence_enabled());
        assert_eq!(config.log_retention_days(), 30); // Default fallback
        assert!(!config.log_sensitive_data());
    }

    #[test]
    fn test_app_configuration_set() {
        let entries = vec![ConfigEntry::new(
            ConfigKey::new("persistence.enabled").unwrap(),
            ConfigValue::Boolean(false),
            ConfigMetadata::new(ConfigCategory::Persistence, "Enable logging"),
        )];

        let mut config = AppConfiguration::from_entries(entries);

        let key = ConfigKey::new("persistence.enabled").unwrap();
        assert!(config.set(key, ConfigValue::Boolean(true)).is_ok());
        assert!(config.is_persistence_enabled());
    }

    #[test]
    fn test_app_configuration_type_mismatch() {
        let entries = vec![ConfigEntry::new(
            ConfigKey::new("persistence.enabled").unwrap(),
            ConfigValue::Boolean(false),
            ConfigMetadata::new(ConfigCategory::Persistence, "Enable logging"),
        )];

        let mut config = AppConfiguration::from_entries(entries);

        let key = ConfigKey::new("persistence.enabled").unwrap();
        let result = config.set(key, ConfigValue::String("true".to_string()));
        assert!(matches!(
            result,
            Err(ConfigValidationError::TypeMismatch { .. })
        ));
    }

    #[test]
    fn test_should_log_model() {
        let entries = vec![
            ConfigEntry::new(
                ConfigKey::new("persistence.enabled").unwrap(),
                ConfigValue::Boolean(true),
                ConfigMetadata::new(ConfigCategory::Persistence, "Enable logging"),
            ),
            ConfigEntry::new(
                ConfigKey::new("persistence.enabled_models").unwrap(),
                ConfigValue::StringList(vec![]),
                ConfigMetadata::new(ConfigCategory::Persistence, "Enabled models"),
            ),
        ];

        let config = AppConfiguration::from_entries(entries);

        // Empty list = log all
        assert!(config.should_log_model("gpt-4"));
        assert!(config.should_log_model("claude-3"));
    }

    #[test]
    fn test_should_log_model_specific_list() {
        let entries = vec![
            ConfigEntry::new(
                ConfigKey::new("persistence.enabled").unwrap(),
                ConfigValue::Boolean(true),
                ConfigMetadata::new(ConfigCategory::Persistence, "Enable logging"),
            ),
            ConfigEntry::new(
                ConfigKey::new("persistence.enabled_models").unwrap(),
                ConfigValue::StringList(vec!["gpt-4".to_string()]),
                ConfigMetadata::new(ConfigCategory::Persistence, "Enabled models"),
            ),
        ];

        let config = AppConfiguration::from_entries(entries);

        assert!(config.should_log_model("gpt-4"));
        assert!(!config.should_log_model("claude-3"));
    }

    #[test]
    fn test_list_by_category() {
        let entries = vec![
            ConfigEntry::new(
                ConfigKey::new("persistence.enabled").unwrap(),
                ConfigValue::Boolean(true),
                ConfigMetadata::new(ConfigCategory::Persistence, "Enable logging"),
            ),
            ConfigEntry::new(
                ConfigKey::new("cache.enabled").unwrap(),
                ConfigValue::Boolean(true),
                ConfigMetadata::new(ConfigCategory::Cache, "Enable caching"),
            ),
        ];

        let config = AppConfiguration::from_entries(entries);

        let persistence_entries = config.list_by_category(ConfigCategory::Persistence);
        assert_eq!(persistence_entries.len(), 1);
        assert_eq!(
            persistence_entries[0].key().as_str(),
            "persistence.enabled"
        );
    }
}
