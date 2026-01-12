//! Application configuration domain entities

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::domain::storage::{StorageEntity, StorageKey};

/// Storage key for the singleton AppConfiguration
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AppConfigurationId(String);

impl AppConfigurationId {
    /// The singleton configuration ID
    pub const SINGLETON: &'static str = "app_config";

    pub fn singleton() -> Self {
        Self(Self::SINGLETON.to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for AppConfigurationId {
    fn default() -> Self {
        Self::singleton()
    }
}

impl StorageKey for AppConfigurationId {
    fn as_str(&self) -> &str {
        &self.0
    }
}

/// Configuration key identifier (for individual settings)
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
}

impl std::fmt::Display for ConfigCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A single configuration entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigEntry {
    key: ConfigKey,
    value: ConfigValue,
    category: ConfigCategory,
    description: Option<String>,
    updated_at: DateTime<Utc>,
}

impl ConfigEntry {
    pub fn new(
        key: ConfigKey,
        value: ConfigValue,
        category: ConfigCategory,
        description: Option<String>,
    ) -> Self {
        Self {
            key,
            value,
            category,
            description,
            updated_at: Utc::now(),
        }
    }

    pub fn key(&self) -> &ConfigKey {
        &self.key
    }

    pub fn value(&self) -> &ConfigValue {
        &self.value
    }

    pub fn category(&self) -> ConfigCategory {
        self.category
    }

    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    pub fn set_value(&mut self, value: ConfigValue) {
        self.value = value;
        self.updated_at = Utc::now();
    }
}

/// Application configuration collection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfiguration {
    id: AppConfigurationId,
    entries: HashMap<String, ConfigEntry>,
}

impl Default for AppConfiguration {
    fn default() -> Self {
        Self {
            id: AppConfigurationId::singleton(),
            entries: HashMap::new(),
        }
    }
}

impl StorageEntity for AppConfiguration {
    type Key = AppConfigurationId;

    fn key(&self) -> &Self::Key {
        &self.id
    }
}

impl AppConfiguration {
    pub fn new() -> Self {
        Self::default()
    }

    /// Create configuration with default values
    pub fn with_defaults() -> Self {
        let mut config = Self::new();

        // Persistence settings
        config.set_default(
            "persistence.enabled",
            ConfigValue::Boolean(false),
            ConfigCategory::Persistence,
            Some("Enable execution logging"),
        );
        config.set_default(
            "persistence.enabled_models",
            ConfigValue::StringList(vec![]),
            ConfigCategory::Persistence,
            Some("List of model IDs to log executions for (empty = all if enabled)"),
        );
        config.set_default(
            "persistence.enabled_workflows",
            ConfigValue::StringList(vec![]),
            ConfigCategory::Persistence,
            Some("List of workflow IDs to log executions for (empty = all if enabled)"),
        );
        config.set_default(
            "persistence.log_retention_days",
            ConfigValue::Integer(30),
            ConfigCategory::Persistence,
            Some("Number of days to retain execution logs"),
        );
        config.set_default(
            "persistence.log_sensitive_data",
            ConfigValue::Boolean(false),
            ConfigCategory::Persistence,
            Some("Whether to log full input/output (may contain sensitive data)"),
        );

        // Logging settings
        config.set_default(
            "logging.level",
            ConfigValue::String("info".to_string()),
            ConfigCategory::Logging,
            Some("Log level (trace, debug, info, warn, error)"),
        );
        config.set_default(
            "logging.format",
            ConfigValue::String("json".to_string()),
            ConfigCategory::Logging,
            Some("Log format (json, pretty)"),
        );

        // Cache settings
        config.set_default(
            "cache.enabled",
            ConfigValue::Boolean(true),
            ConfigCategory::Cache,
            Some("Enable response caching"),
        );
        config.set_default(
            "cache.ttl_seconds",
            ConfigValue::Integer(3600),
            ConfigCategory::Cache,
            Some("Cache TTL in seconds"),
        );
        config.set_default(
            "cache.max_entries",
            ConfigValue::Integer(10000),
            ConfigCategory::Cache,
            Some("Maximum cache entries"),
        );

        // Security settings
        config.set_default(
            "security.require_api_key",
            ConfigValue::Boolean(true),
            ConfigCategory::Security,
            Some("Require API key for all requests"),
        );
        config.set_default(
            "security.allowed_origins",
            ConfigValue::StringList(vec!["*".to_string()]),
            ConfigCategory::Security,
            Some("Allowed CORS origins"),
        );

        // Rate limit settings
        config.set_default(
            "rate_limit.enabled",
            ConfigValue::Boolean(true),
            ConfigCategory::RateLimit,
            Some("Enable rate limiting"),
        );
        config.set_default(
            "rate_limit.default_rpm",
            ConfigValue::Integer(60),
            ConfigCategory::RateLimit,
            Some("Default requests per minute"),
        );

        config
    }

