//! In-memory API key repository implementation

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::domain::api_key::{ApiKey, ApiKeyId, ApiKeyRepository, ApiKeyStatus};
use crate::domain::DomainError;

/// In-memory implementation of ApiKeyRepository
#[derive(Debug)]
pub struct InMemoryApiKeyRepository {
    keys: Arc<RwLock<HashMap<String, ApiKey>>>,
    prefix_index: Arc<RwLock<HashMap<String, String>>>,
}

impl InMemoryApiKeyRepository {
    /// Create a new in-memory repository
    pub fn new() -> Self {
        Self {
            keys: Arc::new(RwLock::new(HashMap::new())),
            prefix_index: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a repository with initial keys
    pub fn with_keys(keys: Vec<ApiKey>) -> Self {
        let repo = Self::new();
        let keys_map: HashMap<String, ApiKey> = keys
            .iter()
            .map(|k| (k.id().as_str().to_string(), k.clone()))
            .collect();
        let prefix_map: HashMap<String, String> = keys
            .iter()
            .map(|k| (k.key_prefix().to_string(), k.id().as_str().to_string()))
            .collect();

        *futures::executor::block_on(repo.keys.write()) = keys_map;
        *futures::executor::block_on(repo.prefix_index.write()) = prefix_map;

        repo
    }
}

impl Default for InMemoryApiKeyRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ApiKeyRepository for InMemoryApiKeyRepository {
    async fn get(&self, id: &ApiKeyId) -> Result<Option<ApiKey>, DomainError> {
        let keys = self.keys.read().await;
        Ok(keys.get(id.as_str()).cloned())
    }

    async fn get_by_prefix(&self, prefix: &str) -> Result<Option<ApiKey>, DomainError> {
        let prefix_index = self.prefix_index.read().await;

        if let Some(key_id) = prefix_index.get(prefix) {
            let keys = self.keys.read().await;
            Ok(keys.get(key_id).cloned())
        } else {
            Ok(None)
        }
    }

    async fn create(&self, api_key: ApiKey) -> Result<ApiKey, DomainError> {
        let mut keys = self.keys.write().await;
        let mut prefix_index = self.prefix_index.write().await;

        let id = api_key.id().as_str().to_string();
        let prefix = api_key.key_prefix().to_string();

        if keys.contains_key(&id) {
            return Err(DomainError::conflict(format!(
                "API key with ID '{}' already exists",
                id
            )));
        }

        if prefix_index.contains_key(&prefix) {
            return Err(DomainError::conflict(format!(
                "API key with prefix '{}' already exists",
                prefix
            )));
        }

        keys.insert(id.clone(), api_key.clone());
        prefix_index.insert(prefix, id);

        Ok(api_key)
    }

    async fn update(&self, api_key: &ApiKey) -> Result<ApiKey, DomainError> {
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
        let mut keys = self.keys.write().await;
        let mut prefix_index = self.prefix_index.write().await;

        if let Some(key) = keys.remove(id.as_str()) {
            prefix_index.remove(key.key_prefix());
            Ok(true)
        } else {
            Ok(false)
        }
    }

    async fn list(&self, status: Option<ApiKeyStatus>) -> Result<Vec<ApiKey>, DomainError> {
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
        let mut keys = self.keys.write().await;

        if let Some(key) = keys.get_mut(id.as_str()) {
            key.record_usage();
            Ok(())
        } else {
            Err(DomainError::not_found(format!(
                "API key '{}' not found",
                id
            )))
        }
    }

    async fn get_expiring_before(
        &self,
        before: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<ApiKey>, DomainError> {
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
    use crate::domain::team::TeamId;
    use chrono::{Duration, Utc};

    fn create_test_key(id: &str, prefix: &str) -> ApiKey {
        let key_id = ApiKeyId::new(id).unwrap();
        let team_id = TeamId::administrators();
        ApiKey::new(key_id, format!("Test Key {}", id), "hash", prefix, team_id)
    }

    #[tokio::test]
    async fn test_create_and_get() {
        let repo = InMemoryApiKeyRepository::new();
        let key = create_test_key("test-1", "pk_test_1_");

        repo.create(key.clone()).await.unwrap();

        let retrieved = repo.get(key.id()).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name(), key.name());
    }

    #[tokio::test]
    async fn test_get_by_prefix() {
        let repo = InMemoryApiKeyRepository::new();
        let key = create_test_key("test-1", "pk_unique_");

        repo.create(key.clone()).await.unwrap();

        let retrieved = repo.get_by_prefix("pk_unique_").await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id().as_str(), "test-1");
    }

    #[tokio::test]
    async fn test_create_duplicate_id() {
        let repo = InMemoryApiKeyRepository::new();
        let key1 = create_test_key("test-1", "pk_1_");
        let key2 = create_test_key("test-1", "pk_2_");

        repo.create(key1).await.unwrap();
        let result = repo.create(key2).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_create_duplicate_prefix() {
        let repo = InMemoryApiKeyRepository::new();
        let key1 = create_test_key("test-1", "pk_same_");
        let key2 = create_test_key("test-2", "pk_same_");

        repo.create(key1).await.unwrap();
        let result = repo.create(key2).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_update() {
        let repo = InMemoryApiKeyRepository::new();
        let mut key = create_test_key("test-1", "pk_test_");

        repo.create(key.clone()).await.unwrap();

        key.set_name("Updated Name");
        repo.update(&key).await.unwrap();

        let retrieved = repo.get(key.id()).await.unwrap().unwrap();
        assert_eq!(retrieved.name(), "Updated Name");
    }

    #[tokio::test]
    async fn test_delete() {
        let repo = InMemoryApiKeyRepository::new();
        let key = create_test_key("test-1", "pk_test_");

        repo.create(key.clone()).await.unwrap();

        let deleted = repo.delete(key.id()).await.unwrap();
        assert!(deleted);

        let retrieved = repo.get(key.id()).await.unwrap();
        assert!(retrieved.is_none());

        // Prefix should also be removed
        let by_prefix = repo.get_by_prefix("pk_test_").await.unwrap();
        assert!(by_prefix.is_none());
    }

    #[tokio::test]
    async fn test_list_all() {
        let repo = InMemoryApiKeyRepository::new();

        repo.create(create_test_key("test-1", "pk_1_")).await.unwrap();
        repo.create(create_test_key("test-2", "pk_2_")).await.unwrap();

        let all = repo.list(None).await.unwrap();
        assert_eq!(all.len(), 2);
    }

    #[tokio::test]
    async fn test_list_by_status() {
        let repo = InMemoryApiKeyRepository::new();

        let mut key1 = create_test_key("test-1", "pk_1_");
        let key2 = create_test_key("test-2", "pk_2_");

        repo.create(key1.clone()).await.unwrap();
        repo.create(key2).await.unwrap();

        key1.suspend();
        repo.update(&key1).await.unwrap();

        let active = repo.list(Some(ApiKeyStatus::Active)).await.unwrap();
        assert_eq!(active.len(), 1);

        let suspended = repo.list(Some(ApiKeyStatus::Suspended)).await.unwrap();
        assert_eq!(suspended.len(), 1);
    }

    #[tokio::test]
    async fn test_count() {
        let repo = InMemoryApiKeyRepository::new();

        repo.create(create_test_key("test-1", "pk_1_")).await.unwrap();
        repo.create(create_test_key("test-2", "pk_2_")).await.unwrap();

        let count = repo.count(None).await.unwrap();
        assert_eq!(count, 2);
    }

    #[tokio::test]
    async fn test_record_usage() {
        let repo = InMemoryApiKeyRepository::new();
        let key = create_test_key("test-1", "pk_test_");

        repo.create(key.clone()).await.unwrap();

        assert!(repo.get(key.id()).await.unwrap().unwrap().last_used_at().is_none());

        repo.record_usage(key.id()).await.unwrap();

        assert!(repo.get(key.id()).await.unwrap().unwrap().last_used_at().is_some());
    }

    #[tokio::test]
    async fn test_get_expiring_before() {
        let repo = InMemoryApiKeyRepository::new();
        let team_id = TeamId::administrators();

        let key_id = ApiKeyId::new("expiring").unwrap();
        let expiring_key = ApiKey::new(key_id, "Expiring Key", "hash", "pk_exp_", team_id.clone())
            .with_expiration(Utc::now() + Duration::hours(1));

        let key_id2 = ApiKeyId::new("not-expiring").unwrap();
        let not_expiring = ApiKey::new(key_id2, "Not Expiring", "hash", "pk_not_", team_id)
            .with_expiration(Utc::now() + Duration::days(30));

        repo.create(expiring_key).await.unwrap();
        repo.create(not_expiring).await.unwrap();

        let expiring = repo.get_expiring_before(Utc::now() + Duration::hours(2)).await.unwrap();
        assert_eq!(expiring.len(), 1);
        assert_eq!(expiring[0].id().as_str(), "expiring");
    }
}
