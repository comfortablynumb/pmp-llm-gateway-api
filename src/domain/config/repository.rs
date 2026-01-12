//! Repository traits for configuration and execution logs

use async_trait::async_trait;

use crate::domain::error::DomainError;

use super::{
    AppConfiguration, ConfigKey, ConfigValue, ExecutionLog, ExecutionLogId, ExecutionLogQuery,
    ExecutionStats,
};

/// Repository trait for application configuration
#[async_trait]
pub trait ConfigRepository: Send + Sync {
    /// Get the current configuration
    async fn get(&self) -> Result<AppConfiguration, DomainError>;

    /// Update a configuration value
    async fn set(&self, key: &ConfigKey, value: ConfigValue) -> Result<(), DomainError>;

    /// Reset configuration to defaults
    async fn reset(&self) -> Result<(), DomainError>;
}

/// Repository trait for execution logs
#[async_trait]
pub trait ExecutionLogRepository: Send + Sync {
    /// Get an execution log by ID
    async fn get(&self, id: &ExecutionLogId) -> Result<Option<ExecutionLog>, DomainError>;

    /// List execution logs matching query
    async fn list(&self, query: &ExecutionLogQuery) -> Result<Vec<ExecutionLog>, DomainError>;

    /// Count execution logs matching query
    async fn count(&self, query: &ExecutionLogQuery) -> Result<usize, DomainError>;

    /// Save an execution log
    async fn save(&self, log: &ExecutionLog) -> Result<(), DomainError>;

    /// Delete an execution log by ID
    async fn delete(&self, id: &ExecutionLogId) -> Result<bool, DomainError>;

    /// Delete logs older than specified days
    async fn delete_older_than(&self, days: i64) -> Result<usize, DomainError>;

    /// Get aggregated statistics
    async fn stats(&self, query: &ExecutionLogQuery) -> Result<ExecutionStats, DomainError>;
}
