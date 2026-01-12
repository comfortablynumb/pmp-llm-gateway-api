//! Azure OpenAI Plugin
//!
//! Built-in plugin for Azure OpenAI LLM provider.

use crate::domain::credentials::CredentialType;
use crate::domain::llm::LlmProvider;
use crate::domain::plugin::{
    ExtensionType, LlmProviderConfig, LlmProviderPlugin, Plugin, PluginContext, PluginError,
    PluginMetadata, PluginState,
};
use crate::infrastructure::llm::{AzureOpenAiConfig, AzureOpenAiProvider, HttpClient};
use async_trait::async_trait;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;

/// Azure OpenAI plugin implementation
#[derive(Debug)]
pub struct AzureOpenAiPlugin {
    metadata: PluginMetadata,
    state: AtomicU8,
}

impl AzureOpenAiPlugin {
    /// Create a new Azure OpenAI plugin
    pub fn new() -> Self {
        Self {
            metadata: PluginMetadata::new("azure_openai", "Azure OpenAI", "1.0.0")
                .with_description("Azure OpenAI LLM provider plugin for GPT models")
                .with_author("PMP LLM Gateway")
                .with_homepage("https://learn.microsoft.com/en-us/azure/ai-services/openai/"),
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

impl Default for AzureOpenAiPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Plugin for AzureOpenAiPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    fn extension_types(&self) -> Vec<ExtensionType> {
        vec![ExtensionType::LlmProvider, ExtensionType::EmbeddingProvider]
    }

    async fn initialize(&self, _context: PluginContext) -> Result<(), PluginError> {
        self.set_state(PluginState::Initializing);
        // Azure OpenAI plugin has no complex initialization
        self.set_state(PluginState::Ready);
        Ok(())
    }

    async fn health_check(&self) -> Result<bool, PluginError> {
        // Azure health is checked via actual API calls
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
impl LlmProviderPlugin for AzureOpenAiPlugin {
    fn supported_credential_types(&self) -> Vec<CredentialType> {
        vec![CredentialType::AzureOpenAi]
    }

    async fn create_llm_provider(
        &self,
        config: LlmProviderConfig,
    ) -> Result<Arc<dyn LlmProvider>, PluginError> {
        if !self.supports_credential_type(&config.credential_type) {
            return Err(PluginError::unsupported_credential_type(
                "azure_openai",
                format!("{:?}", config.credential_type),
            ));
        }

        // Azure requires an endpoint
        let endpoint = config.base_url().ok_or_else(|| {
            PluginError::configuration(
                "azure_openai",
                "Azure OpenAI requires 'base_url' parameter with the endpoint URL",
            )
        })?;

        let http_client = HttpClient::new();

        let mut azure_config = AzureOpenAiConfig::new(endpoint, &config.api_key);

        // Check for custom API version
        if let Some(api_version) = config.additional_params.get("api_version") {
            azure_config = azure_config.with_api_version(api_version);
        }

        let provider = AzureOpenAiProvider::new(http_client, azure_config);

        Ok(Arc::new(provider))
    }

    fn available_models(&self) -> Vec<&'static str> {
        // Azure uses deployment names, not model names
        // These are common deployment naming conventions
        vec![
            "gpt-4o",
            "gpt-4o-mini",
            "gpt-4-turbo",
            "gpt-4",
            "gpt-35-turbo",
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_azure_plugin_metadata() {
        let plugin = AzureOpenAiPlugin::new();
        let metadata = plugin.metadata();

        assert_eq!(metadata.id, "azure_openai");
        assert_eq!(metadata.name, "Azure OpenAI");
        assert_eq!(metadata.version, "1.0.0");
    }

    #[test]
    fn test_azure_plugin_extension_types() {
        let plugin = AzureOpenAiPlugin::new();
        let types = plugin.extension_types();

        assert!(types.contains(&ExtensionType::LlmProvider));
        assert!(types.contains(&ExtensionType::EmbeddingProvider));
    }

    #[test]
    fn test_azure_plugin_supported_credential_types() {
        let plugin = AzureOpenAiPlugin::new();
        let types = plugin.supported_credential_types();

        assert!(types.contains(&CredentialType::AzureOpenAi));
        assert!(!types.contains(&CredentialType::OpenAi));
    }

    #[test]
    fn test_azure_plugin_available_models() {
        let plugin = AzureOpenAiPlugin::new();
        let models = plugin.available_models();

        assert!(models.contains(&"gpt-4o"));
        assert!(models.contains(&"gpt-4"));
        assert!(models.contains(&"gpt-35-turbo"));
    }

    #[tokio::test]
    async fn test_azure_plugin_initialize() {
        let plugin = AzureOpenAiPlugin::new();

        assert_eq!(plugin.state(), PluginState::Registered);

        plugin.initialize(PluginContext::new()).await.unwrap();

        assert_eq!(plugin.state(), PluginState::Ready);
    }

    #[tokio::test]
    async fn test_azure_plugin_shutdown() {
        let plugin = AzureOpenAiPlugin::new();
        plugin.initialize(PluginContext::new()).await.unwrap();

        plugin.shutdown().await.unwrap();

        assert_eq!(plugin.state(), PluginState::Stopped);
    }

    #[tokio::test]
    async fn test_azure_plugin_health_check() {
        let plugin = AzureOpenAiPlugin::new();

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
    async fn test_azure_plugin_unsupported_credential_type() {
        let plugin = AzureOpenAiPlugin::new();
        plugin.initialize(PluginContext::new()).await.unwrap();

        let config = LlmProviderConfig::new(CredentialType::OpenAi, "test-cred", "sk-test");

        let result = plugin.create_llm_provider(config).await;
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(matches!(err, PluginError::UnsupportedCredentialType { .. }));
    }

    #[tokio::test]
    async fn test_azure_plugin_missing_endpoint() {
        let plugin = AzureOpenAiPlugin::new();
        plugin.initialize(PluginContext::new()).await.unwrap();

        // Azure requires base_url (endpoint)
        let config = LlmProviderConfig::new(CredentialType::AzureOpenAi, "test-cred", "sk-test");

        let result = plugin.create_llm_provider(config).await;
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(matches!(err, PluginError::Configuration { .. }));
    }
}
