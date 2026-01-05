//! Storage factory for runtime storage selection

use std::sync::Arc;

use crate::domain::storage::{Storage, StorageEntity};
use crate::domain::DomainError;

use super::in_memory::InMemoryStorage;
use super::postgres::{PostgresConfig, PostgresStorage};

/// Supported storage types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StorageType {
    /// In-memory storage (for testing/development)
    InMemory,
    /// PostgreSQL storage
    Postgres,
}

impl StorageType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "memory" | "inmemory" | "in-memory" | "in_memory" => Some(Self::InMemory),
            "postgres" | "postgresql" | "pg" => Some(Self::Postgres),
            _ => None,
        }
    }
}

/// Storage configuration
#[derive(Debug, Clone)]
pub enum StorageConfig {
    /// In-memory storage configuration
    InMemory,
    /// PostgreSQL storage configuration
    Postgres(PostgresConfig),
}

impl StorageConfig {
    /// Creates an in-memory storage configuration
    pub fn in_memory() -> Self {
        Self::InMemory
    }

    /// Creates a PostgreSQL storage configuration
    pub fn postgres(config: PostgresConfig) -> Self {
        Self::Postgres(config)
    }

    /// Creates a PostgreSQL configuration from a URL
    pub fn postgres_url(url: impl Into<String>) -> Self {
        Self::Postgres(PostgresConfig::new(url))
    }

    /// Returns the storage type
    pub fn storage_type(&self) -> StorageType {
        match self {
            Self::InMemory => StorageType::InMemory,
            Self::Postgres(_) => StorageType::Postgres,
        }
    }
}

/// Factory for creating storage instances
#[derive(Debug)]
pub struct StorageFactory;

impl StorageFactory {
    /// Creates a storage instance based on the configuration
    pub async fn create<E>(
        config: &StorageConfig,
        table_name: &str,
    ) -> Result<Arc<dyn Storage<E>>, DomainError>
    where
        E: StorageEntity + 'static,
    {
        match config {
            StorageConfig::InMemory => {
                Ok(Arc::new(InMemoryStorage::<E>::new()))
            }
            StorageConfig::Postgres(pg_config) => {
                let storage = PostgresStorage::<E>::connect(pg_config, table_name).await?;
                storage.ensure_table().await?;
                Ok(Arc::new(storage))
            }
        }
    }

    /// Creates an in-memory storage
    pub fn create_in_memory<E>() -> Arc<InMemoryStorage<E>>
    where
        E: StorageEntity,
    {
        Arc::new(InMemoryStorage::new())
    }

    /// Creates a PostgreSQL storage
    pub async fn create_postgres<E>(
        config: &PostgresConfig,
        table_name: &str,
    ) -> Result<Arc<PostgresStorage<E>>, DomainError>
    where
        E: StorageEntity + 'static,
    {
        let storage = PostgresStorage::connect(config, table_name).await?;
        storage.ensure_table().await?;
        Ok(Arc::new(storage))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_type_from_str() {
        assert_eq!(
            StorageType::from_str("memory"),
            Some(StorageType::InMemory)
        );
        assert_eq!(
            StorageType::from_str("inmemory"),
            Some(StorageType::InMemory)
        );
        assert_eq!(
            StorageType::from_str("in-memory"),
            Some(StorageType::InMemory)
        );
        assert_eq!(
            StorageType::from_str("postgres"),
            Some(StorageType::Postgres)
        );
        assert_eq!(
            StorageType::from_str("postgresql"),
            Some(StorageType::Postgres)
        );
        assert_eq!(StorageType::from_str("pg"), Some(StorageType::Postgres));
        assert_eq!(StorageType::from_str("unknown"), None);
    }

    #[test]
    fn test_storage_config_types() {
        let in_memory = StorageConfig::in_memory();
        assert_eq!(in_memory.storage_type(), StorageType::InMemory);

        let postgres = StorageConfig::postgres_url("postgres://localhost/test");
        assert_eq!(postgres.storage_type(), StorageType::Postgres);
    }

    #[test]
    fn test_storage_config_postgres() {
        let config = PostgresConfig::new("postgres://localhost/test")
            .with_max_connections(20);
        let storage_config = StorageConfig::postgres(config.clone());

        if let StorageConfig::Postgres(pg_config) = storage_config {
            assert_eq!(pg_config.url, config.url);
            assert_eq!(pg_config.max_connections, 20);
        } else {
            panic!("Expected Postgres config");
        }
    }
}
