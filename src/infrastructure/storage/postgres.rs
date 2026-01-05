//! PostgreSQL storage implementation with connection pooling

use std::fmt::Debug;
use std::marker::PhantomData;

use async_trait::async_trait;
use sqlx::postgres::{PgPool, PgPoolOptions};
use sqlx::Row;

use crate::domain::storage::{Storage, StorageEntity, StorageKey};
use crate::domain::DomainError;

/// PostgreSQL storage configuration
#[derive(Debug, Clone)]
pub struct PostgresConfig {
    /// Database connection URL
    pub url: String,
    /// Maximum number of connections in the pool
    pub max_connections: u32,
    /// Minimum number of connections to maintain
    pub min_connections: u32,
    /// Connection timeout in seconds
    pub connect_timeout_secs: u64,
    /// Idle timeout in seconds
    pub idle_timeout_secs: u64,
}

impl Default for PostgresConfig {
    fn default() -> Self {
        Self {
            url: "postgres://localhost/pmp_llm_gateway".to_string(),
            max_connections: 10,
            min_connections: 1,
            connect_timeout_secs: 30,
            idle_timeout_secs: 600,
        }
    }
}

impl PostgresConfig {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            ..Default::default()
        }
    }

    pub fn with_max_connections(mut self, max: u32) -> Self {
        self.max_connections = max;
        self
    }

    pub fn with_min_connections(mut self, min: u32) -> Self {
        self.min_connections = min;
        self
    }

    pub fn with_connect_timeout(mut self, secs: u64) -> Self {
        self.connect_timeout_secs = secs;
        self
    }

    pub fn with_idle_timeout(mut self, secs: u64) -> Self {
        self.idle_timeout_secs = secs;
        self
    }
}

/// PostgreSQL storage implementation with connection pooling
///
/// Stores entities as JSON in a table with (entity_type, key, data) columns.
/// Uses sqlx connection pool for efficient connection management.
pub struct PostgresStorage<E>
where
    E: StorageEntity,
{
    pool: PgPool,
    table_name: String,
    _phantom: PhantomData<E>,
}

impl<E> Debug for PostgresStorage<E>
where
    E: StorageEntity,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PostgresStorage")
            .field("table_name", &self.table_name)
            .finish()
    }
}

impl<E> PostgresStorage<E>
where
    E: StorageEntity,
{
    /// Creates a new PostgreSQL storage with the given pool and table name
    pub fn new(pool: PgPool, table_name: impl Into<String>) -> Self {
        Self {
            pool,
            table_name: table_name.into(),
            _phantom: PhantomData,
        }
    }

    /// Creates a new PostgreSQL storage with connection pooling
    pub async fn connect(
        config: &PostgresConfig,
        table_name: impl Into<String>,
    ) -> Result<Self, DomainError> {
        let pool = PgPoolOptions::new()
            .max_connections(config.max_connections)
            .min_connections(config.min_connections)
            .acquire_timeout(std::time::Duration::from_secs(config.connect_timeout_secs))
            .idle_timeout(std::time::Duration::from_secs(config.idle_timeout_secs))
            .connect(&config.url)
            .await
            .map_err(|e| DomainError::storage(format!("Failed to connect to PostgreSQL: {}", e)))?;

        Ok(Self::new(pool, table_name))
    }

    /// Returns a reference to the connection pool
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Ensures the storage table exists
    pub async fn ensure_table(&self) -> Result<(), DomainError> {
        let query = format!(
            r#"
            CREATE TABLE IF NOT EXISTS {} (
                key VARCHAR(255) PRIMARY KEY,
                data JSONB NOT NULL,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            )
            "#,
            self.table_name
        );

        sqlx::query(&query)
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::storage(format!("Failed to create table: {}", e)))?;

        Ok(())
    }
}

