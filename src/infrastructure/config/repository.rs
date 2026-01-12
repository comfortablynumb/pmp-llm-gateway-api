//! Storage-backed implementations for configuration and execution log repositories

use async_trait::async_trait;
use std::sync::Arc;

use crate::domain::{
    config::{
        AppConfiguration, AppConfigurationId, ConfigKey, ConfigRepository, ConfigValue,
        ExecutionLog, ExecutionLogId, ExecutionLogQuery, ExecutionLogRepository, ExecutionStats,
        ExecutionStatus,
    },
    storage::{Storage, StorageEntity},
    DomainError,
};

/// Storage-backed configuration repository
pub struct StorageConfigRepository {
    storage: Arc<dyn Storage<AppConfiguration>>,
}

impl StorageConfigRepository {
    pub fn new(storage: Arc<dyn Storage<AppConfiguration>>) -> Self {
        Self { storage }
    }
}

#[async_trait]
impl ConfigRepository for StorageConfigRepository {
    async fn get(&self) -> Result<AppConfiguration, DomainError> {
        let key = AppConfigurationId::singleton();

        match self.storage.get(&key).await? {
            Some(config) => Ok(config),
            None => {
                // Return defaults if not found (first run)
                Ok(AppConfiguration::with_defaults())
            }
        }
    }

    async fn set(&self, key: &ConfigKey, value: ConfigValue) -> Result<(), DomainError> {
        let storage_key = AppConfigurationId::singleton();

        // Get current config or defaults
        let mut config = match self.storage.get(&storage_key).await? {
            Some(c) => c,
            None => AppConfiguration::with_defaults(),
        };

        // Update the value
        config.set(key.clone(), value).map_err(|e| {
            DomainError::validation(format!("Configuration validation error: {}", e))
        })?;

        // Save back (create if not exists, update if exists)
        if self.storage.exists(&storage_key).await? {
            self.storage.update(config).await?;
        } else {
            self.storage.create(config).await?;
        }

        Ok(())
    }

    async fn reset(&self) -> Result<(), DomainError> {
        let key = AppConfigurationId::singleton();

        // Delete existing and create fresh defaults
        let _ = self.storage.delete(&key).await?;
        self.storage.create(AppConfiguration::with_defaults()).await?;

        Ok(())
    }
}

/// Storage-backed execution log repository
pub struct StorageExecutionLogRepository {
    storage: Arc<dyn Storage<ExecutionLog>>,
}

impl StorageExecutionLogRepository {
    pub fn new(storage: Arc<dyn Storage<ExecutionLog>>) -> Self {
        Self { storage }
    }
}

#[async_trait]
impl ExecutionLogRepository for StorageExecutionLogRepository {
    async fn get(&self, id: &ExecutionLogId) -> Result<Option<ExecutionLog>, DomainError> {
        self.storage.get(id).await
    }

    async fn list(&self, query: &ExecutionLogQuery) -> Result<Vec<ExecutionLog>, DomainError> {
        let all_logs = self.storage.list().await?;
        let filtered = filter_logs(all_logs.iter(), query);
        let mut result: Vec<_> = filtered.cloned().collect();

        // Sort by created_at descending (newest first)
        result.sort_by(|a, b| b.created_at().cmp(&a.created_at()));

        // Apply offset and limit
        let offset = query.offset.unwrap_or(0);
        let limit = query.limit.unwrap_or(usize::MAX);

        Ok(result.into_iter().skip(offset).take(limit).collect())
    }

    async fn count(&self, query: &ExecutionLogQuery) -> Result<usize, DomainError> {
        let all_logs = self.storage.list().await?;
        Ok(filter_logs(all_logs.iter(), query).count())
    }

    async fn save(&self, log: &ExecutionLog) -> Result<(), DomainError> {
        if self.storage.exists(log.key()).await? {
            self.storage.update(log.clone()).await?;
        } else {
            self.storage.create(log.clone()).await?;
        }

        Ok(())
    }

    async fn delete(&self, id: &ExecutionLogId) -> Result<bool, DomainError> {
        self.storage.delete(id).await
    }

    async fn delete_older_than(&self, days: i64) -> Result<usize, DomainError> {
        let cutoff = chrono::Utc::now() - chrono::Duration::days(days);
        let all_logs = self.storage.list().await?;

        let to_delete: Vec<_> = all_logs
            .iter()
            .filter(|log| log.created_at() < cutoff)
            .collect();

        let count = to_delete.len();

        for log in to_delete {
            self.storage.delete(log.key()).await?;
        }

        Ok(count)
    }

