//! Execution log service - Records and queries execution history

use std::sync::Arc;

use crate::domain::{
    ConfigRepository, DomainError, ExecutionLog, ExecutionLogId, ExecutionLogQuery,
    ExecutionLogRepository, ExecutionStats, ExecutionStatus, ExecutionType, Executor,
    ExecutionTokenUsage, WorkflowStepLog,
};

/// Parameters for recording an execution
#[derive(Debug, Clone)]
pub struct RecordExecutionParams {
    pub execution_type: ExecutionType,
    pub resource_id: String,
    pub resource_name: Option<String>,
    pub status: ExecutionStatus,
    pub execution_time_ms: u64,
    pub executor: Executor,
    pub input: Option<serde_json::Value>,
    pub output: Option<serde_json::Value>,
    pub error: Option<String>,
    pub cost_micros: Option<i64>,
    pub token_usage: Option<ExecutionTokenUsage>,
    /// Whether this was an async execution
    pub is_async: bool,
    /// Workflow step logs (only for workflow executions)
    pub workflow_steps: Option<Vec<WorkflowStepLog>>,
}

impl RecordExecutionParams {
    /// Create params for a successful model execution
    pub fn model_success(
        model_id: impl Into<String>,
        execution_time_ms: u64,
        executor: Executor,
    ) -> Self {
        Self {
            execution_type: ExecutionType::Model,
            resource_id: model_id.into(),
            resource_name: None,
            status: ExecutionStatus::Success,
            execution_time_ms,
            executor,
            input: None,
            output: None,
            error: None,
            cost_micros: None,
            token_usage: None,
            is_async: false,
            workflow_steps: None,
        }
    }

    /// Create params for a failed model execution
    pub fn model_failed(
        model_id: impl Into<String>,
        error: impl Into<String>,
        execution_time_ms: u64,
        executor: Executor,
    ) -> Self {
        Self {
            execution_type: ExecutionType::Model,
            resource_id: model_id.into(),
            resource_name: None,
            status: ExecutionStatus::Failed,
            execution_time_ms,
            executor,
            input: None,
            output: None,
            error: Some(error.into()),
            cost_micros: None,
            token_usage: None,
            is_async: false,
            workflow_steps: None,
        }
    }

    /// Create params for a successful workflow execution
    pub fn workflow_success(
        workflow_id: impl Into<String>,
        execution_time_ms: u64,
        executor: Executor,
    ) -> Self {
        Self {
            execution_type: ExecutionType::Workflow,
            resource_id: workflow_id.into(),
            resource_name: None,
            status: ExecutionStatus::Success,
            execution_time_ms,
            executor,
            input: None,
            output: None,
            error: None,
            cost_micros: None,
            token_usage: None,
            is_async: false,
            workflow_steps: None,
        }
    }

    /// Create params for a failed workflow execution
    pub fn workflow_failed(
        workflow_id: impl Into<String>,
        error: impl Into<String>,
        execution_time_ms: u64,
        executor: Executor,
    ) -> Self {
        Self {
            execution_type: ExecutionType::Workflow,
            resource_id: workflow_id.into(),
            resource_name: None,
            status: ExecutionStatus::Failed,
            execution_time_ms,
            executor,
            input: None,
            output: None,
            error: Some(error.into()),
            cost_micros: None,
            token_usage: None,
            is_async: false,
            workflow_steps: None,
        }
    }

    /// Create params for a pending ingestion (async)
    pub fn ingestion_pending(kb_id: impl Into<String>, executor: Executor) -> Self {
        Self {
            execution_type: ExecutionType::Ingestion,
            resource_id: kb_id.into(),
            resource_name: None,
            status: ExecutionStatus::Pending,
            execution_time_ms: 0,
            executor,
            input: None,
            output: None,
            error: None,
            cost_micros: None,
            token_usage: None,
            is_async: true,
            workflow_steps: None,
        }
    }

