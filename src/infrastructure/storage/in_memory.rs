//! In-memory storage implementation

use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::RwLock;

use async_trait::async_trait;

use crate::domain::storage::{Storage, StorageEntity, StorageKey};
use crate::domain::DomainError;

/// Thread-safe in-memory storage implementation
///
/// Useful for testing and development. Data is lost when the process terminates.
#[derive(Debug)]
pub struct InMemoryStorage<E>
where
    E: StorageEntity,
{
    entities: RwLock<HashMap<String, E>>,
}

impl<E> Default for InMemoryStorage<E>
where
    E: StorageEntity,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<E> InMemoryStorage<E>
where
    E: StorageEntity,
{
    /// Creates a new empty in-memory storage
    pub fn new() -> Self {
        Self {
            entities: RwLock::new(HashMap::new()),
        }
    }

    /// Creates storage pre-populated with entities
    pub fn with_entities(entities: Vec<E>) -> Self {
        let storage = Self::new();
        {
            let mut map = storage.entities.write().unwrap();

            for entity in entities {
                map.insert(entity.key().as_str().to_string(), entity);
            }
        }
        storage
    }
}

#[async_trait]
impl<E> Storage<E> for InMemoryStorage<E>
where
    E: StorageEntity + 'static,
{
    async fn get(&self, key: &E::Key) -> Result<Option<E>, DomainError> {
        let entities = self.entities.read().map_err(|e| {
            DomainError::storage(format!("Failed to acquire read lock: {}", e))
        })?;

        Ok(entities.get(key.as_str()).cloned())
    }

    async fn list(&self) -> Result<Vec<E>, DomainError> {
        let entities = self.entities.read().map_err(|e| {
            DomainError::storage(format!("Failed to acquire read lock: {}", e))
        })?;

        Ok(entities.values().cloned().collect())
    }

    async fn create(&self, entity: E) -> Result<E, DomainError> {
        let key = entity.key().as_str().to_string();
        let mut entities = self.entities.write().map_err(|e| {
            DomainError::storage(format!("Failed to acquire write lock: {}", e))
        })?;

        if entities.contains_key(&key) {
            return Err(DomainError::conflict(format!(
                "Entity with key '{}' already exists",
                key
            )));
        }

        entities.insert(key, entity.clone());
        Ok(entity)
    }

    async fn update(&self, entity: E) -> Result<E, DomainError> {
        let key = entity.key().as_str().to_string();
        let mut entities = self.entities.write().map_err(|e| {
            DomainError::storage(format!("Failed to acquire write lock: {}", e))
        })?;

        if !entities.contains_key(&key) {
            return Err(DomainError::not_found(format!(
                "Entity with key '{}' not found",
                key
            )));
        }

        entities.insert(key, entity.clone());
        Ok(entity)
    }

    async fn delete(&self, key: &E::Key) -> Result<bool, DomainError> {
        let mut entities = self.entities.write().map_err(|e| {
            DomainError::storage(format!("Failed to acquire write lock: {}", e))
        })?;

        Ok(entities.remove(key.as_str()).is_some())
    }

    async fn clear(&self) -> Result<(), DomainError> {
        let mut entities = self.entities.write().map_err(|e| {
            DomainError::storage(format!("Failed to acquire write lock: {}", e))
        })?;

        entities.clear();
        Ok(())
    }

    async fn count(&self) -> Result<usize, DomainError> {
        let entities = self.entities.read().map_err(|e| {
            DomainError::storage(format!("Failed to acquire read lock: {}", e))
        })?;

        Ok(entities.len())
    }

    async fn exists(&self, key: &E::Key) -> Result<bool, DomainError> {
        let entities = self.entities.read().map_err(|e| {
            DomainError::storage(format!("Failed to acquire read lock: {}", e))
        })?;

        Ok(entities.contains_key(key.as_str()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::storage::StorageKey;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    struct TestId(String);

    impl StorageKey for TestId {
        fn as_str(&self) -> &str {
            &self.0
        }
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct TestEntity {
        id: String,
        name: String,
        value: i32,
    }

    impl StorageEntity for TestEntity {
        type Key = TestId;

        fn key(&self) -> &Self::Key {
            unsafe { &*(&self.id as *const String as *const TestId) }
        }
    }

    fn entity(id: &str, name: &str, value: i32) -> TestEntity {
        TestEntity {
            id: id.to_string(),
            name: name.to_string(),
            value,
        }
    }

    #[tokio::test]
    async fn test_create_and_get() {
        let storage: InMemoryStorage<TestEntity> = InMemoryStorage::new();
        let e = entity("1", "Test", 42);

        storage.create(e.clone()).await.unwrap();

        let result = storage.get(&TestId("1".to_string())).await.unwrap();
        assert_eq!(result, Some(e));
    }

    #[tokio::test]
    async fn test_create_conflict() {
        let storage: InMemoryStorage<TestEntity> = InMemoryStorage::new();
        let e = entity("1", "Test", 42);

        storage.create(e.clone()).await.unwrap();
        let result = storage.create(e).await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DomainError::Conflict { .. }));
    }

    #[tokio::test]
    async fn test_update() {
        let storage: InMemoryStorage<TestEntity> = InMemoryStorage::new();
        let e = entity("1", "Test", 42);

        storage.create(e).await.unwrap();

        let updated = entity("1", "Updated", 100);
        storage.update(updated.clone()).await.unwrap();

        let result = storage.get(&TestId("1".to_string())).await.unwrap();
        assert_eq!(result.unwrap().name, "Updated");
    }

    #[tokio::test]
    async fn test_update_not_found() {
        let storage: InMemoryStorage<TestEntity> = InMemoryStorage::new();
        let e = entity("1", "Test", 42);

        let result = storage.update(e).await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DomainError::NotFound { .. }));
    }

    #[tokio::test]
    async fn test_delete() {
        let storage: InMemoryStorage<TestEntity> = InMemoryStorage::new();
        let e = entity("1", "Test", 42);

        storage.create(e).await.unwrap();
        let deleted = storage.delete(&TestId("1".to_string())).await.unwrap();

        assert!(deleted);

        let exists = storage.exists(&TestId("1".to_string())).await.unwrap();
        assert!(!exists);
    }

    #[tokio::test]
    async fn test_delete_not_found() {
        let storage: InMemoryStorage<TestEntity> = InMemoryStorage::new();

        let deleted = storage.delete(&TestId("1".to_string())).await.unwrap();
        assert!(!deleted);
    }

    #[tokio::test]
    async fn test_list() {
        let storage: InMemoryStorage<TestEntity> = InMemoryStorage::new();

        storage.create(entity("1", "A", 1)).await.unwrap();
        storage.create(entity("2", "B", 2)).await.unwrap();
        storage.create(entity("3", "C", 3)).await.unwrap();

        let list = storage.list().await.unwrap();
        assert_eq!(list.len(), 3);
    }

    #[tokio::test]
    async fn test_count() {
        let storage: InMemoryStorage<TestEntity> = InMemoryStorage::new();

        storage.create(entity("1", "A", 1)).await.unwrap();
        storage.create(entity("2", "B", 2)).await.unwrap();

        let count = storage.count().await.unwrap();
        assert_eq!(count, 2);
    }

    #[tokio::test]
    async fn test_clear() {
        let storage: InMemoryStorage<TestEntity> = InMemoryStorage::new();

        storage.create(entity("1", "A", 1)).await.unwrap();
        storage.create(entity("2", "B", 2)).await.unwrap();

        storage.clear().await.unwrap();

        let count = storage.count().await.unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_save_creates_new() {
        let storage: InMemoryStorage<TestEntity> = InMemoryStorage::new();
        let e = entity("1", "Test", 42);

        storage.save(e.clone()).await.unwrap();

        let result = storage.get(&TestId("1".to_string())).await.unwrap();
        assert_eq!(result, Some(e));
    }

    #[tokio::test]
    async fn test_save_updates_existing() {
        let storage: InMemoryStorage<TestEntity> = InMemoryStorage::new();

        storage.create(entity("1", "Original", 1)).await.unwrap();
        storage.save(entity("1", "Updated", 2)).await.unwrap();

        let result = storage.get(&TestId("1".to_string())).await.unwrap();
        assert_eq!(result.unwrap().name, "Updated");
    }

    #[tokio::test]
    async fn test_with_entities() {
        let entities = vec![entity("1", "A", 1), entity("2", "B", 2)];
        let storage: InMemoryStorage<TestEntity> = InMemoryStorage::with_entities(entities);

        let count = storage.count().await.unwrap();
        assert_eq!(count, 2);
    }
}
