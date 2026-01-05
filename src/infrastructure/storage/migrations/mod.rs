//! Database migrations infrastructure

use async_trait::async_trait;
use sqlx::postgres::PgPool;

use crate::domain::DomainError;

/// Trait for running database migrations
#[async_trait]
pub trait Migrator: Send + Sync {
    /// Runs all pending migrations
    async fn run(&self) -> Result<(), DomainError>;

    /// Reverts the last migration
    async fn revert(&self) -> Result<(), DomainError>;

    /// Returns the current migration version
    async fn version(&self) -> Result<Option<i64>, DomainError>;
}

/// PostgreSQL migrator using sqlx embedded migrations
#[derive(Debug)]
pub struct PostgresMigrator {
    pool: PgPool,
}

impl PostgresMigrator {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Creates the migrations table if it doesn't exist
    async fn ensure_migrations_table(&self) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS _migrations (
                version BIGINT PRIMARY KEY,
                description TEXT NOT NULL,
                installed_on TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                success BOOLEAN NOT NULL DEFAULT TRUE
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::storage(format!("Failed to create migrations table: {}", e)))?;

        Ok(())
    }

    /// Runs a single migration
    pub async fn run_migration(&self, migration: &Migration) -> Result<(), DomainError> {
        self.ensure_migrations_table().await?;

        // Check if already applied
        let applied: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM _migrations WHERE version = $1)",
        )
        .bind(migration.version)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DomainError::storage(format!("Failed to check migration status: {}", e)))?;

        if applied {
            return Ok(());
        }

        // Run the migration
        sqlx::query(&migration.up)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                DomainError::storage(format!(
                    "Failed to run migration {}: {}",
                    migration.version, e
                ))
            })?;

        // Record the migration
        sqlx::query(
            "INSERT INTO _migrations (version, description) VALUES ($1, $2)",
        )
        .bind(migration.version)
        .bind(&migration.description)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            DomainError::storage(format!("Failed to record migration {}: {}", migration.version, e))
        })?;

        Ok(())
    }

    /// Reverts a single migration
    pub async fn revert_migration(&self, migration: &Migration) -> Result<(), DomainError> {
        self.ensure_migrations_table().await?;

        // Check if applied
        let applied: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM _migrations WHERE version = $1)",
        )
        .bind(migration.version)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DomainError::storage(format!("Failed to check migration status: {}", e)))?;

        if !applied {
            return Ok(());
        }

        // Run the down migration
        sqlx::query(&migration.down)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                DomainError::storage(format!(
                    "Failed to revert migration {}: {}",
                    migration.version, e
                ))
            })?;

        // Remove the migration record
        sqlx::query("DELETE FROM _migrations WHERE version = $1")
            .bind(migration.version)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                DomainError::storage(format!(
                    "Failed to remove migration record {}: {}",
                    migration.version, e
                ))
            })?;

        Ok(())
    }

    /// Returns the latest applied migration version
    pub async fn current_version(&self) -> Result<Option<i64>, DomainError> {
        self.ensure_migrations_table().await?;

        let version: Option<i64> = sqlx::query_scalar(
            "SELECT MAX(version) FROM _migrations WHERE success = TRUE",
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DomainError::storage(format!("Failed to get migration version: {}", e)))?;

        Ok(version)
    }

    /// Returns all applied migration versions
    pub async fn applied_versions(&self) -> Result<Vec<i64>, DomainError> {
        self.ensure_migrations_table().await?;

        let versions: Vec<i64> = sqlx::query_scalar(
            "SELECT version FROM _migrations WHERE success = TRUE ORDER BY version",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::storage(format!("Failed to get applied migrations: {}", e)))?;

        Ok(versions)
    }
}

/// Represents a database migration
#[derive(Debug, Clone)]
pub struct Migration {
    /// Migration version (timestamp-based recommended)
    pub version: i64,
    /// Human-readable description
    pub description: String,
    /// SQL to run when applying the migration
    pub up: String,
    /// SQL to run when reverting the migration
    pub down: String,
}

impl Migration {
    pub fn new(
        version: i64,
        description: impl Into<String>,
        up: impl Into<String>,
        down: impl Into<String>,
    ) -> Self {
        Self {
            version,
            description: description.into(),
            up: up.into(),
            down: down.into(),
        }
    }
}

/// Collection of migrations for the storage layer
pub fn storage_migrations() -> Vec<Migration> {
    vec![
        Migration::new(
            1,
            "Create models table",
            r#"
            CREATE TABLE IF NOT EXISTS models (
                key VARCHAR(255) PRIMARY KEY,
                data JSONB NOT NULL,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            );
            CREATE INDEX IF NOT EXISTS idx_models_created_at ON models(created_at);
            "#,
            r#"
            DROP TABLE IF EXISTS models;
            "#,
        ),
        Migration::new(
            2,
            "Create prompts table",
            r#"
            CREATE TABLE IF NOT EXISTS prompts (
                key VARCHAR(255) PRIMARY KEY,
                data JSONB NOT NULL,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            );
            CREATE INDEX IF NOT EXISTS idx_prompts_created_at ON prompts(created_at);
            "#,
            r#"
            DROP TABLE IF EXISTS prompts;
            "#,
        ),
        Migration::new(
            3,
            "Create chains table",
            r#"
            CREATE TABLE IF NOT EXISTS chains (
                key VARCHAR(255) PRIMARY KEY,
                data JSONB NOT NULL,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            );
            CREATE INDEX IF NOT EXISTS idx_chains_created_at ON chains(created_at);
            "#,
            r#"
            DROP TABLE IF EXISTS chains;
            "#,
        ),
        Migration::new(
            4,
            "Create credentials table",
            r#"
            CREATE TABLE IF NOT EXISTS credentials (
                key VARCHAR(255) PRIMARY KEY,
                data JSONB NOT NULL,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            );
            CREATE INDEX IF NOT EXISTS idx_credentials_created_at ON credentials(created_at);
            "#,
            r#"
            DROP TABLE IF EXISTS credentials;
            "#,
        ),
    ]
}

/// Runs all pending storage migrations
pub async fn run_storage_migrations(pool: &PgPool) -> Result<(), DomainError> {
    let migrator = PostgresMigrator::new(pool.clone());
    let migrations = storage_migrations();

    for migration in migrations {
        migrator.run_migration(&migration).await?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migration_creation() {
        let migration = Migration::new(1, "Test migration", "CREATE TABLE test", "DROP TABLE test");

        assert_eq!(migration.version, 1);
        assert_eq!(migration.description, "Test migration");
        assert_eq!(migration.up, "CREATE TABLE test");
        assert_eq!(migration.down, "DROP TABLE test");
    }

    #[test]
    fn test_storage_migrations_order() {
        let migrations = storage_migrations();

        assert!(!migrations.is_empty());

        // Verify migrations are in order
        for i in 1..migrations.len() {
            assert!(
                migrations[i].version > migrations[i - 1].version,
                "Migrations should be in ascending order"
            );
        }
    }

    #[test]
    fn test_storage_migrations_content() {
        let migrations = storage_migrations();

        // Verify all migrations have required content
        for migration in migrations {
            assert!(!migration.description.is_empty());
            assert!(!migration.up.is_empty());
            assert!(!migration.down.is_empty());
        }
    }
}