    /// Create params for a successful ingestion
    pub fn ingestion_success(
        kb_id: impl Into<String>,
        execution_time_ms: u64,
        executor: Executor,
    ) -> Self {
        Self {
            execution_type: ExecutionType::Ingestion,
            resource_id: kb_id.into(),
            resource_name: None,
            status: ExecutionStatus::Success,
            execution_time_ms,
            executor,
            input: None,
            output: None,
            error: None,
            cost_micros: None,
            token_usage: None,
            is_async: true,
            workflow_steps: None,
        }
    }

    /// Create params for a failed ingestion
    pub fn ingestion_failed(
        kb_id: impl Into<String>,
        error: impl Into<String>,
        execution_time_ms: u64,
        executor: Executor,
    ) -> Self {
        Self {
            execution_type: ExecutionType::Ingestion,
            resource_id: kb_id.into(),
            resource_name: None,
            status: ExecutionStatus::Failed,
            execution_time_ms,
            executor,
            input: None,
            output: None,
            error: Some(error.into()),
            cost_micros: None,
            token_usage: None,
            is_async: true,
            workflow_steps: None,
        }
    }

    pub fn with_resource_name(mut self, name: impl Into<String>) -> Self {
        self.resource_name = Some(name.into());
        self
    }

    pub fn with_input(mut self, input: serde_json::Value) -> Self {
        self.input = Some(input);
        self
    }

    pub fn with_output(mut self, output: serde_json::Value) -> Self {
        self.output = Some(output);
        self
    }

    pub fn with_cost(mut self, cost_micros: i64) -> Self {
        self.cost_micros = Some(cost_micros);
        self
    }

    pub fn with_token_usage(mut self, usage: ExecutionTokenUsage) -> Self {
        self.token_usage = Some(usage);
        self
    }

    pub fn with_async(mut self, is_async: bool) -> Self {
        self.is_async = is_async;
        self
    }

    pub fn with_workflow_steps(mut self, steps: Vec<WorkflowStepLog>) -> Self {
        self.workflow_steps = Some(steps);
        self
    }
}

/// Execution log service for recording and querying execution history
pub struct ExecutionLogService {
    repository: Arc<dyn ExecutionLogRepository>,
    config_repository: Arc<dyn ConfigRepository>,
}

impl ExecutionLogService {
    /// Create a new ExecutionLogService
    pub fn new(
        repository: Arc<dyn ExecutionLogRepository>,
        config_repository: Arc<dyn ConfigRepository>,
    ) -> Self {
        Self {
            repository,
            config_repository,
        }
    }

    /// Record an execution (if logging is enabled for this resource)
    pub async fn record(&self, params: RecordExecutionParams) -> Result<Option<ExecutionLog>, DomainError> {
        // Check if logging is enabled for this resource
        let config = self.config_repository.get().await?;

        let should_log = match params.execution_type {
            ExecutionType::Model | ExecutionType::ChatCompletion => {
                config.should_log_model(&params.resource_id)
            }
            ExecutionType::Workflow => config.should_log_workflow(&params.resource_id),
            // Always log ingestion operations
            ExecutionType::Ingestion => true,
        };

        if !should_log {
            return Ok(None);
        }

        // Create the execution log
        let mut log = ExecutionLog::new(
            params.execution_type,
            params.resource_id,
            params.status,
            params.execution_time_ms,
            params.executor,
        );

        if let Some(name) = params.resource_name {
            log = log.with_resource_name(name);
        }

        // Only log input/output if sensitive data logging is enabled
        if config.log_sensitive_data() {
            if let Some(input) = params.input {
                log = log.with_input(input);
            }

            if let Some(output) = params.output {
                log = log.with_output(output);
            }
        }

        if let Some(error) = params.error {
            log = log.with_error(error);
        }

        if let Some(cost) = params.cost_micros {
            log = log.with_cost(cost);
        }

        if let Some(usage) = params.token_usage {
            log = log.with_token_usage(usage);
        }

        // Set async flag
        log = log.with_async(params.is_async);

        // Add workflow steps if present (and sensitive data logging is enabled)
        if config.log_sensitive_data() {
            if let Some(steps) = params.workflow_steps {
                log = log.with_workflow_steps(steps);
            }
        }

        // Save the log
        self.repository.save(&log).await?;

        Ok(Some(log))
    }

