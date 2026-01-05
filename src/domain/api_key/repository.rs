//! API Key repository trait

use async_trait::async_trait;
use std::fmt::Debug;

use super::entity::{ApiKey, ApiKeyId, ApiKeyStatus};
use crate::domain::DomainError;

/// Repository trait for API key storage
#[async_trait]
pub trait ApiKeyRepository: Send + Sync + Debug {
    /// Get an API key by its ID
    async fn get(&self, id: &ApiKeyId) -> Result<Option<ApiKey>, DomainError>;

    /// Get an API key by its key prefix (for lookup during authentication)
    async fn get_by_prefix(&self, prefix: &str) -> Result<Option<ApiKey>, DomainError>;

    /// Create a new API key
    async fn create(&self, api_key: ApiKey) -> Result<ApiKey, DomainError>;

    /// Update an existing API key
    async fn update(&self, api_key: &ApiKey) -> Result<ApiKey, DomainError>;

    /// Delete an API key
    async fn delete(&self, id: &ApiKeyId) -> Result<bool, DomainError>;

    /// List all API keys (optionally filtered by status)
    async fn list(&self, status: Option<ApiKeyStatus>) -> Result<Vec<ApiKey>, DomainError>;

    /// Count API keys (optionally filtered by status)
    async fn count(&self, status: Option<ApiKeyStatus>) -> Result<usize, DomainError>;

    /// Check if an API key ID exists
    async fn exists(&self, id: &ApiKeyId) -> Result<bool, DomainError> {
        Ok(self.get(id).await?.is_some())
    }

    /// Record usage of an API key
    async fn record_usage(&self, id: &ApiKeyId) -> Result<(), DomainError>;

    /// Get API keys expiring before a given timestamp
    async fn get_expiring_before(
        &self,
        before: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<ApiKey>, DomainError>;
}

#[cfg(test)]
pub mod mock {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    /// Mock API key repository for testing
    #[derive(Debug, Default)]
    pub struct MockApiKeyRepository {
        keys: Arc<RwLock<HashMap<String, ApiKey>>>,
        should_fail: Arc<RwLock<bool>>,
    }

    impl MockApiKeyRepository {
        /// Create a new mock repository
        pub fn new() -> Self {
            Self::default()
        }

        /// Set whether operations should fail
        pub async fn set_should_fail(&self, fail: bool) {
            *self.should_fail.write().await = fail;
        }

        async fn check_should_fail(&self) -> Result<(), DomainError> {
            if *self.should_fail.read().await {
                return Err(DomainError::storage("Mock repository configured to fail"));
            }
            Ok(())
        }
    }

    #[async_trait]
    impl ApiKeyRepository for MockApiKeyRepository {
        async fn get(&self, id: &ApiKeyId) -> Result<Option<ApiKey>, DomainError> {
            self.check_should_fail().await?;
            let keys = self.keys.read().await;
            Ok(keys.get(id.as_str()).cloned())
        }

        async fn get_by_prefix(&self, prefix: &str) -> Result<Option<ApiKey>, DomainError> {
            self.check_should_fail().await?;
            let keys = self.keys.read().await;
            Ok(keys.values().find(|k| k.key_prefix() == prefix).cloned())
        }

        async fn create(&self, api_key: ApiKey) -> Result<ApiKey, DomainError> {
            self.check_should_fail().await?;
            let mut keys = self.keys.write().await;
            let id = api_key.id().as_str().to_string();

            if keys.contains_key(&id) {
                return Err(DomainError::conflict(format!(
                    "API key with ID '{}' already exists",
                    id
                )));
            }

            keys.insert(id, api_key.clone());
            Ok(api_key)
        }

        async fn update(&self, api_key: &ApiKey) -> Result<ApiKey, DomainError> {
            self.check_should_fail().await?;
            let mut keys = self.keys.write().await;
            let id = api_key.id().as_str().to_string();

            if !keys.contains_key(&id) {
                return Err(DomainError::not_found(format!(
                    "API key '{}' not found",
                    id
                )));
            }

            keys.insert(id, api_key.clone());
            Ok(api_key.clone())
        }

        async fn delete(&self, id: &ApiKeyId) -> Result<bool, DomainError> {
            self.check_should_fail().await?;
            let mut keys = self.keys.write().await;
            Ok(keys.remove(id.as_str()).is_some())
        }

