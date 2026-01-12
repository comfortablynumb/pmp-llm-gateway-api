//! In-memory stored credential repository implementation

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::domain::credentials::{
    CredentialId, CredentialType, StoredCredential, StoredCredentialRepository,
};
use crate::domain::DomainError;

/// In-memory implementation of StoredCredentialRepository
#[derive(Debug)]
pub struct InMemoryStoredCredentialRepository {
    credentials: Arc<RwLock<HashMap<String, StoredCredential>>>,
}

impl InMemoryStoredCredentialRepository {
    /// Create a new empty repository
    pub fn new() -> Self {
        Self {
            credentials: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a repository with initial credentials
    pub fn with_credentials(credentials: Vec<StoredCredential>) -> Self {
        let mut map = HashMap::new();

        for cred in credentials {
            map.insert(cred.id().as_str().to_string(), cred);
        }

        Self {
            credentials: Arc::new(RwLock::new(map)),
        }
    }
}

impl Default for InMemoryStoredCredentialRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl StoredCredentialRepository for InMemoryStoredCredentialRepository {
    async fn get(&self, id: &CredentialId) -> Result<Option<StoredCredential>, DomainError> {
        let credentials = self.credentials.read().await;
        Ok(credentials.get(id.as_str()).cloned())
    }

    async fn list(&self) -> Result<Vec<StoredCredential>, DomainError> {
        let credentials = self.credentials.read().await;
        Ok(credentials.values().cloned().collect())
    }

    async fn list_by_type(
        &self,
        credential_type: &CredentialType,
    ) -> Result<Vec<StoredCredential>, DomainError> {
        let credentials = self.credentials.read().await;
        Ok(credentials
            .values()
            .filter(|c| c.credential_type() == credential_type)
            .cloned()
            .collect())
    }

    async fn create(&self, credential: StoredCredential) -> Result<StoredCredential, DomainError> {
        let mut credentials = self.credentials.write().await;
        let id = credential.id().as_str().to_string();

        if credentials.contains_key(&id) {
            return Err(DomainError::conflict(format!(
                "Credential with ID '{}' already exists",
                id
            )));
        }

        credentials.insert(id, credential.clone());
        Ok(credential)
    }

    async fn update(&self, credential: StoredCredential) -> Result<StoredCredential, DomainError> {
        let mut credentials = self.credentials.write().await;
        let id = credential.id().as_str().to_string();

        if !credentials.contains_key(&id) {
            return Err(DomainError::not_found(format!(
                "Credential with ID '{}' not found",
                id
            )));
        }

        credentials.insert(id, credential.clone());
        Ok(credential)
    }

    async fn delete(&self, id: &CredentialId) -> Result<(), DomainError> {
        let mut credentials = self.credentials.write().await;

        if credentials.remove(id.as_str()).is_none() {
            return Err(DomainError::not_found(format!(
                "Credential with ID '{}' not found",
                id.as_str()
            )));
        }

        Ok(())
    }

    async fn exists(&self, id: &CredentialId) -> Result<bool, DomainError> {
        let credentials = self.credentials.read().await;
        Ok(credentials.contains_key(id.as_str()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_credential(id: &str, cred_type: CredentialType) -> StoredCredential {
        StoredCredential::new(
            CredentialId::new(id).unwrap(),
            format!("Test {}", id),
            cred_type,
            "test-api-key",
        )
    }

    #[tokio::test]
    async fn test_create_and_get() {
        let repo = InMemoryStoredCredentialRepository::new();
        let cred = create_test_credential("openai-1", CredentialType::OpenAi);

        let created = repo.create(cred.clone()).await.unwrap();
        assert_eq!(created.id().as_str(), "openai-1");

        let fetched = repo
            .get(&CredentialId::new("openai-1").unwrap())
            .await
            .unwrap();
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().name(), "Test openai-1");
    }

    #[tokio::test]
    async fn test_create_duplicate() {
        let repo = InMemoryStoredCredentialRepository::new();
        let cred = create_test_credential("openai-1", CredentialType::OpenAi);

        repo.create(cred.clone()).await.unwrap();
        let result = repo.create(cred).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list_by_type() {
        let repo = InMemoryStoredCredentialRepository::new();

        repo.create(create_test_credential("openai-1", CredentialType::OpenAi))
            .await
            .unwrap();
        repo.create(create_test_credential("openai-2", CredentialType::OpenAi))
            .await
            .unwrap();
        repo.create(create_test_credential("anthropic-1", CredentialType::Anthropic))
            .await
            .unwrap();

        let openai_creds = repo.list_by_type(&CredentialType::OpenAi).await.unwrap();
        assert_eq!(openai_creds.len(), 2);

        let anthropic_creds = repo.list_by_type(&CredentialType::Anthropic).await.unwrap();
        assert_eq!(anthropic_creds.len(), 1);
    }

    #[tokio::test]
    async fn test_delete() {
        let repo = InMemoryStoredCredentialRepository::new();
        let cred = create_test_credential("openai-1", CredentialType::OpenAi);

        repo.create(cred).await.unwrap();

        let id = CredentialId::new("openai-1").unwrap();
        assert!(repo.exists(&id).await.unwrap());

        repo.delete(&id).await.unwrap();
        assert!(!repo.exists(&id).await.unwrap());
    }
}