    /// Get an execution log by ID
    pub async fn get(&self, id: &str) -> Result<Option<ExecutionLog>, DomainError> {
        let log_id = ExecutionLogId::new(id)
            .map_err(|e| DomainError::validation(format!("Invalid log ID: {}", e)))?;
        self.repository.get(&log_id).await
    }

    /// List execution logs with optional filtering
    pub async fn list(&self, query: &ExecutionLogQuery) -> Result<Vec<ExecutionLog>, DomainError> {
        self.repository.list(query).await
    }

    /// Count execution logs matching query
    pub async fn count(&self, query: &ExecutionLogQuery) -> Result<usize, DomainError> {
        self.repository.count(query).await
    }

    /// Delete an execution log by ID
    pub async fn delete(&self, id: &str) -> Result<bool, DomainError> {
        let log_id = ExecutionLogId::new(id)
            .map_err(|e| DomainError::validation(format!("Invalid log ID: {}", e)))?;
        self.repository.delete(&log_id).await
    }

    /// Delete logs older than the configured retention period
    pub async fn cleanup_old_logs(&self) -> Result<usize, DomainError> {
        let config = self.config_repository.get().await?;
        let retention_days = config.log_retention_days();
        self.repository.delete_older_than(retention_days).await
    }

    /// Delete logs older than specified days
    pub async fn delete_older_than(&self, days: i64) -> Result<usize, DomainError> {
        self.repository.delete_older_than(days).await
    }

    /// Get aggregated statistics
    pub async fn stats(&self, query: &ExecutionLogQuery) -> Result<ExecutionStats, DomainError> {
        self.repository.stats(query).await
    }

    /// Update an existing execution log (used for async operations)
    pub async fn update(&self, log: &ExecutionLog) -> Result<(), DomainError> {
        self.repository.save(log).await
    }

