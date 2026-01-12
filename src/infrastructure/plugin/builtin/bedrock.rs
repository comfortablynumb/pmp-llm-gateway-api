//! AWS Bedrock Plugin
//!
//! Built-in plugin for AWS Bedrock LLM provider.

use crate::domain::credentials::CredentialType;
use crate::domain::llm::LlmProvider;
use crate::domain::plugin::{
    ExtensionType, LlmProviderConfig, LlmProviderPlugin, Plugin, PluginContext, PluginError,
    PluginMetadata, PluginState,
};
use crate::infrastructure::llm::{BedrockClient, BedrockProvider};
use async_trait::async_trait;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;

/// AWS Bedrock plugin implementation
#[derive(Debug)]
pub struct BedrockPlugin {
    metadata: PluginMetadata,
    state: AtomicU8,
}

impl BedrockPlugin {
    /// Create a new AWS Bedrock plugin
    pub fn new() -> Self {
        Self {
            metadata: PluginMetadata::new("aws_bedrock", "AWS Bedrock", "1.0.0")
                .with_description("AWS Bedrock LLM provider plugin for Claude and Titan models")
                .with_author("PMP LLM Gateway")
                .with_homepage("https://docs.aws.amazon.com/bedrock/"),
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

impl Default for BedrockPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Plugin for BedrockPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    fn extension_types(&self) -> Vec<ExtensionType> {
        vec![ExtensionType::LlmProvider, ExtensionType::EmbeddingProvider]
    }

    async fn initialize(&self, _context: PluginContext) -> Result<(), PluginError> {
        self.set_state(PluginState::Initializing);
        // AWS SDK initialization happens lazily when providers are created
        self.set_state(PluginState::Ready);
        Ok(())
    }

    async fn health_check(&self) -> Result<bool, PluginError> {
        // AWS Bedrock health is checked via actual API calls
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
impl LlmProviderPlugin for BedrockPlugin {
    fn supported_credential_types(&self) -> Vec<CredentialType> {
        vec![CredentialType::AwsBedrock]
    }

    async fn create_llm_provider(
        &self,
        config: LlmProviderConfig,
    ) -> Result<Arc<dyn LlmProvider>, PluginError> {
        if !self.supports_credential_type(&config.credential_type) {
            return Err(PluginError::unsupported_credential_type(
                "aws_bedrock",
                format!("{:?}", config.credential_type),
            ));
        }

        // Get optional region from additional params
        let region = config.additional_params.get("region").map(String::as_str);

        // Initialize AWS SDK config
        let aws_config = if let Some(region) = region {
            aws_config::defaults(aws_config::BehaviorVersion::latest())
                .region(aws_config::Region::new(region.to_string()))
                .load()
                .await
        } else {
            aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await
        };

        let client = BedrockClient::new(&aws_config).await;
        let provider = BedrockProvider::new(client);

        Ok(Arc::new(provider))
    }

    fn available_models(&self) -> Vec<&'static str> {
        vec![
            "anthropic.claude-3-5-sonnet-20241022-v2:0",
            "anthropic.claude-3-5-haiku-20241022-v1:0",
            "anthropic.claude-3-opus-20240229-v1:0",
            "anthropic.claude-3-sonnet-20240229-v1:0",
            "anthropic.claude-3-haiku-20240307-v1:0",
            "amazon.titan-text-express-v1",
            "amazon.titan-text-lite-v1",
            "amazon.titan-embed-text-v1",
            "amazon.titan-embed-text-v2:0",
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bedrock_plugin_metadata() {
        let plugin = BedrockPlugin::new();
        let metadata = plugin.metadata();

        assert_eq!(metadata.id, "aws_bedrock");
        assert_eq!(metadata.name, "AWS Bedrock");
        assert_eq!(metadata.version, "1.0.0");
    }

    #[test]
    fn test_bedrock_plugin_extension_types() {
        let plugin = BedrockPlugin::new();
        let types = plugin.extension_types();

        assert!(types.contains(&ExtensionType::LlmProvider));
        assert!(types.contains(&ExtensionType::EmbeddingProvider));
    }

    #[test]
    fn test_bedrock_plugin_supported_credential_types() {
        let plugin = BedrockPlugin::new();
        let types = plugin.supported_credential_types();

        assert!(types.contains(&CredentialType::AwsBedrock));
        assert!(!types.contains(&CredentialType::OpenAi));
    }

    #[test]
    fn test_bedrock_plugin_available_models() {
        let plugin = BedrockPlugin::new();
        let models = plugin.available_models();

        assert!(models.contains(&"anthropic.claude-3-5-sonnet-20241022-v2:0"));
        assert!(models.contains(&"amazon.titan-text-express-v1"));
        assert!(models.contains(&"amazon.titan-embed-text-v1"));
    }

    #[tokio::test]
    async fn test_bedrock_plugin_initialize() {
        let plugin = BedrockPlugin::new();

        assert_eq!(plugin.state(), PluginState::Registered);

        plugin.initialize(PluginContext::new()).await.unwrap();

        assert_eq!(plugin.state(), PluginState::Ready);
    }

    #[tokio::test]
    async fn test_bedrock_plugin_shutdown() {
        let plugin = BedrockPlugin::new();
        plugin.initialize(PluginContext::new()).await.unwrap();

        plugin.shutdown().await.unwrap();

        assert_eq!(plugin.state(), PluginState::Stopped);
    }

    #[tokio::test]
    async fn test_bedrock_plugin_health_check() {
        let plugin = BedrockPlugin::new();

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
    async fn test_bedrock_plugin_unsupported_credential_type() {
        let plugin = BedrockPlugin::new();
        plugin.initialize(PluginContext::new()).await.unwrap();

        let config = LlmProviderConfig::new(CredentialType::OpenAi, "test-cred", "sk-test");

        let result = plugin.create_llm_provider(config).await;
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(matches!(err, PluginError::UnsupportedCredentialType { .. }));
    }
}