    async fn stats(&self, query: &ExecutionLogQuery) -> Result<ExecutionStats, DomainError> {
        let all_logs = self.storage.list().await?;
        let filtered: Vec<_> = filter_logs(all_logs.iter(), query).collect();

        if filtered.is_empty() {
            return Ok(ExecutionStats::empty());
        }

        let total_executions = filtered.len();
        let successful_executions = filtered
            .iter()
            .filter(|log| log.status() == ExecutionStatus::Success)
            .count();
        let failed_executions = total_executions - successful_executions;

        let total_cost_micros: i64 =
            filtered.iter().filter_map(|log| log.cost_micros()).sum();

        let total_input_tokens: u64 = filtered
            .iter()
            .filter_map(|log| log.token_usage())
            .map(|u| u.input_tokens as u64)
            .sum();

        let total_output_tokens: u64 = filtered
            .iter()
            .filter_map(|log| log.token_usage())
            .map(|u| u.output_tokens as u64)
            .sum();

        let total_time_ms: u64 = filtered.iter().map(|log| log.execution_time_ms()).sum();
        let avg_execution_time_ms = total_time_ms as f64 / total_executions as f64;

        let mut executions_by_type: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();

        for log in &filtered {
            *executions_by_type
                .entry(log.execution_type().to_string())
                .or_insert(0) += 1;
        }

        let mut executions_by_resource: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();

        for log in &filtered {
            *executions_by_resource
                .entry(log.resource_id().to_string())
                .or_insert(0) += 1;
        }

        Ok(ExecutionStats {
            total_executions,
            successful_executions,
            failed_executions,
            total_cost_micros,
            total_input_tokens,
            total_output_tokens,
            avg_execution_time_ms,
            executions_by_type,
            executions_by_resource,
        })
    }
}

