//! Credential Provider plugin trait
//!
//! Defines the interface for plugins that provide credential management capabilities.

use super::entity::Plugin;
use super::error::PluginError;
use crate::domain::credentials::{CredentialProvider, CredentialType};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

/// Source type for credential providers
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CredentialSourceType {
    /// Environment variables
    Environment,

    /// AWS Secrets Manager
    AwsSecretsManager,

    /// HashiCorp Vault
    Vault,

    /// File-based credentials
    File,

    /// Custom source
    Custom(String),
}

impl CredentialSourceType {
    pub fn as_str(&self) -> &str {
        match self {
            CredentialSourceType::Environment => "environment",
            CredentialSourceType::AwsSecretsManager => "aws_secrets_manager",
            CredentialSourceType::Vault => "vault",
            CredentialSourceType::File => "file",
            CredentialSourceType::Custom(name) => name,
        }
    }
}

/// Configuration for creating a credential provider instance
#[derive(Debug, Clone)]
pub struct CredentialProviderConfig {
    /// The source type for credentials
    pub source_type: CredentialSourceType,

    /// Additional parameters
    pub additional_params: HashMap<String, String>,
}

impl CredentialProviderConfig {
    pub fn new(source_type: CredentialSourceType) -> Self {
        Self {
            source_type,
            additional_params: HashMap::new(),
        }
    }

    pub fn with_param(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.additional_params.insert(key.into(), value.into());
        self
    }

    /// Get a parameter value
    pub fn get_param(&self, key: &str) -> Option<&String> {
        self.additional_params.get(key)
    }

    /// Helper for environment variable prefix
    pub fn env_prefix(&self) -> Option<&String> {
        self.get_param("env_prefix")
    }

    /// Helper for Vault address
    pub fn vault_address(&self) -> Option<&String> {
        self.get_param("vault_address")
    }

    /// Helper for Vault path
    pub fn vault_path(&self) -> Option<&String> {
        self.get_param("vault_path")
    }

    /// Helper for AWS secret name
    pub fn aws_secret_name(&self) -> Option<&String> {
        self.get_param("aws_secret_name")
    }

    /// Helper for AWS region
    pub fn aws_region(&self) -> Option<&String> {
        self.get_param("aws_region")
    }
}

/// Trait for plugins that provide credential management capabilities
#[async_trait]
pub trait CredentialProviderPlugin: Plugin {
    /// Get the source types this plugin supports
    fn supported_source_types(&self) -> Vec<CredentialSourceType>;

    /// Check if this plugin supports the given source type
    fn supports_source_type(&self, source_type: &CredentialSourceType) -> bool {
        self.supported_source_types().contains(source_type)
    }

    /// Get the credential types this plugin can provide
    fn supported_credential_types(&self) -> Vec<CredentialType>;

    /// Check if this plugin supports the given credential type
    fn supports_credential_type(&self, credential_type: &CredentialType) -> bool {
        self.supported_credential_types().contains(credential_type)
    }

    /// Create a credential provider instance with the given configuration
    ///
    /// # Arguments
    /// * `config` - Configuration containing provider settings
    ///
    /// # Returns
    /// * `Ok(Arc<dyn CredentialProvider>)` - The created provider instance
    /// * `Err(PluginError)` - If provider creation fails
    async fn create_credential_provider(
        &self,
        config: CredentialProviderConfig,
    ) -> Result<Arc<dyn CredentialProvider>, PluginError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_credential_source_type_as_str() {
        assert_eq!(CredentialSourceType::Environment.as_str(), "environment");
        assert_eq!(
            CredentialSourceType::AwsSecretsManager.as_str(),
            "aws_secrets_manager"
        );
        assert_eq!(CredentialSourceType::Vault.as_str(), "vault");
        assert_eq!(CredentialSourceType::File.as_str(), "file");
        assert_eq!(
            CredentialSourceType::Custom("my-source".to_string()).as_str(),
            "my-source"
        );
    }

    #[test]
    fn test_credential_provider_config_builder() {
        let config = CredentialProviderConfig::new(CredentialSourceType::Vault)
            .with_param("vault_address", "https://vault.example.com:8200")
            .with_param("vault_path", "secret/data/api-keys");

        assert!(matches!(config.source_type, CredentialSourceType::Vault));
        assert_eq!(
            config.vault_address(),
            Some(&"https://vault.example.com:8200".to_string())
        );
        assert_eq!(
            config.vault_path(),
            Some(&"secret/data/api-keys".to_string())
        );
    }

    #[test]
    fn test_credential_provider_config_aws() {
        let config = CredentialProviderConfig::new(CredentialSourceType::AwsSecretsManager)
            .with_param("aws_secret_name", "prod/llm-gateway/keys")
            .with_param("aws_region", "us-east-1");

        assert!(matches!(
            config.source_type,
            CredentialSourceType::AwsSecretsManager
        ));
        assert_eq!(
            config.aws_secret_name(),
            Some(&"prod/llm-gateway/keys".to_string())
        );
        assert_eq!(config.aws_region(), Some(&"us-east-1".to_string()));
    }
}
