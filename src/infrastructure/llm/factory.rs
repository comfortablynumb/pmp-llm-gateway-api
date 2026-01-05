use serde::Deserialize;
use std::sync::Arc;

use super::bedrock::{BedrockClient, BedrockProvider};
use super::http_client::HttpClient;
use super::{AnthropicProvider, AzureOpenAiProvider, OpenAiProvider};
use crate::domain::{Credential, CredentialType, DomainError, LlmProvider};
use crate::infrastructure::llm::azure_openai::AzureOpenAiConfig;

/// LLM provider configuration
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LlmProviderConfig {
    OpenAi,
    Anthropic,
    AzureOpenAi {
        endpoint: String,
        #[serde(default = "default_api_version")]
        api_version: String,
    },
    AwsBedrock {
        #[serde(default)]
        region: Option<String>,
    },
}

fn default_api_version() -> String {
    "2024-02-01".to_string()
}

/// Factory for creating LLM providers
#[derive(Debug)]
pub struct LlmProviderFactory;

impl LlmProviderFactory {
    /// Create an LLM provider from configuration and credential
    pub fn create(
        config: &LlmProviderConfig,
        credential: &Credential,
    ) -> Result<Arc<dyn LlmProvider>, DomainError> {
        let http_client = HttpClient::new();

        match config {
            LlmProviderConfig::OpenAi => {
                Self::validate_credential_type(credential, &CredentialType::OpenAi)?;
                let provider = OpenAiProvider::new(http_client, credential.api_key());
                Ok(Arc::new(provider))
            }

            LlmProviderConfig::Anthropic => {
                Self::validate_credential_type(credential, &CredentialType::Anthropic)?;
                let provider = AnthropicProvider::new(http_client, credential.api_key());
                Ok(Arc::new(provider))
            }

            LlmProviderConfig::AzureOpenAi {
                endpoint,
                api_version,
            } => {
                Self::validate_credential_type(credential, &CredentialType::AzureOpenAi)?;

                let azure_config = AzureOpenAiConfig::new(endpoint, credential.api_key())
                    .with_api_version(api_version);

                let provider = AzureOpenAiProvider::new(http_client, azure_config);
                Ok(Arc::new(provider))
            }

            LlmProviderConfig::AwsBedrock { .. } => {
                // Bedrock requires async initialization - use create_bedrock_async instead
                Err(DomainError::configuration(
                    "Bedrock provider requires async initialization. Use create_bedrock_async instead.",
                ))
            }
        }
    }

    /// Create an AWS Bedrock provider asynchronously
    pub async fn create_bedrock_async(
        region: Option<&str>,
    ) -> Result<Arc<dyn LlmProvider>, DomainError> {
        let config = if let Some(region) = region {
            aws_config::defaults(aws_config::BehaviorVersion::latest())
                .region(aws_config::Region::new(region.to_string()))
                .load()
                .await
        } else {
            aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await
        };

        let client = BedrockClient::new(&config).await;
        Ok(Arc::new(BedrockProvider::new(client)))
    }

    /// Create an OpenAI provider directly
    pub fn create_openai(api_key: impl Into<String>) -> Arc<dyn LlmProvider> {
        Arc::new(OpenAiProvider::new(HttpClient::new(), api_key))
    }

    /// Create an OpenAI provider with custom base URL
    pub fn create_openai_with_base_url(
        api_key: impl Into<String>,
        base_url: impl Into<String>,
    ) -> Arc<dyn LlmProvider> {
        Arc::new(OpenAiProvider::with_base_url(
            HttpClient::new(),
            api_key,
            base_url,
        ))
    }

    /// Create an Anthropic provider directly
    pub fn create_anthropic(api_key: impl Into<String>) -> Arc<dyn LlmProvider> {
        Arc::new(AnthropicProvider::new(HttpClient::new(), api_key))
    }

    /// Create an Anthropic provider with custom base URL
    pub fn create_anthropic_with_base_url(
        api_key: impl Into<String>,
        base_url: impl Into<String>,
    ) -> Arc<dyn LlmProvider> {
        Arc::new(AnthropicProvider::with_base_url(
            HttpClient::new(),
            api_key,
            base_url,
        ))
    }

    /// Create an Azure OpenAI provider directly
    pub fn create_azure_openai(
        endpoint: impl Into<String>,
        api_key: impl Into<String>,
    ) -> Arc<dyn LlmProvider> {
        let config = AzureOpenAiConfig::new(endpoint, api_key);
        Arc::new(AzureOpenAiProvider::new(HttpClient::new(), config))
    }

    fn validate_credential_type(
        credential: &Credential,
        expected: &CredentialType,
    ) -> Result<(), DomainError> {
        if credential.credential_type() != expected {
            return Err(DomainError::configuration(format!(
                "Expected credential type {:?}, got {:?}",
                expected,
                credential.credential_type()
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_openai_provider() {
        let provider = LlmProviderFactory::create_openai("test-key");
        assert_eq!(provider.provider_name(), "openai");
    }

    #[test]
    fn test_create_anthropic_provider() {
        let provider = LlmProviderFactory::create_anthropic("test-key");
        assert_eq!(provider.provider_name(), "anthropic");
    }

    #[test]
    fn test_create_azure_provider() {
        let provider = LlmProviderFactory::create_azure_openai(
            "https://test.openai.azure.com",
            "test-key",
        );
        assert_eq!(provider.provider_name(), "azure_openai");
    }

    #[test]
    fn test_factory_with_credential() {
        let credential = Credential::new(CredentialType::OpenAi, "sk-test".to_string());
        let config = LlmProviderConfig::OpenAi;

        let provider = LlmProviderFactory::create(&config, &credential).unwrap();
        assert_eq!(provider.provider_name(), "openai");
    }

    #[test]
    fn test_factory_wrong_credential_type() {
        let credential = Credential::new(CredentialType::Anthropic, "sk-test".to_string());
        let config = LlmProviderConfig::OpenAi;

        let result = LlmProviderFactory::create(&config, &credential);
        assert!(result.is_err());
    }

    #[test]
    fn test_factory_bedrock_requires_async() {
        let credential = Credential::new(CredentialType::AwsBedrock, "unused".to_string());
        let config = LlmProviderConfig::AwsBedrock { region: None };

        let result = LlmProviderFactory::create(&config, &credential);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("async initialization"));
    }
}