fn filter_logs<'a>(
    logs: impl Iterator<Item = &'a ExecutionLog>,
    query: &ExecutionLogQuery,
) -> impl Iterator<Item = &'a ExecutionLog> {
    logs.filter(move |log| {
        if let Some(exec_type) = &query.execution_type {
            if log.execution_type() != *exec_type {
                return false;
            }
        }

        if let Some(resource_id) = &query.resource_id {
            if log.resource_id() != resource_id {
                return false;
            }
        }

        if let Some(status) = &query.status {
            if log.status() != *status {
                return false;
            }
        }

        if let Some(api_key_id) = &query.api_key_id {
            if log.executor().api_key_id.as_deref() != Some(api_key_id.as_str()) {
                return false;
            }
        }

        if let Some(user_id) = &query.user_id {
            if log.executor().user_id.as_deref() != Some(user_id.as_str()) {
                return false;
            }
        }

        if let Some(from_date) = &query.from_date {
            if log.created_at() < *from_date {
                return false;
            }
        }

        if let Some(to_date) = &query.to_date {
            if log.created_at() > *to_date {
                return false;
            }
        }

        true
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::config::{ExecutionType, Executor, TokenUsage};
    use crate::infrastructure::storage::InMemoryStorage;

    fn create_config_repo() -> StorageConfigRepository {
        let storage = Arc::new(InMemoryStorage::<AppConfiguration>::new());
        StorageConfigRepository::new(storage)
    }

    fn create_log_repo() -> StorageExecutionLogRepository {
        let storage = Arc::new(InMemoryStorage::<ExecutionLog>::new());
        StorageExecutionLogRepository::new(storage)
    }

    #[tokio::test]
    async fn test_config_repository_get_defaults() {
        let repo = create_config_repo();
        let config = repo.get().await.unwrap();

        assert!(!config.is_persistence_enabled());
        assert_eq!(config.log_retention_days(), 30);
    }

    #[tokio::test]
    async fn test_config_repository_set_value() {
        let repo = create_config_repo();
        let key = ConfigKey::new("persistence.enabled").unwrap();

        repo.set(&key, ConfigValue::Boolean(true)).await.unwrap();

        let config = repo.get().await.unwrap();
        assert!(config.is_persistence_enabled());
    }

    #[tokio::test]
    async fn test_config_repository_reset() {
        let repo = create_config_repo();
        let key = ConfigKey::new("persistence.enabled").unwrap();

        repo.set(&key, ConfigValue::Boolean(true)).await.unwrap();
        repo.reset().await.unwrap();

        let config = repo.get().await.unwrap();
        assert!(!config.is_persistence_enabled());
    }

    #[tokio::test]
    async fn test_execution_log_repository_crud() {
        let repo = create_log_repo();
        let executor = Executor::from_api_key("test-key");
        let log = ExecutionLog::success(ExecutionType::Model, "gpt-4", 100, executor);
        let log_id = log.id().clone();

        // Save
        repo.save(&log).await.unwrap();

        // Get
        let retrieved = repo.get(&log_id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id().as_str(), log_id.as_str());

        // Delete
        let deleted = repo.delete(&log_id).await.unwrap();
        assert!(deleted);

        // Verify deleted
        let retrieved = repo.get(&log_id).await.unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_execution_log_repository_list_with_filters() {
        let repo = create_log_repo();

        // Create some logs
        let executor1 = Executor::from_api_key("key-1");
        let log1 = ExecutionLog::success(ExecutionType::Model, "gpt-4", 100, executor1);

        let executor2 = Executor::from_api_key("key-2");
        let log2 = ExecutionLog::failed(
            ExecutionType::Workflow,
            "my-workflow",
            "Error",
            200,
            executor2,
        );

        repo.save(&log1).await.unwrap();
        repo.save(&log2).await.unwrap();

        // List all
        let query = ExecutionLogQuery::new();
        let logs = repo.list(&query).await.unwrap();
        assert_eq!(logs.len(), 2);

        // Filter by type
        let query = ExecutionLogQuery::new().with_execution_type(ExecutionType::Model);
        let logs = repo.list(&query).await.unwrap();
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].execution_type(), ExecutionType::Model);

        // Filter by status
        let query = ExecutionLogQuery::new().with_status(ExecutionStatus::Failed);
        let logs = repo.list(&query).await.unwrap();
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].status(), ExecutionStatus::Failed);
    }

    #[tokio::test]
    async fn test_execution_log_repository_stats() {
        let repo = create_log_repo();

        let executor = Executor::from_api_key("key-1");
        let log1 = ExecutionLog::success(ExecutionType::Model, "gpt-4", 100, executor.clone())
            .with_cost(1000)
            .with_token_usage(TokenUsage::new(50, 30));

        let log2 = ExecutionLog::success(ExecutionType::Model, "gpt-4", 150, executor.clone())
            .with_cost(1500)
            .with_token_usage(TokenUsage::new(60, 40));

        let log3 = ExecutionLog::failed(ExecutionType::Workflow, "wf-1", "Error", 200, executor);

        repo.save(&log1).await.unwrap();
        repo.save(&log2).await.unwrap();
        repo.save(&log3).await.unwrap();

        let query = ExecutionLogQuery::new();
        let stats = repo.stats(&query).await.unwrap();

        assert_eq!(stats.total_executions, 3);
        assert_eq!(stats.successful_executions, 2);
        assert_eq!(stats.failed_executions, 1);
        assert_eq!(stats.total_cost_micros, 2500);
        assert_eq!(stats.total_input_tokens, 110);
        assert_eq!(stats.total_output_tokens, 70);
        assert!((stats.avg_execution_time_ms - 150.0).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_execution_log_repository_count() {
        let repo = create_log_repo();

        let executor = Executor::from_api_key("key-1");
        repo.save(&ExecutionLog::success(
            ExecutionType::Model,
            "m1",
            100,
            executor.clone(),
        ))
        .await
        .unwrap();
        repo.save(&ExecutionLog::success(
            ExecutionType::Model,
            "m2",
            100,
            executor.clone(),
        ))
        .await
        .unwrap();
        repo.save(&ExecutionLog::success(
            ExecutionType::Workflow,
            "w1",
            100,
            executor,
        ))
        .await
        .unwrap();

        let count = repo.count(&ExecutionLogQuery::new()).await.unwrap();
        assert_eq!(count, 3);

        let count = repo
            .count(&ExecutionLogQuery::new().with_execution_type(ExecutionType::Model))
            .await
            .unwrap();
        assert_eq!(count, 2);
    }

    #[tokio::test]
    async fn test_execution_log_repository_pagination() {
        let repo = create_log_repo();

        let executor = Executor::from_api_key("key-1");
        for i in 0..10 {
            repo.save(&ExecutionLog::success(
                ExecutionType::Model,
                format!("model-{}", i),
                100,
                executor.clone(),
            ))
            .await
            .unwrap();
        }

        // First page
        let query = ExecutionLogQuery::new().with_limit(3).with_offset(0);
        let logs = repo.list(&query).await.unwrap();
        assert_eq!(logs.len(), 3);

        // Second page
        let query = ExecutionLogQuery::new().with_limit(3).with_offset(3);
        let logs = repo.list(&query).await.unwrap();
        assert_eq!(logs.len(), 3);

        // Last page (partial)
        let query = ExecutionLogQuery::new().with_limit(3).with_offset(9);
        let logs = repo.list(&query).await.unwrap();
        assert_eq!(logs.len(), 1);
    }
}
