//! OpenAI Plugin
//!
//! Built-in plugin for OpenAI LLM provider.

use crate::domain::credentials::CredentialType;
use crate::domain::llm::LlmProvider;
use crate::domain::plugin::{
    ExtensionType, LlmProviderConfig, LlmProviderPlugin, Plugin, PluginContext, PluginError,
    PluginMetadata, PluginState,
};
use crate::infrastructure::llm::{HttpClient, OpenAiProvider};
use async_trait::async_trait;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;

/// OpenAI plugin implementation
#[derive(Debug)]
pub struct OpenAiPlugin {
    metadata: PluginMetadata,
    state: AtomicU8,
}

impl OpenAiPlugin {
    /// Create a new OpenAI plugin
    pub fn new() -> Self {
        Self {
            metadata: PluginMetadata::new("openai", "OpenAI", "1.0.0")
                .with_description("OpenAI LLM provider plugin for GPT models")
                .with_author("PMP LLM Gateway")
                .with_homepage("https://platform.openai.com/docs/api-reference"),
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

impl Default for OpenAiPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Plugin for OpenAiPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    fn extension_types(&self) -> Vec<ExtensionType> {
        vec![ExtensionType::LlmProvider, ExtensionType::EmbeddingProvider]
    }

    async fn initialize(&self, _context: PluginContext) -> Result<(), PluginError> {
        self.set_state(PluginState::Initializing);
        // OpenAI plugin has no complex initialization
        self.set_state(PluginState::Ready);
        Ok(())
    }

    async fn health_check(&self) -> Result<bool, PluginError> {
        // OpenAI health is checked via actual API calls
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
impl LlmProviderPlugin for OpenAiPlugin {
    fn supported_credential_types(&self) -> Vec<CredentialType> {
        vec![CredentialType::OpenAi]
    }

    async fn create_llm_provider(
        &self,
        config: LlmProviderConfig,
    ) -> Result<Arc<dyn LlmProvider>, PluginError> {
        if !self.supports_credential_type(&config.credential_type) {
            return Err(PluginError::unsupported_credential_type(
                "openai",
                format!("{:?}", config.credential_type),
            ));
        }

        let http_client = HttpClient::new();

        let provider = if let Some(base_url) = config.base_url() {
            OpenAiProvider::with_base_url(http_client, &config.api_key, base_url)
        } else {
            OpenAiProvider::new(http_client, &config.api_key)
        };

        Ok(Arc::new(provider))
    }

    fn available_models(&self) -> Vec<&'static str> {
        vec![
            "gpt-4o",
            "gpt-4o-mini",
            "gpt-4-turbo",
            "gpt-4-turbo-preview",
            "gpt-4",
            "gpt-4-0613",
            "gpt-3.5-turbo",
            "gpt-3.5-turbo-0125",
            "gpt-3.5-turbo-1106",
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openai_plugin_metadata() {
        let plugin = OpenAiPlugin::new();
        let metadata = plugin.metadata();

        assert_eq!(metadata.id, "openai");
        assert_eq!(metadata.name, "OpenAI");
        assert_eq!(metadata.version, "1.0.0");
    }

    #[test]
    fn test_openai_plugin_extension_types() {
        let plugin = OpenAiPlugin::new();
        let types = plugin.extension_types();

        assert!(types.contains(&ExtensionType::LlmProvider));
        assert!(types.contains(&ExtensionType::EmbeddingProvider));
    }

    #[test]
    fn test_openai_plugin_supported_credential_types() {
        let plugin = OpenAiPlugin::new();
        let types = plugin.supported_credential_types();

        assert!(types.contains(&CredentialType::OpenAi));
        assert!(!types.contains(&CredentialType::Anthropic));
    }

    #[test]
    fn test_openai_plugin_available_models() {
        let plugin = OpenAiPlugin::new();
        let models = plugin.available_models();

        assert!(models.contains(&"gpt-4o"));
        assert!(models.contains(&"gpt-4"));
        assert!(models.contains(&"gpt-3.5-turbo"));
    }

    #[tokio::test]
    async fn test_openai_plugin_initialize() {
        let plugin = OpenAiPlugin::new();

        assert_eq!(plugin.state(), PluginState::Registered);

        plugin.initialize(PluginContext::new()).await.unwrap();

        assert_eq!(plugin.state(), PluginState::Ready);
    }

    #[tokio::test]
    async fn test_openai_plugin_shutdown() {
        let plugin = OpenAiPlugin::new();
        plugin.initialize(PluginContext::new()).await.unwrap();

        plugin.shutdown().await.unwrap();

        assert_eq!(plugin.state(), PluginState::Stopped);
    }

    #[tokio::test]
    async fn test_openai_plugin_health_check() {
        let plugin = OpenAiPlugin::new();

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
    async fn test_openai_plugin_unsupported_credential_type() {
        let plugin = OpenAiPlugin::new();
        plugin.initialize(PluginContext::new()).await.unwrap();

        let config = LlmProviderConfig::new(CredentialType::Anthropic, "test-cred", "sk-test");

        let result = plugin.create_llm_provider(config).await;
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(matches!(err, PluginError::UnsupportedCredentialType { .. }));
    }
}
