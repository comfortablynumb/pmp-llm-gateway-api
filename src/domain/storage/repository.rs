//! Storage trait definition

use std::fmt::Debug;

use async_trait::async_trait;

use crate::domain::DomainError;

use super::entity::{StorageEntity, StorageKey};

/// Generic storage trait for CRUD operations on any entity type
#[async_trait]
pub trait Storage<E>: Send + Sync + Debug
where
    E: StorageEntity + 'static,
{
    /// Retrieves an entity by its key
    async fn get(&self, key: &E::Key) -> Result<Option<E>, DomainError>;

    /// Retrieves all entities
    async fn list(&self) -> Result<Vec<E>, DomainError>;

    /// Creates a new entity, returns error if already exists
    async fn create(&self, entity: E) -> Result<E, DomainError>;

    /// Updates an existing entity, returns error if not found
    async fn update(&self, entity: E) -> Result<E, DomainError>;

    /// Saves an entity (creates if not exists, updates if exists)
    async fn save(&self, entity: E) -> Result<E, DomainError> {
        if self.exists(entity.key()).await? {
            self.update(entity).await
        } else {
            self.create(entity).await
        }
    }

    /// Deletes an entity by its key, returns true if deleted
    async fn delete(&self, key: &E::Key) -> Result<bool, DomainError>;

    /// Checks if an entity exists by its key
    async fn exists(&self, key: &E::Key) -> Result<bool, DomainError> {
        Ok(self.get(key).await?.is_some())
    }

    /// Returns the count of entities
    async fn count(&self) -> Result<usize, DomainError> {
        Ok(self.list().await?.len())
    }

    /// Clears all entities (use with caution)
    async fn clear(&self) -> Result<(), DomainError>;
}

#[cfg(test)]
pub mod mock {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    /// Mock storage for testing
    #[derive(Debug)]
    pub struct MockStorage<E>
    where
        E: StorageEntity,
    {
        entities: Mutex<HashMap<String, E>>,
        error: Mutex<Option<String>>,
    }

    impl<E> Default for MockStorage<E>
    where
        E: StorageEntity,
    {
        fn default() -> Self {
            Self::new()
        }
    }

    impl<E> MockStorage<E>
    where
        E: StorageEntity,
    {
        pub fn new() -> Self {
            Self {
                entities: Mutex::new(HashMap::new()),
                error: Mutex::new(None),
            }
        }

        pub fn with_entity(self, entity: E) -> Self {
            self.entities
                .lock()
                .unwrap()
                .insert(entity.key().as_str().to_string(), entity);
            self
        }

        pub fn with_error(self, error: impl Into<String>) -> Self {
            *self.error.lock().unwrap() = Some(error.into());
            self
        }

        fn check_error(&self) -> Result<(), DomainError> {
            if let Some(error) = self.error.lock().unwrap().clone() {
                return Err(DomainError::storage(error));
            }
            Ok(())
        }
    }