    fn set_default(
        &mut self,
        key: &str,
        value: ConfigValue,
        category: ConfigCategory,
        description: Option<&str>,
    ) {
        let config_key = ConfigKey::new(key).expect("Invalid default config key");
        let entry = ConfigEntry::new(
            config_key,
            value,
            category,
            description.map(|s| s.to_string()),
        );
        self.entries.insert(key.to_string(), entry);
    }

    pub fn get(&self, key: &str) -> Option<&ConfigEntry> {
        self.entries.get(key)
    }

    pub fn get_value(&self, key: &str) -> Option<&ConfigValue> {
        self.entries.get(key).map(|e| e.value())
    }

    pub fn set(&mut self, key: ConfigKey, value: ConfigValue) -> Result<(), ConfigValidationError> {
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
        assert_eq!(list_val.as_string_list(), Some(&["a".to_string(), "b".to_string()][..]));
    }

    #[test]
    fn test_app_configuration_defaults() {
        let config = AppConfiguration::with_defaults();

        assert!(!config.is_persistence_enabled());
        assert_eq!(config.log_retention_days(), 30);
        assert!(!config.log_sensitive_data());
        assert!(config.enabled_models().is_empty());
        assert!(config.enabled_workflows().is_empty());
    }

    #[test]
    fn test_app_configuration_set() {
        let mut config = AppConfiguration::with_defaults();

        let key = ConfigKey::new("persistence.enabled").unwrap();
        assert!(config.set(key, ConfigValue::Boolean(true)).is_ok());
        assert!(config.is_persistence_enabled());
    }

    #[test]
    fn test_app_configuration_type_mismatch() {
        let mut config = AppConfiguration::with_defaults();

        let key = ConfigKey::new("persistence.enabled").unwrap();
        let result = config.set(key, ConfigValue::String("true".to_string()));
        assert!(matches!(result, Err(ConfigValidationError::TypeMismatch { .. })));
    }

    #[test]
    fn test_should_log_model() {
        let mut config = AppConfiguration::with_defaults();

        // Persistence disabled
        assert!(!config.should_log_model("gpt-4"));

        // Enable persistence
        let key = ConfigKey::new("persistence.enabled").unwrap();
        config.set(key, ConfigValue::Boolean(true)).unwrap();

        // Empty list = log all
        assert!(config.should_log_model("gpt-4"));
        assert!(config.should_log_model("claude-3"));

        // Specific list
        let key = ConfigKey::new("persistence.enabled_models").unwrap();
        config
            .set(key, ConfigValue::StringList(vec!["gpt-4".to_string()]))
            .unwrap();

        assert!(config.should_log_model("gpt-4"));
        assert!(!config.should_log_model("claude-3"));
    }

    #[test]
    fn test_list_by_category() {
        let config = AppConfiguration::with_defaults();

        let persistence_entries = config.list_by_category(ConfigCategory::Persistence);
        assert!(!persistence_entries.is_empty());

        for entry in persistence_entries {
            assert_eq!(entry.category(), ConfigCategory::Persistence);
        }
    }
}
