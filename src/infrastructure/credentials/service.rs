//! Credential service for managing stored credentials

use std::sync::Arc;

use crate::domain::credentials::{
    CredentialId, CredentialType, StoredCredential, StoredCredentialRepository,
};
use crate::domain::DomainError;

/// Request to create a new credential
#[derive(Debug, Clone)]
pub struct CreateCredentialRequest {
    pub id: String,
    pub name: String,
    pub credential_type: CredentialType,
    pub api_key: String,
    /// Endpoint URL (for Azure OpenAI)
    pub endpoint: Option<String>,
    /// Deployment (Azure OpenAI) or header name (HTTP API Key, e.g., "Authorization")
    pub deployment: Option<String>,
    /// Header value template for HTTP API Key credentials (e.g., "Bearer ${api-key}")
    pub header_value: Option<String>,
}

/// Request to update a credential
#[derive(Debug, Clone)]
pub struct UpdateCredentialRequest {
    pub name: Option<String>,
    pub api_key: Option<String>,
    pub endpoint: Option<Option<String>>,
    pub deployment: Option<Option<String>>,
    pub header_value: Option<Option<String>>,
    pub enabled: Option<bool>,
}

/// Trait for credential service operations (used by workflow executor)
#[async_trait::async_trait]
pub trait CredentialServiceTrait: Send + Sync + std::fmt::Debug {
    /// Get a credential by ID
    async fn get(&self, id: &str) -> Result<Option<StoredCredential>, DomainError>;
    /// List all credentials
    async fn list(&self) -> Result<Vec<StoredCredential>, DomainError>;
}

/// Service for managing stored credentials
#[derive(Debug)]
pub struct CredentialService<R: StoredCredentialRepository> {
    repository: Arc<R>,
}

impl<R: StoredCredentialRepository> CredentialService<R> {
    /// Create a new credential service
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }

    /// Create a new credential
    pub async fn create(
        &self,
        request: CreateCredentialRequest,
    ) -> Result<StoredCredential, DomainError> {
        let id = CredentialId::new(&request.id)?;

        let mut credential = StoredCredential::new(
            id,
            request.name,
            request.credential_type,
            request.api_key,
        );

        if let Some(endpoint) = request.endpoint {
            credential = credential.with_endpoint(endpoint);
        }

        if let Some(deployment) = request.deployment {
            credential = credential.with_deployment(deployment);
        }

        if let Some(header_value) = request.header_value {
            credential = credential.with_header_value(header_value);
        }

        self.repository.create(credential).await
    }

    /// Get a credential by ID
    pub async fn get(&self, id: &str) -> Result<Option<StoredCredential>, DomainError> {
        let credential_id = CredentialId::new(id)?;
        self.repository.get(&credential_id).await
    }

    /// List all credentials
    pub async fn list(&self) -> Result<Vec<StoredCredential>, DomainError> {
        self.repository.list().await
    }

    /// List credentials by provider type
    pub async fn list_by_type(
        &self,
        credential_type: &CredentialType,
    ) -> Result<Vec<StoredCredential>, DomainError> {
        self.repository.list_by_type(credential_type).await
    }

    /// Update a credential
    pub async fn update(
        &self,
        id: &str,
        request: UpdateCredentialRequest,
    ) -> Result<StoredCredential, DomainError> {
        let credential_id = CredentialId::new(id)?;

        let mut credential = self
            .repository
            .get(&credential_id)
            .await?
            .ok_or_else(|| DomainError::not_found(format!("Credential '{}' not found", id)))?;

        credential.update(
            request.name,
            request.api_key,
            request.endpoint,
            request.deployment,
            request.header_value,
            request.enabled,
        );

        self.repository.update(credential).await
    }

    /// Delete a credential
    pub async fn delete(&self, id: &str) -> Result<(), DomainError> {
        let credential_id = CredentialId::new(id)?;
        self.repository.delete(&credential_id).await
    }

    /// Check if a credential exists
    pub async fn exists(&self, id: &str) -> Result<bool, DomainError> {
        let credential_id = CredentialId::new(id)?;
        self.repository.exists(&credential_id).await
    }

    /// Validate that a credential exists and matches the expected provider type
    pub async fn validate_for_model(
        &self,
        credential_id: &str,
        expected_provider: &CredentialType,
    ) -> Result<(), DomainError> {
        let cred_id = CredentialId::new(credential_id)?;
        let credential = self
            .repository
            .get(&cred_id)
            .await?
            .ok_or_else(|| {
                DomainError::not_found(format!("Credential '{}' not found", credential_id))
            })?;

        if credential.credential_type() != expected_provider {
            return Err(DomainError::validation(format!(
                "Credential '{}' has provider '{}' but model requires '{}'",
                credential_id,
                credential.credential_type(),
                expected_provider
            )));
        }

        if !credential.is_enabled() {
            return Err(DomainError::validation(format!(
                "Credential '{}' is disabled",
                credential_id
            )));
        }

        Ok(())
    }
}

#[async_trait::async_trait]
impl<R: StoredCredentialRepository + 'static> CredentialServiceTrait for CredentialService<R> {
    async fn get(&self, id: &str) -> Result<Option<StoredCredential>, DomainError> {
        let credential_id = CredentialId::new(id)?;
        self.repository.get(&credential_id).await
    }

    async fn list(&self) -> Result<Vec<StoredCredential>, DomainError> {
        self.repository.list().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::credentials::InMemoryStoredCredentialRepository;

    fn create_service() -> CredentialService<InMemoryStoredCredentialRepository> {
        CredentialService::new(Arc::new(InMemoryStoredCredentialRepository::new()))
    }

    #[tokio::test]
    async fn test_create_credential() {
        let service = create_service();

        let request = CreateCredentialRequest {
            id: "openai-prod".to_string(),
            name: "Production OpenAI".to_string(),
            credential_type: CredentialType::OpenAi,
            api_key: "sk-test-key".to_string(),
            endpoint: None,
            deployment: None,
            header_value: None,
        };

        let created = service.create(request).await.unwrap();
        assert_eq!(created.id().as_str(), "openai-prod");
        assert_eq!(created.name(), "Production OpenAI");
    }

    #[tokio::test]
    async fn test_validate_for_model_success() {
        let service = create_service();

        let request = CreateCredentialRequest {
            id: "openai-prod".to_string(),
            name: "Production OpenAI".to_string(),
            credential_type: CredentialType::OpenAi,
            api_key: "sk-test-key".to_string(),
            endpoint: None,
            deployment: None,
            header_value: None,
        };
        service.create(request).await.unwrap();

        let result = service
            .validate_for_model("openai-prod", &CredentialType::OpenAi)
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_validate_for_model_wrong_provider() {
        let service = create_service();

        let request = CreateCredentialRequest {
            id: "openai-prod".to_string(),
            name: "Production OpenAI".to_string(),
            credential_type: CredentialType::OpenAi,
            api_key: "sk-test-key".to_string(),
            endpoint: None,
            deployment: None,
            header_value: None,
        };
        service.create(request).await.unwrap();

        let result = service
            .validate_for_model("openai-prod", &CredentialType::Anthropic)
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_validate_for_model_not_found() {
        let service = create_service();

        let result = service
            .validate_for_model("nonexistent", &CredentialType::OpenAi)
            .await;
        assert!(result.is_err());
    }
}
