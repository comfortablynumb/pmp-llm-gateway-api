//! Plugin configuration via TOML
//!
//! Supports loading plugin configuration from TOML files with:
//! - Enable/disable built-in plugins
//! - Custom plugin settings
//! - Per-provider configuration

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use thiserror::Error;
use tracing::{debug, info, warn};

/// Error type for plugin configuration
#[derive(Debug, Error)]
pub enum PluginConfigError {
    #[error("Failed to read config file: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Failed to parse TOML: {0}")]
    TomlError(#[from] toml::de::Error),

    #[error("Invalid configuration: {0}")]
    ValidationError(String),
}

/// Root configuration structure for plugins
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginConfig {
    /// Global plugin settings
    #[serde(default)]
    pub settings: PluginSettings,

    /// Built-in provider configurations
    #[serde(default)]
    pub providers: ProviderConfigs,

    /// Custom plugin paths (for future extension)
    #[serde(default)]
    pub custom_plugins: Vec<CustomPluginEntry>,
}

/// Global settings for the plugin system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginSettings {
    /// Enable the plugin system (default: true)
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Maximum number of cached providers per credential (default: 100)
    #[serde(default = "default_cache_size")]
    pub max_provider_cache_size: usize,

    /// Log plugin operations at debug level (default: false)
    #[serde(default)]
    pub debug_logging: bool,
}

impl Default for PluginSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            max_provider_cache_size: 100,
            debug_logging: false,
        }
    }
}

/// Configuration for built-in LLM providers
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProviderConfigs {
    /// OpenAI provider configuration
    #[serde(default)]
    pub openai: ProviderEntry,

    /// Anthropic provider configuration
    #[serde(default)]
    pub anthropic: ProviderEntry,

    /// Azure OpenAI provider configuration
    #[serde(default)]
    pub azure_openai: ProviderEntry,

    /// AWS Bedrock provider configuration
    #[serde(default)]
    pub bedrock: ProviderEntry,
}

/// Configuration for a single provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderEntry {
    /// Enable this provider (default: true)
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Provider-specific settings
    #[serde(default)]
    pub settings: HashMap<String, toml::Value>,
}

impl Default for ProviderEntry {
    fn default() -> Self {
        Self {
            enabled: true,
            settings: HashMap::new(),
        }
    }
}

/// Entry for a custom plugin (future extension)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomPluginEntry {
    /// Plugin identifier
    pub id: String,

    /// Path to the plugin (currently unused, reserved for future)
    pub path: Option<String>,

    /// Enable this plugin
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Plugin-specific configuration
    #[serde(default)]
    pub config: HashMap<String, toml::Value>,
}

fn default_true() -> bool {
    true
}

fn default_cache_size() -> usize {
    100
}

impl PluginConfig {
    /// Create a new default configuration with all providers enabled
    pub fn new() -> Self {
        Self::default()
    }

    /// Load configuration from a TOML file
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, PluginConfigError> {
        let path = path.as_ref();
        info!(path = %path.display(), "Loading plugin configuration");

        let content = std::fs::read_to_string(path)?;
        let config: PluginConfig = toml::from_str(&content)?;

        config.validate()?;
        debug!("Plugin configuration loaded successfully");

        Ok(config)
    }

    /// Load configuration from a TOML string
    pub fn from_str(content: &str) -> Result<Self, PluginConfigError> {
        let config: PluginConfig = toml::from_str(content)?;
        config.validate()?;
        Ok(config)
    }

    /// Try to load from file, falling back to defaults
    pub fn load_or_default(path: impl AsRef<Path>) -> Self {
        let path = path.as_ref();

        if !path.exists() {
            debug!(path = %path.display(), "Plugin config file not found, using defaults");
            return Self::default();
        }

        match Self::from_file(path) {
            Ok(config) => config,
            Err(e) => {
                warn!(
                    path = %path.display(),
                    error = %e,
                    "Failed to load plugin config, using defaults"
                );
                Self::default()
            }
        }
    }

    /// Validate the configuration
    fn validate(&self) -> Result<(), PluginConfigError> {
        if self.settings.max_provider_cache_size == 0 {
            return Err(PluginConfigError::ValidationError(
                "max_provider_cache_size must be greater than 0".into(),
            ));
        }

        Ok(())
    }

    /// Check if a built-in provider is enabled
    pub fn is_provider_enabled(&self, provider: BuiltinProvider) -> bool {
        if !self.settings.enabled {
            return false;
        }

        match provider {
            BuiltinProvider::OpenAi => self.providers.openai.enabled,
            BuiltinProvider::Anthropic => self.providers.anthropic.enabled,
            BuiltinProvider::AzureOpenAi => self.providers.azure_openai.enabled,
            BuiltinProvider::Bedrock => self.providers.bedrock.enabled,
        }
    }

    /// Get settings for a built-in provider
    pub fn get_provider_settings(&self, provider: BuiltinProvider) -> &HashMap<String, toml::Value> {
        match provider {
            BuiltinProvider::OpenAi => &self.providers.openai.settings,
            BuiltinProvider::Anthropic => &self.providers.anthropic.settings,
            BuiltinProvider::AzureOpenAi => &self.providers.azure_openai.settings,
            BuiltinProvider::Bedrock => &self.providers.bedrock.settings,
        }
    }

