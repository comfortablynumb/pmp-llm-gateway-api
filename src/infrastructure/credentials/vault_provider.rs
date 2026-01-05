use async_trait::async_trait;
use serde::Deserialize;
use std::collections::HashMap;

use crate::domain::{Credential, CredentialProvider, CredentialType, DomainError};

/// Configuration for Vault secret mapping
#[derive(Debug, Clone)]
pub struct VaultSecretMapping {
    pub path: String,
    pub api_key_field: String,
    pub additional_fields: HashMap<String, String>,
}

impl VaultSecretMapping {
    pub fn new(path: impl Into<String>, api_key_field: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            api_key_field: api_key_field.into(),
            additional_fields: HashMap::new(),
        }
    }

    pub fn with_field(
        mut self,
        param_name: impl Into<String>,
        vault_field: impl Into<String>,
    ) -> Self {
        self.additional_fields
            .insert(param_name.into(), vault_field.into());
        self
    }
}

/// Vault client configuration
#[derive(Debug, Clone)]
pub struct VaultConfig {
    pub address: String,
    pub token: String,
    pub mount_path: String,
}

impl VaultConfig {
    pub fn new(address: impl Into<String>, token: impl Into<String>) -> Self {
        Self {
            address: address.into(),
            token: token.into(),
            mount_path: "secret".to_string(),
        }
    }

    pub fn with_mount_path(mut self, mount_path: impl Into<String>) -> Self {
        self.mount_path = mount_path.into();
        self
    }
}

/// Trait for Vault client operations (for mocking)
#[async_trait]
pub trait VaultClientTrait: Send + Sync + std::fmt::Debug {
    async fn read_secret(&self, path: &str) -> Result<HashMap<String, String>, DomainError>;
}

/// Real Vault HTTP client
#[derive(Debug)]
pub struct HttpVaultClient {
    config: VaultConfig,
    http_client: reqwest::Client,
}

impl HttpVaultClient {
    pub fn new(config: VaultConfig) -> Self {
        Self {
            config,
            http_client: reqwest::Client::new(),
        }
    }
}

#[derive(Deserialize)]
struct VaultResponse {
    data: VaultData,
}

#[derive(Deserialize)]
struct VaultData {
    data: HashMap<String, serde_json::Value>,
}

#[async_trait]
impl VaultClientTrait for HttpVaultClient {
    async fn read_secret(&self, path: &str) -> Result<HashMap<String, String>, DomainError> {
        let url = format!(
            "{}/v1/{}/data/{}",
            self.config.address, self.config.mount_path, path
        );

        let response = self
            .http_client
            .get(&url)
            .header("X-Vault-Token", &self.config.token)
            .send()
            .await
            .map_err(|e| DomainError::credential(format!("Vault request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(DomainError::credential(format!(
                "Vault returned error status: {}",
                response.status()
            )));
        }

        let vault_response: VaultResponse = response.json().await.map_err(|e| {
            DomainError::credential(format!("Failed to parse Vault response: {}", e))
        })?;

        let data = vault_response
            .data
            .data
            .into_iter()
            .filter_map(|(k, v)| v.as_str().map(|s| (k, s.to_string())))
            .collect();

        Ok(data)
    }
}

/// Credential provider that reads from HashiCorp Vault
#[derive(Debug)]
pub struct VaultCredentialProvider<C: VaultClientTrait> {
    client: C,
    mappings: HashMap<CredentialType, VaultSecretMapping>,
}

impl VaultCredentialProvider<HttpVaultClient> {
    pub fn new(config: VaultConfig) -> Self {
        Self {
            client: HttpVaultClient::new(config),
            mappings: HashMap::new(),
        }
    }
}

impl<C: VaultClientTrait> VaultCredentialProvider<C> {
    pub fn with_client(client: C) -> Self {
        Self {
            client,
            mappings: HashMap::new(),
        }
    }

