//! Storage-backed stored credential repository implementation

use async_trait::async_trait;
use std::sync::Arc;

use crate::domain::credentials::{
    CredentialId, CredentialType, StoredCredential, StoredCredentialRepository,
};
use crate::domain::storage::Storage;
use crate::domain::DomainError;

/// Storage-backed implementation of StoredCredentialRepository
#[derive(Debug)]
pub struct StorageStoredCredentialRepository {
    storage: Arc<dyn Storage<StoredCredential>>,
}

impl StorageStoredCredentialRepository {
    /// Create a new storage-backed repository
    pub fn new(storage: Arc<dyn Storage<StoredCredential>>) -> Self {
        Self { storage }
    }
}

#[async_trait]
impl StoredCredentialRepository for StorageStoredCredentialRepository {
    async fn get(&self, id: &CredentialId) -> Result<Option<StoredCredential>, DomainError> {
        self.storage.get(id).await
    }

    async fn list(&self) -> Result<Vec<StoredCredential>, DomainError> {
        self.storage.list().await
    }

    async fn list_by_type(
        &self,
        credential_type: &CredentialType,
    ) -> Result<Vec<StoredCredential>, DomainError> {
        let all = self.storage.list().await?;
        Ok(all
            .into_iter()
            .filter(|c| c.credential_type() == credential_type)
            .collect())
    }

    async fn create(&self, credential: StoredCredential) -> Result<StoredCredential, DomainError> {
        if self.storage.exists(credential.id()).await? {
            return Err(DomainError::conflict(format!(
                "Credential with ID '{}' already exists",
                credential.id()
            )));
        }

        self.storage.create(credential).await
    }

    async fn update(&self, credential: StoredCredential) -> Result<StoredCredential, DomainError> {
        if !self.storage.exists(credential.id()).await? {
            return Err(DomainError::not_found(format!(
                "Credential with ID '{}' not found",
                credential.id()
            )));
        }

        self.storage.update(credential).await
    }

    async fn delete(&self, id: &CredentialId) -> Result<(), DomainError> {
        if !self.storage.exists(id).await? {
            return Err(DomainError::not_found(format!(
                "Credential with ID '{}' not found",
                id.as_str()
            )));
        }

        self.storage.delete(id).await?;
        Ok(())
    }

    async fn exists(&self, id: &CredentialId) -> Result<bool, DomainError> {
        self.storage.exists(id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::storage::InMemoryStorage;

    fn create_repo() -> StorageStoredCredentialRepository {
        let storage = Arc::new(InMemoryStorage::<StoredCredential>::new());
        StorageStoredCredentialRepository::new(storage)
    }

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
        let repo = create_repo();
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
        let repo = create_repo();
        let cred = create_test_credential("openai-1", CredentialType::OpenAi);

        repo.create(cred.clone()).await.unwrap();
        let result = repo.create(cred).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }

    #[tokio::test]
    async fn test_list_by_type() {
        let repo = create_repo();

        repo.create(create_test_credential("openai-1", CredentialType::OpenAi))
            .await
            .unwrap();
        repo.create(create_test_credential("openai-2", CredentialType::OpenAi))
            .await
            .unwrap();
        repo.create(create_test_credential(
            "anthropic-1",
            CredentialType::Anthropic,
        ))
        .await
        .unwrap();

        let openai_creds = repo.list_by_type(&CredentialType::OpenAi).await.unwrap();
        assert_eq!(openai_creds.len(), 2);

        let anthropic_creds = repo.list_by_type(&CredentialType::Anthropic).await.unwrap();
        assert_eq!(anthropic_creds.len(), 1);
    }

    #[tokio::test]
    async fn test_update() {
        let repo = create_repo();
        let cred = create_test_credential("openai-1", CredentialType::OpenAi);
        repo.create(cred).await.unwrap();

        let mut updated = repo
            .get(&CredentialId::new("openai-1").unwrap())
            .await
            .unwrap()
            .unwrap();
        updated.update(Some("Updated Name".to_string()), None, None, None, None, None);

        repo.update(updated).await.unwrap();

        let fetched = repo
            .get(&CredentialId::new("openai-1").unwrap())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(fetched.name(), "Updated Name");
    }

    #[tokio::test]
    async fn test_delete() {
        let repo = create_repo();
        let cred = create_test_credential("openai-1", CredentialType::OpenAi);

        repo.create(cred).await.unwrap();

        let id = CredentialId::new("openai-1").unwrap();
        assert!(repo.exists(&id).await.unwrap());

        repo.delete(&id).await.unwrap();
        assert!(!repo.exists(&id).await.unwrap());
    }

    #[tokio::test]
    async fn test_delete_not_found() {
        let repo = create_repo();
        let id = CredentialId::new("nonexistent").unwrap();

        let result = repo.delete(&id).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }
}
