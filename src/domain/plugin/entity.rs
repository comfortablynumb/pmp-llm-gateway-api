//! Plugin entity types and core trait
//!
//! Defines the Plugin trait and associated metadata structures.

use super::error::PluginError;
use super::extensions::ExtensionType;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Debug;

/// Plugin metadata containing identification and capability information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    /// Unique identifier for the plugin
    pub id: String,

    /// Human-readable name
    pub name: String,

    /// Plugin version (semver format)
    pub version: String,

    /// Plugin description
    pub description: String,

    /// Author or maintainer
    pub author: Option<String>,

    /// License identifier
    pub license: Option<String>,

    /// Homepage or documentation URL
    pub homepage: Option<String>,
}

impl PluginMetadata {
    pub fn new(id: impl Into<String>, name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            version: version.into(),
            description: String::new(),
            author: None,
            license: None,
            homepage: None,
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    pub fn with_author(mut self, author: impl Into<String>) -> Self {
        self.author = Some(author.into());
        self
    }

    pub fn with_license(mut self, license: impl Into<String>) -> Self {
        self.license = Some(license.into());
        self
    }

    pub fn with_homepage(mut self, homepage: impl Into<String>) -> Self {
        self.homepage = Some(homepage.into());
        self
    }
}

/// Context provided to plugins during initialization
#[derive(Debug, Clone)]
pub struct PluginContext {
    /// Plugin-specific configuration from the main config
    pub config: HashMap<String, serde_json::Value>,

    /// Base URL for API calls (if applicable)
    pub base_url: Option<String>,

    /// Environment name (development, staging, production)
    pub environment: String,
}

impl PluginContext {
    pub fn new() -> Self {
        Self {
            config: HashMap::new(),
            base_url: None,
            environment: "development".to_string(),
        }
    }

    pub fn with_config(mut self, config: HashMap<String, serde_json::Value>) -> Self {
        self.config = config;
        self
    }

    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = Some(base_url.into());
        self
    }

    pub fn with_environment(mut self, environment: impl Into<String>) -> Self {
        self.environment = environment.into();
        self
    }

    /// Get a config value as a specific type
    pub fn get_config<T: for<'de> Deserialize<'de>>(
        &self,
        key: &str,
    ) -> Result<Option<T>, serde_json::Error> {
        match self.config.get(key) {
            Some(value) => serde_json::from_value(value.clone()).map(Some),
            None => Ok(None),
        }
    }

    /// Get a config value with a default
    pub fn get_config_or<T: for<'de> Deserialize<'de>>(
        &self,
        key: &str,
        default: T,
    ) -> Result<T, serde_json::Error> {
        self.get_config(key).map(|opt| opt.unwrap_or(default))
    }
}

impl Default for PluginContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Current state of a plugin
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginState {
    /// Plugin is registered but not initialized
    Registered,

    /// Plugin is currently initializing
    Initializing,

    /// Plugin is initialized and ready
    Ready,

    /// Plugin encountered an error
    Error,

    /// Plugin is shutting down
    ShuttingDown,

    /// Plugin has been shut down
    Stopped,
}

impl PluginState {
    pub fn is_ready(&self) -> bool {
        matches!(self, PluginState::Ready)
    }

    pub fn can_initialize(&self) -> bool {
        matches!(self, PluginState::Registered | PluginState::Error)
    }

    pub fn can_shutdown(&self) -> bool {
        matches!(self, PluginState::Ready | PluginState::Error)
    }
}

/// Core plugin trait that all plugins must implement
#[async_trait]
pub trait Plugin: Send + Sync + Debug {
    /// Get plugin metadata
    fn metadata(&self) -> &PluginMetadata;

    /// Get the types of extensions this plugin provides
    fn extension_types(&self) -> Vec<ExtensionType>;

    /// Initialize the plugin with the given context
    async fn initialize(&self, context: PluginContext) -> Result<(), PluginError>;

    /// Check if the plugin is healthy and operational
    async fn health_check(&self) -> Result<bool, PluginError>;

    /// Shutdown the plugin gracefully
    async fn shutdown(&self) -> Result<(), PluginError>;

    /// Get the current plugin state
    fn state(&self) -> PluginState;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_metadata_builder() {
        let metadata = PluginMetadata::new("openai", "OpenAI Plugin", "1.0.0")
            .with_description("OpenAI LLM provider plugin")
            .with_author("Anthropic")
            .with_license("MIT")
            .with_homepage("https://github.com/example/plugin");

        assert_eq!(metadata.id, "openai");
        assert_eq!(metadata.name, "OpenAI Plugin");
        assert_eq!(metadata.version, "1.0.0");
        assert_eq!(metadata.description, "OpenAI LLM provider plugin");
        assert_eq!(metadata.author, Some("Anthropic".to_string()));
        assert_eq!(metadata.license, Some("MIT".to_string()));
    }

    #[test]
    fn test_plugin_context_builder() {
        let mut config = HashMap::new();
        config.insert("api_key".to_string(), serde_json::json!("test-key"));
        config.insert("timeout".to_string(), serde_json::json!(30));

        let context = PluginContext::new()
            .with_config(config)
            .with_base_url("https://api.openai.com/v1")
            .with_environment("production");

        assert_eq!(context.base_url, Some("https://api.openai.com/v1".to_string()));
        assert_eq!(context.environment, "production");

        let api_key: Option<String> = context.get_config("api_key").unwrap();
        assert_eq!(api_key, Some("test-key".to_string()));

        let timeout: i32 = context.get_config_or("timeout", 60).unwrap();
        assert_eq!(timeout, 30);

        let missing: i32 = context.get_config_or("missing", 100).unwrap();
        assert_eq!(missing, 100);
    }

    #[test]
    fn test_plugin_state_transitions() {
        assert!(PluginState::Registered.can_initialize());
        assert!(PluginState::Error.can_initialize());
        assert!(!PluginState::Ready.can_initialize());

        assert!(PluginState::Ready.can_shutdown());
        assert!(PluginState::Error.can_shutdown());
        assert!(!PluginState::Registered.can_shutdown());

        assert!(PluginState::Ready.is_ready());
        assert!(!PluginState::Registered.is_ready());
    }

    #[test]
    fn test_plugin_state_serialization() {
        let state = PluginState::Ready;
        let json = serde_json::to_string(&state).unwrap();
        assert_eq!(json, "\"ready\"");

        let deserialized: PluginState = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, PluginState::Ready);
    }
}
