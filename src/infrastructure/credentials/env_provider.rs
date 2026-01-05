use async_trait::async_trait;
use std::collections::HashMap;
use std::env;

use crate::domain::{Credential, CredentialProvider, CredentialType, DomainError};

/// Environment variable mappings for each credential type
#[derive(Debug, Clone)]
pub struct EnvMapping {
    pub api_key_var: String,
    pub additional_vars: HashMap<String, String>,
}

impl EnvMapping {
    pub fn new(api_key_var: impl Into<String>) -> Self {
        Self {
            api_key_var: api_key_var.into(),
            additional_vars: HashMap::new(),
        }
    }

    pub fn with_var(mut self, param_name: impl Into<String>, env_var: impl Into<String>) -> Self {
        self.additional_vars.insert(param_name.into(), env_var.into());
        self
    }
}

/// Credential provider that reads from environment variables
#[derive(Debug)]
pub struct EnvCredentialProvider {
    mappings: HashMap<CredentialType, EnvMapping>,
}

impl EnvCredentialProvider {
    pub fn new() -> Self {
        Self {
            mappings: HashMap::new(),
        }
    }

    pub fn with_mapping(mut self, credential_type: CredentialType, mapping: EnvMapping) -> Self {
        self.mappings.insert(credential_type, mapping);
        self
    }

    pub fn with_defaults(mut self) -> Self {
        self.mappings.insert(
            CredentialType::OpenAi,
            EnvMapping::new("OPENAI_API_KEY"),
        );

        self.mappings.insert(
            CredentialType::Anthropic,
            EnvMapping::new("ANTHROPIC_API_KEY"),
        );

        self.mappings.insert(
            CredentialType::AzureOpenAi,
            EnvMapping::new("AZURE_OPENAI_API_KEY")
                .with_var("endpoint", "AZURE_OPENAI_ENDPOINT")
                .with_var("deployment", "AZURE_OPENAI_DEPLOYMENT")
                .with_var("api_version", "AZURE_OPENAI_API_VERSION"),
        );

        self.mappings.insert(
            CredentialType::AwsBedrock,
            EnvMapping::new("AWS_ACCESS_KEY_ID")
                .with_var("secret_key", "AWS_SECRET_ACCESS_KEY")
                .with_var("region", "AWS_REGION")
                .with_var("session_token", "AWS_SESSION_TOKEN"),
        );

        self
    }

    fn read_credential(&self, credential_type: &CredentialType) -> Result<Credential, DomainError> {
        let mapping = self.mappings.get(credential_type).ok_or_else(|| {
            DomainError::credential(format!(
                "No environment mapping configured for credential type: {}",
                credential_type
            ))
        })?;

        let api_key = env::var(&mapping.api_key_var).map_err(|_| {
            DomainError::credential(format!(
                "Environment variable '{}' not set for credential type: {}",
                mapping.api_key_var, credential_type
            ))
        })?;

        let mut credential = Credential::new(credential_type.clone(), api_key);

        for (param_name, env_var) in &mapping.additional_vars {
            if let Ok(value) = env::var(env_var) {
                credential = credential.with_param(param_name, value);
            }
        }

        Ok(credential)
    }
}

impl Default for EnvCredentialProvider {
    fn default() -> Self {
        Self::new().with_defaults()
    }
}

#[async_trait]
impl CredentialProvider for EnvCredentialProvider {
    async fn get_credential(
        &self,
        credential_type: &CredentialType,
    ) -> Result<Credential, DomainError> {
        self.read_credential(credential_type)
    }

    async fn supports(&self, credential_type: &CredentialType) -> bool {
        if !self.mappings.contains_key(credential_type) {
            return false;
        }

        let mapping = &self.mappings[credential_type];
        env::var(&mapping.api_key_var).is_ok()
    }

    fn provider_name(&self) -> &'static str {
        "env"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[tokio::test]
    async fn test_env_provider_with_set_variable() {
        // SAFETY: Test runs in isolation
        unsafe { env::set_var("TEST_OPENAI_KEY", "sk-test-123") };

        let provider = EnvCredentialProvider::new()
            .with_mapping(CredentialType::OpenAi, EnvMapping::new("TEST_OPENAI_KEY"));

        let cred = provider.get_credential(&CredentialType::OpenAi).await.unwrap();
        assert_eq!(cred.api_key(), "sk-test-123");

        // SAFETY: Test cleanup
        unsafe { env::remove_var("TEST_OPENAI_KEY") };
    }

    #[tokio::test]
    async fn test_env_provider_missing_variable() {
        let provider = EnvCredentialProvider::new()
            .with_mapping(CredentialType::OpenAi, EnvMapping::new("NONEXISTENT_VAR_12345"));

        let result = provider.get_credential(&CredentialType::OpenAi).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_env_provider_with_additional_params() {
        // SAFETY: Test runs in isolation
        unsafe {
            env::set_var("TEST_AZURE_KEY", "azure-key");
            env::set_var("TEST_AZURE_ENDPOINT", "https://test.openai.azure.com");
        }

        let provider = EnvCredentialProvider::new().with_mapping(
            CredentialType::AzureOpenAi,
            EnvMapping::new("TEST_AZURE_KEY").with_var("endpoint", "TEST_AZURE_ENDPOINT"),
        );

        let cred = provider
            .get_credential(&CredentialType::AzureOpenAi)
            .await
            .unwrap();

        assert_eq!(cred.api_key(), "azure-key");
        assert_eq!(
            cred.get_param("endpoint"),
            Some(&"https://test.openai.azure.com".to_string())
        );

        // SAFETY: Test cleanup
        unsafe {
            env::remove_var("TEST_AZURE_KEY");
            env::remove_var("TEST_AZURE_ENDPOINT");
        }
    }

    #[tokio::test]
    async fn test_supports_returns_false_for_missing_var() {
        let provider = EnvCredentialProvider::new()
            .with_mapping(CredentialType::OpenAi, EnvMapping::new("NONEXISTENT_VAR_67890"));

        assert!(!provider.supports(&CredentialType::OpenAi).await);
    }
}