    #[async_trait]
    impl<E> Storage<E> for MockStorage<E>
    where
        E: StorageEntity + 'static,
    {
        async fn get(&self, key: &E::Key) -> Result<Option<E>, DomainError> {
            self.check_error()?;
            Ok(self
                .entities
                .lock()
                .unwrap()
                .get(key.as_str())
                .cloned())
        }

        async fn list(&self) -> Result<Vec<E>, DomainError> {
            self.check_error()?;
            Ok(self.entities.lock().unwrap().values().cloned().collect())
        }

        async fn create(&self, entity: E) -> Result<E, DomainError> {
            self.check_error()?;
            let key = entity.key().as_str().to_string();
            let mut entities = self.entities.lock().unwrap();

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
            self.check_error()?;
            let key = entity.key().as_str().to_string();
            let mut entities = self.entities.lock().unwrap();

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
            self.check_error()?;
            Ok(self
                .entities
                .lock()
                .unwrap()
                .remove(key.as_str())
                .is_some())
        }

        async fn clear(&self) -> Result<(), DomainError> {
            self.check_error()?;
            self.entities.lock().unwrap().clear();
            Ok(())
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use serde::{Deserialize, Serialize};

        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        struct TestKey(String);

        impl StorageKey for TestKey {
            fn as_str(&self) -> &str {
                &self.0
            }
        }

        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        struct TestEntity {
            id: String,
            name: String,
        }

        impl StorageEntity for TestEntity {
            type Key = TestKey;

            fn key(&self) -> &Self::Key {
                // This is a workaround since we can't store TestKey directly
                // In real code, the entity would store the key type
                unsafe { &*(&self.id as *const String as *const TestKey) }
            }
        }

        fn create_test_entity(id: &str, name: &str) -> TestEntity {
            TestEntity {
                id: id.to_string(),
                name: name.to_string(),
            }
        }

        #[tokio::test]
        async fn test_mock_storage_create() {
            let storage: MockStorage<TestEntity> = MockStorage::new();
            let entity = create_test_entity("1", "Test");

            let result = storage.create(entity.clone()).await;
            assert!(result.is_ok());
            assert_eq!(result.unwrap().name, "Test");
        }

        #[tokio::test]
        async fn test_mock_storage_create_conflict() {
            let entity = create_test_entity("1", "Test");
            let storage: MockStorage<TestEntity> = MockStorage::new().with_entity(entity.clone());

            let result = storage.create(entity).await;
            assert!(result.is_err());
        }

        #[tokio::test]
        async fn test_mock_storage_get() {
            let entity = create_test_entity("1", "Test");
            let storage: MockStorage<TestEntity> = MockStorage::new().with_entity(entity.clone());

            let result = storage.get(&TestKey("1".to_string())).await;
            assert!(result.is_ok());
            assert_eq!(result.unwrap().unwrap().name, "Test");
        }

        #[tokio::test]
        async fn test_mock_storage_get_not_found() {
            let storage: MockStorage<TestEntity> = MockStorage::new();

            let result = storage.get(&TestKey("1".to_string())).await;
            assert!(result.is_ok());
            assert!(result.unwrap().is_none());
        }

        #[tokio::test]
        async fn test_mock_storage_update() {
            let entity = create_test_entity("1", "Test");
            let storage: MockStorage<TestEntity> = MockStorage::new().with_entity(entity);

            let updated = create_test_entity("1", "Updated");
            let result = storage.update(updated).await;
            assert!(result.is_ok());
            assert_eq!(result.unwrap().name, "Updated");
        }

        #[tokio::test]
        async fn test_mock_storage_update_not_found() {
            let storage: MockStorage<TestEntity> = MockStorage::new();
            let entity = create_test_entity("1", "Test");

            let result = storage.update(entity).await;
            assert!(result.is_err());
        }

        #[tokio::test]
        async fn test_mock_storage_delete() {
            let entity = create_test_entity("1", "Test");
            let storage: MockStorage<TestEntity> = MockStorage::new().with_entity(entity);

            let result = storage.delete(&TestKey("1".to_string())).await;
            assert!(result.is_ok());
            assert!(result.unwrap());

            let exists = storage.exists(&TestKey("1".to_string())).await.unwrap();
            assert!(!exists);
        }

        #[tokio::test]
        async fn test_mock_storage_list() {
            let entity1 = create_test_entity("1", "Test1");
            let entity2 = create_test_entity("2", "Test2");
            let storage: MockStorage<TestEntity> = MockStorage::new()
                .with_entity(entity1)
                .with_entity(entity2);

            let result = storage.list().await;
            assert!(result.is_ok());
            assert_eq!(result.unwrap().len(), 2);
        }

        #[tokio::test]
        async fn test_mock_storage_count() {
            let entity1 = create_test_entity("1", "Test1");
            let entity2 = create_test_entity("2", "Test2");
            let storage: MockStorage<TestEntity> = MockStorage::new()
                .with_entity(entity1)
                .with_entity(entity2);

            let result = storage.count().await;
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), 2);
        }

        #[tokio::test]
        async fn test_mock_storage_clear() {
            let entity = create_test_entity("1", "Test");
            let storage: MockStorage<TestEntity> = MockStorage::new().with_entity(entity);

            let result = storage.clear().await;
            assert!(result.is_ok());

            let count = storage.count().await.unwrap();
            assert_eq!(count, 0);
        }

        #[tokio::test]
        async fn test_mock_storage_with_error() {
            let storage: MockStorage<TestEntity> =
                MockStorage::new().with_error("Simulated storage error");

            let result = storage.list().await;
            assert!(result.is_err());
        }
    }
}
