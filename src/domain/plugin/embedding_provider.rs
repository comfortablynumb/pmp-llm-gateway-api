//! Embedding Provider plugin trait
//!
//! Defines the interface for plugins that provide embedding capabilities.

use super::entity::Plugin;
use super::error::PluginError;
use super::llm_provider::LlmProviderConfig;
use crate::domain::credentials::CredentialType;
use crate::domain::embedding::EmbeddingProvider;
use async_trait::async_trait;
use std::sync::Arc;

/// Trait for plugins that provide embedding capabilities
#[async_trait]
pub trait EmbeddingProviderPlugin: Plugin {
    /// Get the credential types this plugin supports for embeddings
    fn supported_credential_types(&self) -> Vec<CredentialType>;

    /// Check if this plugin supports the given credential type
    fn supports_credential_type(&self, credential_type: &CredentialType) -> bool {
        self.supported_credential_types().contains(credential_type)
    }

    /// Create an embedding provider instance with the given configuration
    ///
    /// # Arguments
    /// * `config` - Configuration containing credentials and settings
    ///
    /// # Returns
    /// * `Ok(Arc<dyn EmbeddingProvider>)` - The created provider instance
    /// * `Err(PluginError)` - If provider creation fails
    async fn create_embedding_provider(
        &self,
        config: LlmProviderConfig,
    ) -> Result<Arc<dyn EmbeddingProvider>, PluginError>;

    /// Get the default embedding model for this plugin
    fn default_model(&self) -> &'static str;

    /// Get the list of embedding models available through this plugin
    fn available_models(&self) -> Vec<&'static str>;

    /// Get the embedding dimensions for a specific model
    fn dimensions(&self, model: &str) -> Option<usize>;
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests would require mock implementations
    #[test]
    fn test_embedding_provider_config() {
        let config =
            LlmProviderConfig::new(CredentialType::OpenAi, "openai-default", "sk-test-key")
                .with_param("model", "text-embedding-3-small");

        assert!(matches!(config.credential_type, CredentialType::OpenAi));
        assert_eq!(config.get_param("model"), Some(&"text-embedding-3-small".to_string()));
    }
}