    /// Get list of enabled built-in providers
    pub fn enabled_providers(&self) -> Vec<BuiltinProvider> {
        if !self.settings.enabled {
            return vec![];
        }

        let mut providers = Vec::new();

        if self.providers.openai.enabled {
            providers.push(BuiltinProvider::OpenAi);
        }

        if self.providers.anthropic.enabled {
            providers.push(BuiltinProvider::Anthropic);
        }

        if self.providers.azure_openai.enabled {
            providers.push(BuiltinProvider::AzureOpenAi);
        }

        if self.providers.bedrock.enabled {
            providers.push(BuiltinProvider::Bedrock);
        }

        providers
    }
}

/// Enum representing built-in providers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BuiltinProvider {
    OpenAi,
    Anthropic,
    AzureOpenAi,
    Bedrock,
}

impl BuiltinProvider {
    /// Get all built-in providers
    pub fn all() -> &'static [BuiltinProvider] {
        &[
            BuiltinProvider::OpenAi,
            BuiltinProvider::Anthropic,
            BuiltinProvider::AzureOpenAi,
            BuiltinProvider::Bedrock,
        ]
    }

    /// Get the provider name as a string
    pub fn name(&self) -> &'static str {
        match self {
            BuiltinProvider::OpenAi => "openai",
            BuiltinProvider::Anthropic => "anthropic",
            BuiltinProvider::AzureOpenAi => "azure_openai",
            BuiltinProvider::Bedrock => "bedrock",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = PluginConfig::default();

        assert!(config.settings.enabled);
        assert_eq!(config.settings.max_provider_cache_size, 100);
        assert!(config.providers.openai.enabled);
        assert!(config.providers.anthropic.enabled);
        assert!(config.providers.azure_openai.enabled);
        assert!(config.providers.bedrock.enabled);
    }

    #[test]
    fn test_parse_minimal_config() {
        let toml = r#"
[settings]
enabled = true
"#;

        let config = PluginConfig::from_str(toml).unwrap();
        assert!(config.settings.enabled);
        assert!(config.providers.openai.enabled);
    }

    #[test]
    fn test_parse_disable_provider() {
        let toml = r#"
[providers.anthropic]
enabled = false
"#;

        let config = PluginConfig::from_str(toml).unwrap();
        assert!(config.providers.openai.enabled);
        assert!(!config.providers.anthropic.enabled);
    }

    #[test]
    fn test_parse_provider_settings() {
        let toml = r#"
[providers.openai.settings]
default_model = "gpt-4"
timeout_seconds = 30
"#;

        let config = PluginConfig::from_str(toml).unwrap();
        let settings = config.get_provider_settings(BuiltinProvider::OpenAi);

        assert_eq!(
            settings.get("default_model").unwrap().as_str().unwrap(),
            "gpt-4"
        );
        assert_eq!(
            settings.get("timeout_seconds").unwrap().as_integer().unwrap(),
            30
        );
    }

    #[test]
    fn test_parse_full_config() {
        let toml = r#"
[settings]
enabled = true
max_provider_cache_size = 50
debug_logging = true

[providers.openai]
enabled = true

[providers.openai.settings]
default_model = "gpt-4"

[providers.anthropic]
enabled = false

[providers.azure_openai]
enabled = true

[providers.bedrock]
enabled = true
"#;

        let config = PluginConfig::from_str(toml).unwrap();

        assert!(config.settings.enabled);
        assert_eq!(config.settings.max_provider_cache_size, 50);
        assert!(config.settings.debug_logging);

        assert!(config.is_provider_enabled(BuiltinProvider::OpenAi));
        assert!(!config.is_provider_enabled(BuiltinProvider::Anthropic));
        assert!(config.is_provider_enabled(BuiltinProvider::AzureOpenAi));
        assert!(config.is_provider_enabled(BuiltinProvider::Bedrock));
    }

    #[test]
    fn test_enabled_providers() {
        let toml = r#"
[providers.anthropic]
enabled = false

[providers.bedrock]
enabled = false
"#;

        let config = PluginConfig::from_str(toml).unwrap();
        let enabled = config.enabled_providers();

        assert!(enabled.contains(&BuiltinProvider::OpenAi));
        assert!(!enabled.contains(&BuiltinProvider::Anthropic));
        assert!(enabled.contains(&BuiltinProvider::AzureOpenAi));
        assert!(!enabled.contains(&BuiltinProvider::Bedrock));
    }

    #[test]
    fn test_disabled_plugin_system() {
        let toml = r#"
[settings]
enabled = false
"#;

        let config = PluginConfig::from_str(toml).unwrap();

        assert!(!config.is_provider_enabled(BuiltinProvider::OpenAi));
        assert!(config.enabled_providers().is_empty());
    }

    #[test]
    fn test_validation_zero_cache() {
        let toml = r#"
[settings]
max_provider_cache_size = 0
"#;

        let result = PluginConfig::from_str(toml);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("max_provider_cache_size"));
    }

    #[test]
    fn test_custom_plugins() {
        let toml = r#"
[[custom_plugins]]
id = "my-plugin"
path = "/path/to/plugin.wasm"
enabled = true

[custom_plugins.config]
api_key = "secret"
"#;

        let config = PluginConfig::from_str(toml).unwrap();

        assert_eq!(config.custom_plugins.len(), 1);
        assert_eq!(config.custom_plugins[0].id, "my-plugin");
        assert!(config.custom_plugins[0].enabled);
    }
}
