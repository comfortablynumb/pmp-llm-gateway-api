//! LLM Provider plugin trait
//!
//! Defines the interface for plugins that provide LLM capabilities.

use super::entity::Plugin;
use super::error::PluginError;
use crate::domain::credentials::CredentialType;
use crate::domain::llm::LlmProvider;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

/// Configuration for creating an LLM provider instance
#[derive(Debug, Clone)]
pub struct LlmProviderConfig {
    /// The credential type for authentication
    pub credential_type: CredentialType,

    /// Credential ID for looking up credentials
    pub credential_id: String,

    /// API key or secret for authentication
    pub api_key: String,

    /// Additional parameters (e.g., base_url, api_version, region)
    pub additional_params: HashMap<String, String>,
}

impl LlmProviderConfig {
    pub fn new(
        credential_type: CredentialType,
        credential_id: impl Into<String>,
        api_key: impl Into<String>,
    ) -> Self {
        Self {
            credential_type,
            credential_id: credential_id.into(),
            api_key: api_key.into(),
            additional_params: HashMap::new(),
        }
    }

    pub fn with_param(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.additional_params.insert(key.into(), value.into());
        self
    }

    pub fn with_params(mut self, params: HashMap<String, String>) -> Self {
        self.additional_params.extend(params);
        self
    }

    /// Get a parameter value
    pub fn get_param(&self, key: &str) -> Option<&String> {
        self.additional_params.get(key)
    }

    /// Get base_url if present
    pub fn base_url(&self) -> Option<&String> {
        self.get_param("base_url")
    }

    /// Get api_version if present
    pub fn api_version(&self) -> Option<&String> {
        self.get_param("api_version")
    }

    /// Get region if present
    pub fn region(&self) -> Option<&String> {
        self.get_param("region")
    }
}

/// Trait for plugins that provide LLM (Large Language Model) capabilities
#[async_trait]
pub trait LlmProviderPlugin: Plugin {
    /// Get the credential types this plugin supports
    fn supported_credential_types(&self) -> Vec<CredentialType>;

    /// Check if this plugin supports the given credential type
    fn supports_credential_type(&self, credential_type: &CredentialType) -> bool {
        self.supported_credential_types().contains(credential_type)
    }

    /// Create an LLM provider instance with the given configuration
    ///
    /// # Arguments
    /// * `config` - Configuration containing credentials and settings
    ///
    /// # Returns
    /// * `Ok(Arc<dyn LlmProvider>)` - The created provider instance
    /// * `Err(PluginError)` - If provider creation fails
    async fn create_llm_provider(
        &self,
        config: LlmProviderConfig,
    ) -> Result<Arc<dyn LlmProvider>, PluginError>;

    /// Get the list of models available through this plugin
    fn available_models(&self) -> Vec<&'static str>;

    /// Validate that a model is supported by this plugin
    fn supports_model(&self, model: &str) -> bool {
        self.available_models().iter().any(|m| *m == model)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_llm_provider_config_builder() {
        let config = LlmProviderConfig::new(CredentialType::OpenAi, "openai-default", "sk-test-key")
            .with_param("base_url", "https://api.openai.com/v1")
            .with_param("timeout", "30");

        assert!(matches!(config.credential_type, CredentialType::OpenAi));
        assert_eq!(config.credential_id, "openai-default");
        assert_eq!(config.api_key, "sk-test-key");
        assert_eq!(
            config.base_url(),
            Some(&"https://api.openai.com/v1".to_string())
        );
        assert_eq!(config.get_param("timeout"), Some(&"30".to_string()));
    }

    #[test]
    fn test_llm_provider_config_with_params() {
        let mut params = HashMap::new();
        params.insert("region".to_string(), "us-east-1".to_string());
        params.insert("endpoint".to_string(), "custom".to_string());

        let config =
            LlmProviderConfig::new(CredentialType::AwsBedrock, "bedrock-prod", "").with_params(params);

        assert_eq!(config.region(), Some(&"us-east-1".to_string()));
        assert_eq!(config.get_param("endpoint"), Some(&"custom".to_string()));
    }
}