    pub fn with_mapping(
        mut self,
        credential_type: CredentialType,
        mapping: VaultSecretMapping,
    ) -> Self {
        self.mappings.insert(credential_type, mapping);
        self
    }
}

#[async_trait]
impl<C: VaultClientTrait> CredentialProvider for VaultCredentialProvider<C> {
    async fn get_credential(
        &self,
        credential_type: &CredentialType,
    ) -> Result<Credential, DomainError> {
        let mapping = self.mappings.get(credential_type).ok_or_else(|| {
            DomainError::credential(format!(
                "No Vault mapping configured for credential type: {}",
                credential_type
            ))
        })?;

        let secret_data = self.client.read_secret(&mapping.path).await?;

        let api_key = secret_data.get(&mapping.api_key_field).ok_or_else(|| {
            DomainError::credential(format!(
                "API key field '{}' not found in Vault secret",
                mapping.api_key_field
            ))
        })?;

        let mut credential = Credential::new(credential_type.clone(), api_key.clone());

        for (param_name, vault_field) in &mapping.additional_fields {
            if let Some(value) = secret_data.get(vault_field) {
                credential = credential.with_param(param_name, value);
            }
        }

        Ok(credential)
    }

    async fn supports(&self, credential_type: &CredentialType) -> bool {
        self.mappings.contains_key(credential_type)
    }

    fn provider_name(&self) -> &'static str {
        "vault"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct MockVaultClient {
        secrets: HashMap<String, HashMap<String, String>>,
    }

    impl MockVaultClient {
        fn new() -> Self {
            Self {
                secrets: HashMap::new(),
            }
        }

        fn with_secret(mut self, path: impl Into<String>, data: HashMap<String, String>) -> Self {
            self.secrets.insert(path.into(), data);
            self
        }
    }

    #[async_trait]
    impl VaultClientTrait for MockVaultClient {
        async fn read_secret(&self, path: &str) -> Result<HashMap<String, String>, DomainError> {
            self.secrets
                .get(path)
                .cloned()
                .ok_or_else(|| DomainError::credential("Secret not found in Vault"))
        }
    }

    #[tokio::test]
    async fn test_vault_provider() {
        let mut secret_data = HashMap::new();
        secret_data.insert("api_key".to_string(), "sk-vault-123".to_string());

        let client = MockVaultClient::new().with_secret("llm/openai", secret_data);

        let provider = VaultCredentialProvider::with_client(client)
            .with_mapping(
                CredentialType::OpenAi,
                VaultSecretMapping::new("llm/openai", "api_key"),
            );

        let cred = provider
            .get_credential(&CredentialType::OpenAi)
            .await
            .unwrap();

        assert_eq!(cred.api_key(), "sk-vault-123");
    }

    #[tokio::test]
    async fn test_vault_provider_with_additional_fields() {
        let mut secret_data = HashMap::new();
        secret_data.insert("key".to_string(), "azure-key".to_string());
        secret_data.insert("endpoint".to_string(), "https://vault.azure.com".to_string());

        let client = MockVaultClient::new().with_secret("llm/azure", secret_data);

        let provider = VaultCredentialProvider::with_client(client).with_mapping(
            CredentialType::AzureOpenAi,
            VaultSecretMapping::new("llm/azure", "key").with_field("endpoint", "endpoint"),
        );

        let cred = provider
            .get_credential(&CredentialType::AzureOpenAi)
            .await
            .unwrap();

        assert_eq!(cred.api_key(), "azure-key");
        assert_eq!(
            cred.get_param("endpoint"),
            Some(&"https://vault.azure.com".to_string())
        );
    }

    #[tokio::test]
    async fn test_vault_provider_missing_secret() {
        let client = MockVaultClient::new();

        let provider = VaultCredentialProvider::with_client(client)
            .with_mapping(
                CredentialType::OpenAi,
                VaultSecretMapping::new("nonexistent", "api_key"),
            );

        let result = provider.get_credential(&CredentialType::OpenAi).await;
        assert!(result.is_err());
    }
}