        async fn list(&self, status: Option<ApiKeyStatus>) -> Result<Vec<ApiKey>, DomainError> {
            self.check_should_fail().await?;
            let keys = self.keys.read().await;

            let result: Vec<ApiKey> = keys
                .values()
                .filter(|k| {
                    if let Some(s) = status {
                        k.status() == s
                    } else {
                        true
                    }
                })
                .cloned()
                .collect();

            Ok(result)
        }

        async fn count(&self, status: Option<ApiKeyStatus>) -> Result<usize, DomainError> {
            self.check_should_fail().await?;
            let keys = self.keys.read().await;

            let count = keys
                .values()
                .filter(|k| {
                    if let Some(s) = status {
                        k.status() == s
                    } else {
                        true
                    }
                })
                .count();

            Ok(count)
        }

        async fn record_usage(&self, id: &ApiKeyId) -> Result<(), DomainError> {
            self.check_should_fail().await?;
            let mut keys = self.keys.write().await;

            if let Some(key) = keys.get_mut(id.as_str()) {
                key.record_usage();
                Ok(())
            } else {
                Err(DomainError::not_found(format!("API key '{}' not found", id)))
            }
        }

        async fn get_expiring_before(
            &self,
            before: chrono::DateTime<chrono::Utc>,
        ) -> Result<Vec<ApiKey>, DomainError> {
            self.check_should_fail().await?;
            let keys = self.keys.read().await;

            let result: Vec<ApiKey> = keys
                .values()
                .filter(|k| {
                    if let Some(expires) = k.expires_at() {
                        expires < before
                    } else {
                        false
                    }
                })
                .cloned()
                .collect();

            Ok(result)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::domain::api_key::ApiKeyPermissions;

        fn create_test_key(id: &str) -> ApiKey {
            let key_id = ApiKeyId::new(id).unwrap();
            ApiKey::new(key_id, format!("Test Key {}", id), "hash", "pk_test_")
        }

        #[tokio::test]
        async fn test_create_and_get() {
            let repo = MockApiKeyRepository::new();
            let key = create_test_key("test-1");

            repo.create(key.clone()).await.unwrap();

            let retrieved = repo.get(key.id()).await.unwrap();
            assert!(retrieved.is_some());
            assert_eq!(retrieved.unwrap().name(), key.name());
        }

        #[tokio::test]
        async fn test_get_by_prefix() {
            let repo = MockApiKeyRepository::new();
            let key = create_test_key("test-1");

            repo.create(key.clone()).await.unwrap();

            let retrieved = repo.get_by_prefix("pk_test_").await.unwrap();
            assert!(retrieved.is_some());
        }

        #[tokio::test]
        async fn test_update() {
            let repo = MockApiKeyRepository::new();
            let mut key = create_test_key("test-1");

            repo.create(key.clone()).await.unwrap();

            key.set_name("Updated Name");
            repo.update(&key).await.unwrap();

            let retrieved = repo.get(key.id()).await.unwrap().unwrap();
            assert_eq!(retrieved.name(), "Updated Name");
        }

        #[tokio::test]
        async fn test_delete() {
            let repo = MockApiKeyRepository::new();
            let key = create_test_key("test-1");

            repo.create(key.clone()).await.unwrap();

            let deleted = repo.delete(key.id()).await.unwrap();
            assert!(deleted);

            let retrieved = repo.get(key.id()).await.unwrap();
            assert!(retrieved.is_none());
        }

        #[tokio::test]
        async fn test_list() {
            let repo = MockApiKeyRepository::new();

            repo.create(create_test_key("test-1")).await.unwrap();
            repo.create(create_test_key("test-2")).await.unwrap();

            let all = repo.list(None).await.unwrap();
            assert_eq!(all.len(), 2);

            let active = repo.list(Some(ApiKeyStatus::Active)).await.unwrap();
            assert_eq!(active.len(), 2);
        }

        #[tokio::test]
        async fn test_count() {
            let repo = MockApiKeyRepository::new();

            repo.create(create_test_key("test-1")).await.unwrap();
            repo.create(create_test_key("test-2")).await.unwrap();

            let count = repo.count(None).await.unwrap();
            assert_eq!(count, 2);
        }

        #[tokio::test]
        async fn test_record_usage() {
            let repo = MockApiKeyRepository::new();
            let key = create_test_key("test-1");

            repo.create(key.clone()).await.unwrap();

            repo.record_usage(key.id()).await.unwrap();

            let retrieved = repo.get(key.id()).await.unwrap().unwrap();
            assert!(retrieved.last_used_at().is_some());
        }
    }
}
