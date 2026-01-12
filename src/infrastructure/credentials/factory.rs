use serde::Deserialize;
use std::sync::Arc;
use std::time::Duration;

use super::{
    AwsSecretsCredentialProvider, CachedCredentialProvider, EnvCredentialProvider,
    VaultCredentialProvider,
};
use crate::domain::{Credential, CredentialProvider, CredentialType, DomainError};
use crate::infrastructure::credentials::aws_secrets_provider::RealSecretsManagerClient;
use crate::infrastructure::credentials::vault_provider::{HttpVaultClient, VaultConfig};

/// Provider type configuration
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ProviderConfig {
    Env {
        #[serde(default = "default_true")]
        use_defaults: bool,
    },
    AwsSecrets {
        region: Option<String>,
    },
    Vault {
        address: String,
        token: String,
        #[serde(default = "default_mount_path")]
        mount_path: String,
    },
}

fn default_true() -> bool {
    true
}

fn default_mount_path() -> String {
    "secret".to_string()
}

/// Factory for creating credential providers
#[derive(Debug)]
pub struct CredentialProviderFactory;

impl CredentialProviderFactory {
    /// Create a credential provider from configuration
    pub async fn create(config: &ProviderConfig) -> Result<Arc<dyn CredentialProvider>, DomainError> {
        match config {
            ProviderConfig::Env { use_defaults } => {
                let provider = if *use_defaults {
                    EnvCredentialProvider::default()
                } else {
                    EnvCredentialProvider::new()
                };
                Ok(Arc::new(provider))
            }

            ProviderConfig::AwsSecrets { .. } => {
                let provider = AwsSecretsCredentialProvider::<RealSecretsManagerClient>::new()
                    .await?;
                Ok(Arc::new(provider))
            }

            ProviderConfig::Vault {
                address,
                token,
                mount_path,
            } => {
                let vault_config = VaultConfig::new(address, token).with_mount_path(mount_path);
                let provider = VaultCredentialProvider::<HttpVaultClient>::new(vault_config);
                Ok(Arc::new(provider))
            }
        }
    }

    /// Create a cached credential provider
    pub async fn create_cached(
        config: &ProviderConfig,
        cache_ttl: Duration,
    ) -> Result<Arc<dyn CredentialProvider>, DomainError> {
        let inner = Self::create(config).await?;

        // We need to wrap the Arc<dyn CredentialProvider> in a newtype that implements CredentialProvider
        let cached = CachedCredentialProvider::new(ArcProvider(inner), cache_ttl);
        Ok(Arc::new(cached))
    }
}

/// Wrapper to make Arc<dyn CredentialProvider> implement CredentialProvider
#[derive(Debug)]
struct ArcProvider(Arc<dyn CredentialProvider>);

#[async_trait::async_trait]
impl CredentialProvider for ArcProvider {
    async fn get_credential(
        &self,
        credential_type: &CredentialType,
    ) -> Result<Credential, DomainError> {
        self.0.get_credential(credential_type).await
    }

    async fn supports(&self, credential_type: &CredentialType) -> bool {
        self.0.supports(credential_type).await
    }

    async fn refresh(&self, credential_type: &CredentialType) -> Result<Credential, DomainError> {
        self.0.refresh(credential_type).await
    }

    fn provider_name(&self) -> &'static str {
        self.0.provider_name()
    }
}

/// Chain of credential providers that tries each in order
#[allow(dead_code)]
#[derive(Debug)]
pub struct ChainedCredentialProvider {
    providers: Vec<Arc<dyn CredentialProvider>>,
}

impl ChainedCredentialProvider {
    #[allow(dead_code)]
    pub fn new(providers: Vec<Arc<dyn CredentialProvider>>) -> Self {
        Self { providers }
    }

    #[allow(dead_code)]
    pub fn builder() -> ChainedCredentialProviderBuilder {
        ChainedCredentialProviderBuilder::new()
    }
}

#[async_trait::async_trait]
impl CredentialProvider for ChainedCredentialProvider {
    async fn get_credential(
        &self,
        credential_type: &CredentialType,
    ) -> Result<Credential, DomainError> {
        for provider in &self.providers {
            if provider.supports(credential_type).await {
                match provider.get_credential(credential_type).await {
                    Ok(cred) => {
                        tracing::debug!(
                            provider = provider.provider_name(),
                            credential_type = %credential_type,
                            "Credential found"
                        );
                        return Ok(cred);
                    }
                    Err(e) => {
                        tracing::debug!(
                            provider = provider.provider_name(),
                            credential_type = %credential_type,
                            error = %e,
                            "Provider failed, trying next"
                        );
                        continue;
                    }
                }
            }
        }

        Err(DomainError::credential(format!(
            "No provider could supply credential for type: {}",
            credential_type
        )))
    }

    async fn supports(&self, credential_type: &CredentialType) -> bool {
        for provider in &self.providers {
            if provider.supports(credential_type).await {
                return true;
            }
        }
        false
    }

    fn provider_name(&self) -> &'static str {
        "chained"
    }
}

/// Builder for ChainedCredentialProvider
#[allow(dead_code)]
pub struct ChainedCredentialProviderBuilder {
    providers: Vec<Arc<dyn CredentialProvider>>,
}

impl ChainedCredentialProviderBuilder {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
        }
    }

    #[allow(dead_code)]
    pub fn with_provider(mut self, provider: Arc<dyn CredentialProvider>) -> Self {
        self.providers.push(provider);
        self
    }

    #[allow(dead_code)]
    pub fn build(self) -> ChainedCredentialProvider {
        ChainedCredentialProvider::new(self.providers)
    }
}

impl Default for ChainedCredentialProviderBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::credentials::mock::MockCredentialProvider;

    #[tokio::test]
    async fn test_chained_provider_uses_first_available() {
        let provider1 = Arc::new(MockCredentialProvider::new("first"));
        let provider2 = Arc::new(
            MockCredentialProvider::new("second")
                .with_credential(Credential::new(CredentialType::OpenAi, "sk-second".to_string())),
        );

        let chained = ChainedCredentialProvider::builder()
            .with_provider(provider1)
            .with_provider(provider2)
            .build();

        let cred = chained.get_credential(&CredentialType::OpenAi).await.unwrap();
        assert_eq!(cred.api_key(), "sk-second");
    }

    #[tokio::test]
    async fn test_chained_provider_prefers_first() {
        let provider1 = Arc::new(
            MockCredentialProvider::new("first")
                .with_credential(Credential::new(CredentialType::OpenAi, "sk-first".to_string())),
        );
        let provider2 = Arc::new(
            MockCredentialProvider::new("second")
                .with_credential(Credential::new(CredentialType::OpenAi, "sk-second".to_string())),
        );

        let chained = ChainedCredentialProvider::builder()
            .with_provider(provider1)
            .with_provider(provider2)
            .build();

        let cred = chained.get_credential(&CredentialType::OpenAi).await.unwrap();
        assert_eq!(cred.api_key(), "sk-first");
    }

    #[tokio::test]
    async fn test_chained_provider_no_providers() {
        let chained = ChainedCredentialProvider::builder().build();

        let result = chained.get_credential(&CredentialType::OpenAi).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_factory_creates_env_provider() {
        let config = ProviderConfig::Env { use_defaults: true };
        let provider = CredentialProviderFactory::create(&config).await.unwrap();
        assert_eq!(provider.provider_name(), "env");
    }
}