#[async_trait]
impl<E> Storage<E> for PostgresStorage<E>
where
    E: StorageEntity + 'static,
{
    async fn get(&self, key: &E::Key) -> Result<Option<E>, DomainError> {
        let query = format!(
            "SELECT data FROM {} WHERE key = $1",
            self.table_name
        );

        let result = sqlx::query(&query)
            .bind(key.as_str())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| DomainError::storage(format!("Failed to get entity: {}", e)))?;

        match result {
            Some(row) => {
                let data: serde_json::Value = row.get("data");
                let entity: E = serde_json::from_value(data).map_err(|e| {
                    DomainError::storage(format!("Failed to deserialize entity: {}", e))
                })?;
                Ok(Some(entity))
            }
            None => Ok(None),
        }
    }

    async fn list(&self) -> Result<Vec<E>, DomainError> {
        let query = format!(
            "SELECT data FROM {} ORDER BY created_at",
            self.table_name
        );

        let rows = sqlx::query(&query)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| DomainError::storage(format!("Failed to list entities: {}", e)))?;

        let mut entities = Vec::with_capacity(rows.len());

        for row in rows {
            let data: serde_json::Value = row.get("data");
            let entity: E = serde_json::from_value(data).map_err(|e| {
                DomainError::storage(format!("Failed to deserialize entity: {}", e))
            })?;
            entities.push(entity);
        }

        Ok(entities)
    }

    async fn create(&self, entity: E) -> Result<E, DomainError> {
        let key = entity.key().as_str().to_string();
        let data = serde_json::to_value(&entity).map_err(|e| {
            DomainError::storage(format!("Failed to serialize entity: {}", e))
        })?;

        let query = format!(
            r#"
            INSERT INTO {} (key, data)
            VALUES ($1, $2)
            "#,
            self.table_name
        );

        sqlx::query(&query)
            .bind(&key)
            .bind(&data)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                if e.to_string().contains("duplicate key") {
                    DomainError::conflict(format!("Entity with key '{}' already exists", key))
                } else {
                    DomainError::storage(format!("Failed to create entity: {}", e))
                }
            })?;

        Ok(entity)
    }

    async fn update(&self, entity: E) -> Result<E, DomainError> {
        let key = entity.key().as_str().to_string();
        let data = serde_json::to_value(&entity).map_err(|e| {
            DomainError::storage(format!("Failed to serialize entity: {}", e))
        })?;

        let query = format!(
            r#"
            UPDATE {}
            SET data = $2, updated_at = NOW()
            WHERE key = $1
            "#,
            self.table_name
        );

        let result = sqlx::query(&query)
            .bind(&key)
            .bind(&data)
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::storage(format!("Failed to update entity: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(DomainError::not_found(format!(
                "Entity with key '{}' not found",
                key
            )));
        }

        Ok(entity)
    }

    async fn delete(&self, key: &E::Key) -> Result<bool, DomainError> {
        let query = format!(
            "DELETE FROM {} WHERE key = $1",
            self.table_name
        );

        let result = sqlx::query(&query)
            .bind(key.as_str())
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::storage(format!("Failed to delete entity: {}", e)))?;

        Ok(result.rows_affected() > 0)
    }

    async fn clear(&self) -> Result<(), DomainError> {
        let query = format!("DELETE FROM {}", self.table_name);

        sqlx::query(&query)
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::storage(format!("Failed to clear storage: {}", e)))?;

        Ok(())
    }

    async fn count(&self) -> Result<usize, DomainError> {
        let query = format!("SELECT COUNT(*) as count FROM {}", self.table_name);

        let row = sqlx::query(&query)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| DomainError::storage(format!("Failed to count entities: {}", e)))?;

        let count: i64 = row.get("count");
        Ok(count as usize)
    }

    async fn exists(&self, key: &E::Key) -> Result<bool, DomainError> {
        let query = format!(
            "SELECT EXISTS(SELECT 1 FROM {} WHERE key = $1) as exists",
            self.table_name
        );

        let row = sqlx::query(&query)
            .bind(key.as_str())
            .fetch_one(&self.pool)
            .await
            .map_err(|e| DomainError::storage(format!("Failed to check existence: {}", e)))?;

        let exists: bool = row.get("exists");
        Ok(exists)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_postgres_config_default() {
        let config = PostgresConfig::default();

        assert_eq!(config.max_connections, 10);
        assert_eq!(config.min_connections, 1);
        assert_eq!(config.connect_timeout_secs, 30);
        assert_eq!(config.idle_timeout_secs, 600);
    }

    #[test]
    fn test_postgres_config_builder() {
        let config = PostgresConfig::new("postgres://localhost/test")
            .with_max_connections(20)
            .with_min_connections(5)
            .with_connect_timeout(60)
            .with_idle_timeout(300);

        assert_eq!(config.url, "postgres://localhost/test");
        assert_eq!(config.max_connections, 20);
        assert_eq!(config.min_connections, 5);
        assert_eq!(config.connect_timeout_secs, 60);
        assert_eq!(config.idle_timeout_secs, 300);
    }
}
