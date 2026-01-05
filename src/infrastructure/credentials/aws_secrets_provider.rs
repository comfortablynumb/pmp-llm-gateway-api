use async_trait::async_trait;
use aws_sdk_secretsmanager::Client as SecretsManagerClient;
use std::collections::HashMap;

use crate::domain::{Credential, CredentialProvider, CredentialType, DomainError};

/// Configuration for AWS Secrets Manager credential mapping
#[derive(Debug, Clone)]
pub struct AwsSecretMapping {
    pub secret_name: String,
    pub api_key_field: String,
    pub additional_fields: HashMap<String, String>,
}

impl AwsSecretMapping {
    pub fn new(secret_name: impl Into<String>, api_key_field: impl Into<String>) -> Self {
        Self {
            secret_name: secret_name.into(),
            api_key_field: api_key_field.into(),
            additional_fields: HashMap::new(),
        }
    }

    pub fn with_field(
        mut self,
        param_name: impl Into<String>,
        secret_field: impl Into<String>,
    ) -> Self {
        self.additional_fields
            .insert(param_name.into(), secret_field.into());
        self
    }
}

/// Trait for AWS Secrets Manager client operations (for mocking)
#[async_trait]
pub trait SecretsManagerClientTrait: Send + Sync + std::fmt::Debug {
    async fn get_secret_value(&self, secret_name: &str) -> Result<String, DomainError>;
}

/// Real AWS Secrets Manager client wrapper
#[derive(Debug)]
pub struct RealSecretsManagerClient {
    client: SecretsManagerClient,
}

impl RealSecretsManagerClient {
    pub fn new(client: SecretsManagerClient) -> Self {
        Self { client }
    }
}

#[async_trait]
impl SecretsManagerClientTrait for RealSecretsManagerClient {
    async fn get_secret_value(&self, secret_name: &str) -> Result<String, DomainError> {
        let response = self
            .client
            .get_secret_value()
            .secret_id(secret_name)
            .send()
            .await
            .map_err(|e| DomainError::credential(format!("AWS Secrets Manager error: {}", e)))?;

        response
            .secret_string()
            .map(|s| s.to_string())
            .ok_or_else(|| {
                DomainError::credential("Secret does not contain a string value".to_string())
            })
    }
}

/// Credential provider that reads from AWS Secrets Manager
#[derive(Debug)]
pub struct AwsSecretsCredentialProvider<C: SecretsManagerClientTrait> {
    client: C,
    mappings: HashMap<CredentialType, AwsSecretMapping>,
}

impl AwsSecretsCredentialProvider<RealSecretsManagerClient> {
    pub async fn new() -> Result<Self, DomainError> {
        let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
        let client = SecretsManagerClient::new(&config);

        Ok(Self {
            client: RealSecretsManagerClient::new(client),
            mappings: HashMap::new(),
        })
    }
}

impl<C: SecretsManagerClientTrait> AwsSecretsCredentialProvider<C> {
    pub fn with_client(client: C) -> Self {
        Self {
            client,
            mappings: HashMap::new(),
        }
    }

    pub fn with_mapping(mut self, credential_type: CredentialType, mapping: AwsSecretMapping) -> Self {
        self.mappings.insert(credential_type, mapping);
        self
    }

    fn parse_secret(
        &self,
        secret_string: &str,
        mapping: &AwsSecretMapping,
        credential_type: &CredentialType,
    ) -> Result<Credential, DomainError> {
        let secret_data: serde_json::Value =
            serde_json::from_str(secret_string).map_err(|e| {
                DomainError::credential(format!("Failed to parse secret as JSON: {}", e))
            })?;

        let api_key = secret_data
            .get(&mapping.api_key_field)
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                DomainError::credential(format!(
                    "API key field '{}' not found in secret",
                    mapping.api_key_field
                ))
            })?;

        let mut credential = Credential::new(credential_type.clone(), api_key.to_string());

        for (param_name, secret_field) in &mapping.additional_fields {
            if let Some(value) = secret_data.get(secret_field).and_then(|v| v.as_str()) {
                credential = credential.with_param(param_name, value);
            }
        }

        Ok(credential)
    }
}

#[async_trait]
impl<C: SecretsManagerClientTrait> CredentialProvider for AwsSecretsCredentialProvider<C> {
    async fn get_credential(
        &self,
        credential_type: &CredentialType,
    ) -> Result<Credential, DomainError> {
        let mapping = self.mappings.get(credential_type).ok_or_else(|| {
            DomainError::credential(format!(
                "No AWS Secrets mapping configured for credential type: {}",
                credential_type
            ))
        })?;

        let secret_string = self.client.get_secret_value(&mapping.secret_name).await?;
        self.parse_secret(&secret_string, mapping, credential_type)
    }

    async fn supports(&self, credential_type: &CredentialType) -> bool {
        self.mappings.contains_key(credential_type)
    }

    fn provider_name(&self) -> &'static str {
        "aws_secrets"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct MockSecretsClient {
        secrets: HashMap<String, String>,
    }

    impl MockSecretsClient {
        fn new() -> Self {
            Self {
                secrets: HashMap::new(),
            }
        }

        fn with_secret(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
            self.secrets.insert(name.into(), value.into());
            self
        }
    }

    #[async_trait]
    impl SecretsManagerClientTrait for MockSecretsClient {
        async fn get_secret_value(&self, secret_name: &str) -> Result<String, DomainError> {
            self.secrets
                .get(secret_name)
                .cloned()
                .ok_or_else(|| DomainError::credential("Secret not found"))
        }
    }

    #[tokio::test]
    async fn test_aws_secrets_provider() {
        let client = MockSecretsClient::new().with_secret(
            "llm-gateway/openai",
            r#"{"api_key": "sk-test-123"}"#,
        );

        let provider = AwsSecretsCredentialProvider::with_client(client).with_mapping(
            CredentialType::OpenAi,
            AwsSecretMapping::new("llm-gateway/openai", "api_key"),
        );

        let cred = provider
            .get_credential(&CredentialType::OpenAi)
            .await
            .unwrap();

        assert_eq!(cred.api_key(), "sk-test-123");
    }

    #[tokio::test]
    async fn test_aws_secrets_with_additional_fields() {
        let client = MockSecretsClient::new().with_secret(
            "llm-gateway/azure",
            r#"{"key": "azure-key", "endpoint": "https://test.azure.com", "deployment": "gpt-4"}"#,
        );

        let provider = AwsSecretsCredentialProvider::with_client(client).with_mapping(
            CredentialType::AzureOpenAi,
            AwsSecretMapping::new("llm-gateway/azure", "key")
                .with_field("endpoint", "endpoint")
                .with_field("deployment", "deployment"),
        );

        let cred = provider
            .get_credential(&CredentialType::AzureOpenAi)
            .await
            .unwrap();

        assert_eq!(cred.api_key(), "azure-key");
        assert_eq!(
            cred.get_param("endpoint"),
            Some(&"https://test.azure.com".to_string())
        );
        assert_eq!(cred.get_param("deployment"), Some(&"gpt-4".to_string()));
    }

    #[tokio::test]
    async fn test_aws_secrets_missing_secret() {
        let client = MockSecretsClient::new();

        let provider = AwsSecretsCredentialProvider::with_client(client).with_mapping(
            CredentialType::OpenAi,
            AwsSecretMapping::new("nonexistent", "api_key"),
        );

        let result = provider.get_credential(&CredentialType::OpenAi).await;
        assert!(result.is_err());
    }
}
