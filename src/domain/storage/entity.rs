//! Storage entity traits and types

use std::fmt::Debug;

use serde::{de::DeserializeOwned, Serialize};

/// Trait for types that can be used as storage keys
pub trait StorageKey: Clone + Debug + Send + Sync + Eq + std::hash::Hash {
    /// Returns the key as a string for storage backends that require string keys
    fn as_str(&self) -> &str;
}

/// Trait for types that can be stored
pub trait StorageEntity: Clone + Debug + Send + Sync + Serialize + DeserializeOwned {
    /// The key type for this entity
    type Key: StorageKey;

    /// Returns the entity's key
    fn key(&self) -> &Self::Key;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
    struct TestKey(String);

    impl StorageKey for TestKey {
        fn as_str(&self) -> &str {
            &self.0
        }
    }

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    struct TestEntity {
        id: TestKey,
        name: String,
    }

    impl StorageEntity for TestEntity {
        type Key = TestKey;

        fn key(&self) -> &Self::Key {
            &self.id
        }
    }

    #[test]
    fn test_storage_key_as_str() {
        let key = TestKey("test-key".to_string());
        assert_eq!(key.as_str(), "test-key");
    }

    #[test]
    fn test_storage_entity_key() {
        let entity = TestEntity {
            id: TestKey("entity-1".to_string()),
            name: "Test".to_string(),
        };
        assert_eq!(entity.key().as_str(), "entity-1");
    }
}
