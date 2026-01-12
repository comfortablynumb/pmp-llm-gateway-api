//! Storage-backed API key repository implementation

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::sync::Arc;

use crate::domain::api_key::{ApiKey, ApiKeyId, ApiKeyRepository, ApiKeyStatus};
use crate::domain::storage::Storage;
use crate::domain::DomainError;

/// Storage-backed implementation of ApiKeyRepository
#[derive(Debug)]
pub struct StorageApiKeyRepository {
    storage: Arc<dyn Storage<ApiKey>>,
}

impl StorageApiKeyRepository {
    /// Create a new storage-backed repository
    pub fn new(storage: Arc<dyn Storage<ApiKey>>) -> Self {
        Self { storage }
    }
}

#[async_trait]
impl ApiKeyRepository for StorageApiKeyRepository {
    async fn get(&self, id: &ApiKeyId) -> Result<Option<ApiKey>, DomainError> {
        self.storage.get(id).await
    }

    async fn get_by_prefix(&self, prefix: &str) -> Result<Option<ApiKey>, DomainError> {
        let all = self.storage.list().await?;
        Ok(all.into_iter().find(|k| k.key_prefix() == prefix))
    }

    async fn create(&self, api_key: ApiKey) -> Result<ApiKey, DomainError> {
        if self.storage.exists(api_key.id()).await? {
            return Err(DomainError::conflict(format!(
                "API key with ID '{}' already exists",
                api_key.id()
            )));
        }

        self.storage.create(api_key).await
    }

    async fn update(&self, api_key: &ApiKey) -> Result<ApiKey, DomainError> {
        if !self.storage.exists(api_key.id()).await? {
            return Err(DomainError::not_found(format!(
                "API key '{}' not found",
                api_key.id()
            )));
        }

        self.storage.update(api_key.clone()).await
    }

    async fn delete(&self, id: &ApiKeyId) -> Result<bool, DomainError> {
        if !self.storage.exists(id).await? {
            return Ok(false);
        }

        self.storage.delete(id).await?;
        Ok(true)
    }

    async fn list(&self, status: Option<ApiKeyStatus>) -> Result<Vec<ApiKey>, DomainError> {
        let all = self.storage.list().await?;

        if let Some(s) = status {
            Ok(all.into_iter().filter(|k| k.status() == s).collect())
        } else {
            Ok(all)
        }
    }

    async fn count(&self, status: Option<ApiKeyStatus>) -> Result<usize, DomainError> {
        let all = self.storage.list().await?;

        if let Some(s) = status {
            Ok(all.into_iter().filter(|k| k.status() == s).count())
        } else {
            Ok(all.len())
        }
    }

    async fn record_usage(&self, id: &ApiKeyId) -> Result<(), DomainError> {
        let mut key = self
            .storage
            .get(id)
            .await?
            .ok_or_else(|| DomainError::not_found(format!("API key '{}' not found", id)))?;

        key.record_usage();
        self.storage.update(key).await?;
        Ok(())
    }

    async fn get_expiring_before(
        &self,
        before: DateTime<Utc>,
    ) -> Result<Vec<ApiKey>, DomainError> {
        let all = self.storage.list().await?;

        Ok(all
            .into_iter()
            .filter(|k| {
                if let Some(expires) = k.expires_at() {
                    expires < before
                } else {
                    false
                }
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::team::TeamId;
    use crate::infrastructure::storage::InMemoryStorage;

    fn create_repo() -> StorageApiKeyRepository {
        let storage = Arc::new(InMemoryStorage::<ApiKey>::new());
        StorageApiKeyRepository::new(storage)
    }

    fn create_test_key(id: &str) -> ApiKey {
        let key_id = ApiKeyId::new(id).unwrap();
        let team_id = TeamId::administrators();
        ApiKey::new(key_id, format!("Test Key {}", id), "hash", "pk_test_", team_id)
    }

    #[tokio::test]
    async fn test_create_and_get() {
        let repo = create_repo();
        let key = create_test_key("test-1");

        let created = repo.create(key.clone()).await.unwrap();
        assert_eq!(created.id().as_str(), "test-1");

        let fetched = repo.get(&ApiKeyId::new("test-1").unwrap()).await.unwrap();
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().name(), "Test Key test-1");
    }

    #[tokio::test]
    async fn test_create_duplicate() {
        let repo = create_repo();
        let key = create_test_key("test-1");

        repo.create(key.clone()).await.unwrap();
        let result = repo.create(key).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }

    #[tokio::test]
    async fn test_get_by_prefix() {
        let repo = create_repo();
        let key = create_test_key("test-1");

        repo.create(key).await.unwrap();

        let fetched = repo.get_by_prefix("pk_test_").await.unwrap();
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().id().as_str(), "test-1");
    }

    #[tokio::test]
    async fn test_list_by_status() {
        let repo = create_repo();

        repo.create(create_test_key("test-1")).await.unwrap();
        repo.create(create_test_key("test-2")).await.unwrap();

        let all = repo.list(None).await.unwrap();
        assert_eq!(all.len(), 2);

        let active = repo.list(Some(ApiKeyStatus::Active)).await.unwrap();
        assert_eq!(active.len(), 2);
    }

    #[tokio::test]
    async fn test_update() {
        let repo = create_repo();
        let mut key = create_test_key("test-1");
        repo.create(key.clone()).await.unwrap();

        key.set_name("Updated Name");
        repo.update(&key).await.unwrap();

        let fetched = repo.get(&ApiKeyId::new("test-1").unwrap()).await.unwrap().unwrap();
        assert_eq!(fetched.name(), "Updated Name");
    }

    #[tokio::test]
    async fn test_delete() {
        let repo = create_repo();
        let key = create_test_key("test-1");

        repo.create(key.clone()).await.unwrap();

        let id = ApiKeyId::new("test-1").unwrap();
        assert!(repo.delete(&id).await.unwrap());
        assert!(repo.get(&id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_count() {
        let repo = create_repo();

        repo.create(create_test_key("test-1")).await.unwrap();
        repo.create(create_test_key("test-2")).await.unwrap();

        let count = repo.count(None).await.unwrap();
        assert_eq!(count, 2);
    }

    #[tokio::test]
    async fn test_record_usage() {
        let repo = create_repo();
        let key = create_test_key("test-1");
        repo.create(key.clone()).await.unwrap();

        repo.record_usage(key.id()).await.unwrap();

        let fetched = repo.get(key.id()).await.unwrap().unwrap();
        assert!(fetched.last_used_at().is_some());
    }
}