    /// Record a pending ingestion and return the log
    /// Used for async ingestion operations
    pub async fn record_pending_ingestion(
        &self,
        kb_id: impl Into<String>,
        source_name: impl Into<String>,
        executor: Executor,
        input: serde_json::Value,
    ) -> Result<ExecutionLog, DomainError> {
        let params = RecordExecutionParams::ingestion_pending(kb_id, executor)
            .with_resource_name(source_name)
            .with_input(input);

        // Record always logs ingestion operations
        let log = self.record(params).await?;

        log.ok_or_else(|| DomainError::internal("Failed to create ingestion log"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{AppConfiguration, ExecutionLog};
    use crate::infrastructure::config::{StorageConfigRepository, StorageExecutionLogRepository};
    use crate::infrastructure::storage::InMemoryStorage;
    use crate::domain::ConfigValue;

    fn create_service() -> (ExecutionLogService, Arc<dyn ConfigRepository>) {
        let config_storage = Arc::new(InMemoryStorage::<AppConfiguration>::new());
        let config_repo: Arc<dyn ConfigRepository> =
            Arc::new(StorageConfigRepository::new(config_storage));

        let log_storage = Arc::new(InMemoryStorage::<ExecutionLog>::new());
        let log_repo: Arc<dyn ExecutionLogRepository> =
            Arc::new(StorageExecutionLogRepository::new(log_storage));

        let service = ExecutionLogService::new(log_repo, config_repo.clone());
        (service, config_repo)
    }

    #[tokio::test]
    async fn test_record_when_disabled() {
        let (service, _config) = create_service();

        let params = RecordExecutionParams::model_success("gpt-4", 100, Executor::anonymous());
        let result = service.record(params).await.unwrap();

        // Logging is disabled by default
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_record_when_enabled() {
        let (service, config_repo) = create_service();

        // Enable persistence
        let key = crate::domain::ConfigKey::new("persistence.enabled").unwrap();
        config_repo.set(&key, ConfigValue::Boolean(true)).await.unwrap();

        let params = RecordExecutionParams::model_success("gpt-4", 100, Executor::from_api_key("key-1"));
        let result = service.record(params).await.unwrap();

        assert!(result.is_some());
        let log = result.unwrap();
        assert_eq!(log.execution_type(), ExecutionType::Model);
        assert_eq!(log.resource_id(), "gpt-4");
        assert_eq!(log.status(), ExecutionStatus::Success);
    }

    #[tokio::test]
    async fn test_record_respects_model_filter() {
        let (service, config_repo) = create_service();

        // Enable persistence only for specific models
        let key = crate::domain::ConfigKey::new("persistence.enabled").unwrap();
        config_repo.set(&key, ConfigValue::Boolean(true)).await.unwrap();

        let key = crate::domain::ConfigKey::new("persistence.enabled_models").unwrap();
        config_repo
            .set(&key, ConfigValue::StringList(vec!["gpt-4".to_string()]))
            .await
            .unwrap();

        // This should be logged
        let params = RecordExecutionParams::model_success("gpt-4", 100, Executor::anonymous());
        let result = service.record(params).await.unwrap();
        assert!(result.is_some());

        // This should NOT be logged
        let params = RecordExecutionParams::model_success("claude-3", 100, Executor::anonymous());
        let result = service.record(params).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_record_sensitive_data() {
        let (service, config_repo) = create_service();

        // Enable persistence but not sensitive data
        let key = crate::domain::ConfigKey::new("persistence.enabled").unwrap();
        config_repo.set(&key, ConfigValue::Boolean(true)).await.unwrap();

        let params = RecordExecutionParams::model_success("gpt-4", 100, Executor::anonymous())
            .with_input(serde_json::json!({"prompt": "secret"}))
            .with_output(serde_json::json!({"response": "answer"}));

        let result = service.record(params).await.unwrap().unwrap();
        assert!(result.input().is_none()); // Not logged
        assert!(result.output().is_none()); // Not logged

        // Now enable sensitive data logging
        let key = crate::domain::ConfigKey::new("persistence.log_sensitive_data").unwrap();
        config_repo.set(&key, ConfigValue::Boolean(true)).await.unwrap();

        let params = RecordExecutionParams::model_success("gpt-4", 100, Executor::anonymous())
            .with_input(serde_json::json!({"prompt": "secret"}))
            .with_output(serde_json::json!({"response": "answer"}));

        let result = service.record(params).await.unwrap().unwrap();
        assert!(result.input().is_some()); // Logged
        assert!(result.output().is_some()); // Logged
    }

    #[tokio::test]
    async fn test_list_and_stats() {
        let (service, config_repo) = create_service();

        // Enable persistence
        let key = crate::domain::ConfigKey::new("persistence.enabled").unwrap();
        config_repo.set(&key, ConfigValue::Boolean(true)).await.unwrap();

        // Record some executions
        for i in 0..5 {
            let params = RecordExecutionParams::model_success(
                format!("model-{}", i),
                100,
                Executor::anonymous(),
            );
            service.record(params).await.unwrap();
        }

        // List
        let query = ExecutionLogQuery::new();
        let logs = service.list(&query).await.unwrap();
        assert_eq!(logs.len(), 5);

        // Count
        let count = service.count(&query).await.unwrap();
        assert_eq!(count, 5);

        // Stats
        let stats = service.stats(&query).await.unwrap();
        assert_eq!(stats.total_executions, 5);
        assert_eq!(stats.successful_executions, 5);
    }

    #[tokio::test]
    async fn test_delete() {
        let (service, config_repo) = create_service();

        // Enable persistence
        let key = crate::domain::ConfigKey::new("persistence.enabled").unwrap();
        config_repo.set(&key, ConfigValue::Boolean(true)).await.unwrap();

        let params = RecordExecutionParams::model_success("gpt-4", 100, Executor::anonymous());
        let log = service.record(params).await.unwrap().unwrap();
        let log_id = log.id().as_str().to_string();

        // Delete
        let deleted = service.delete(&log_id).await.unwrap();
        assert!(deleted);

        // Verify deleted
        let result = service.get(&log_id).await.unwrap();
        assert!(result.is_none());
    }
}
