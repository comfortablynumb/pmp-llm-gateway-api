//! Anthropic Plugin
//!
//! Built-in plugin for Anthropic LLM provider (Claude models).

use crate::domain::credentials::CredentialType;
use crate::domain::llm::LlmProvider;
use crate::domain::plugin::{
    ExtensionType, LlmProviderConfig, LlmProviderPlugin, Plugin, PluginContext, PluginError,
    PluginMetadata, PluginState,
};
use crate::infrastructure::llm::{HttpClient, AnthropicProvider};
use async_trait::async_trait;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;

/// Anthropic plugin implementation
#[derive(Debug)]
pub struct AnthropicPlugin {
    metadata: PluginMetadata,
    state: AtomicU8,
}

impl AnthropicPlugin {
    /// Create a new Anthropic plugin
    pub fn new() -> Self {
        Self {
            metadata: PluginMetadata::new("anthropic", "Anthropic", "1.0.0")
                .with_description("Anthropic LLM provider plugin for Claude models")
                .with_author("PMP LLM Gateway")
                .with_homepage("https://docs.anthropic.com/claude/reference"),
            state: AtomicU8::new(0),
        }
    }

    fn get_state(&self) -> PluginState {
        match self.state.load(Ordering::SeqCst) {
            0 => PluginState::Registered,
            1 => PluginState::Initializing,
            2 => PluginState::Ready,
            3 => PluginState::Error,
            4 => PluginState::ShuttingDown,
            5 => PluginState::Stopped,
            _ => PluginState::Error,
        }
    }

    fn set_state(&self, state: PluginState) {
        let value = match state {
            PluginState::Registered => 0,
            PluginState::Initializing => 1,
            PluginState::Ready => 2,
            PluginState::Error => 3,
            PluginState::ShuttingDown => 4,
            PluginState::Stopped => 5,
        };
        self.state.store(value, Ordering::SeqCst);
    }
}

impl Default for AnthropicPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Plugin for AnthropicPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    fn extension_types(&self) -> Vec<ExtensionType> {
        vec![ExtensionType::LlmProvider]
    }

    async fn initialize(&self, _context: PluginContext) -> Result<(), PluginError> {
        self.set_state(PluginState::Initializing);
        // Anthropic plugin has no complex initialization
        self.set_state(PluginState::Ready);
        Ok(())
    }

    async fn health_check(&self) -> Result<bool, PluginError> {
        // Anthropic health is checked via actual API calls
        Ok(self.get_state().is_ready())
    }

    async fn shutdown(&self) -> Result<(), PluginError> {
        self.set_state(PluginState::ShuttingDown);
        self.set_state(PluginState::Stopped);
        Ok(())
    }

    fn state(&self) -> PluginState {
        self.get_state()
    }
}

#[async_trait]
impl LlmProviderPlugin for AnthropicPlugin {
    fn supported_credential_types(&self) -> Vec<CredentialType> {
        vec![CredentialType::Anthropic]
    }

    async fn create_llm_provider(
        &self,
        config: LlmProviderConfig,
    ) -> Result<Arc<dyn LlmProvider>, PluginError> {
        if !self.supports_credential_type(&config.credential_type) {
            return Err(PluginError::unsupported_credential_type(
                "anthropic",
                format!("{:?}", config.credential_type),
            ));
        }

        let http_client = HttpClient::new();

        let provider = if let Some(base_url) = config.base_url() {
            AnthropicProvider::with_base_url(http_client, &config.api_key, base_url)
        } else {
            AnthropicProvider::new(http_client, &config.api_key)
        };

        Ok(Arc::new(provider))
    }

    fn available_models(&self) -> Vec<&'static str> {
        vec![
            "claude-opus-4-20250514",
            "claude-sonnet-4-20250514",
            "claude-3-5-sonnet-20241022",
            "claude-3-5-haiku-20241022",
            "claude-3-opus-20240229",
            "claude-3-sonnet-20240229",
            "claude-3-haiku-20240307",
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_anthropic_plugin_metadata() {
        let plugin = AnthropicPlugin::new();
        let metadata = plugin.metadata();

        assert_eq!(metadata.id, "anthropic");
        assert_eq!(metadata.name, "Anthropic");
        assert_eq!(metadata.version, "1.0.0");
    }

    #[test]
    fn test_anthropic_plugin_extension_types() {
        let plugin = AnthropicPlugin::new();
        let types = plugin.extension_types();

        assert!(types.contains(&ExtensionType::LlmProvider));
        // Anthropic doesn't provide embeddings
        assert!(!types.contains(&ExtensionType::EmbeddingProvider));
    }

    #[test]
    fn test_anthropic_plugin_supported_credential_types() {
        let plugin = AnthropicPlugin::new();
        let types = plugin.supported_credential_types();

        assert!(types.contains(&CredentialType::Anthropic));
        assert!(!types.contains(&CredentialType::OpenAi));
    }

    #[test]
    fn test_anthropic_plugin_available_models() {
        let plugin = AnthropicPlugin::new();
        let models = plugin.available_models();

        assert!(models.contains(&"claude-opus-4-20250514"));
        assert!(models.contains(&"claude-3-5-sonnet-20241022"));
        assert!(models.contains(&"claude-3-opus-20240229"));
    }

    #[tokio::test]
    async fn test_anthropic_plugin_initialize() {
        let plugin = AnthropicPlugin::new();

        assert_eq!(plugin.state(), PluginState::Registered);

        plugin.initialize(PluginContext::new()).await.unwrap();

        assert_eq!(plugin.state(), PluginState::Ready);
    }

    #[tokio::test]
    async fn test_anthropic_plugin_shutdown() {
        let plugin = AnthropicPlugin::new();
        plugin.initialize(PluginContext::new()).await.unwrap();

        plugin.shutdown().await.unwrap();

        assert_eq!(plugin.state(), PluginState::Stopped);
    }

    #[tokio::test]
    async fn test_anthropic_plugin_health_check() {
        let plugin = AnthropicPlugin::new();

        // Not ready yet
        let health = plugin.health_check().await.unwrap();
        assert!(!health);

        // Initialize
        plugin.initialize(PluginContext::new()).await.unwrap();

        // Now ready
        let health = plugin.health_check().await.unwrap();
        assert!(health);
    }

    #[tokio::test]
    async fn test_anthropic_plugin_unsupported_credential_type() {
        let plugin = AnthropicPlugin::new();
        plugin.initialize(PluginContext::new()).await.unwrap();

        let config = LlmProviderConfig::new(CredentialType::OpenAi, "test-cred", "sk-test");

        let result = plugin.create_llm_provider(config).await;
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(matches!(err, PluginError::UnsupportedCredentialType { .. }));
    }
}
