//! Execution log domain entities

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::storage::{StorageEntity, StorageKey};

/// Execution log ID
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ExecutionLogId(String);

impl StorageKey for ExecutionLogId {
    fn as_str(&self) -> &str {
        &self.0
    }
}

impl ExecutionLogId {
    pub fn new(id: impl Into<String>) -> Result<Self, ExecutionLogValidationError> {
        let id = id.into();
        validate_execution_log_id(&id)?;
        Ok(Self(id))
    }

    pub fn generate() -> Self {
        Self(format!("log-{}", Uuid::new_v4()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ExecutionLogId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Type of execution being logged
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionType {
    Model,
    Workflow,
    ChatCompletion,
    Ingestion,
}

impl ExecutionType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ExecutionType::Model => "model",
            ExecutionType::Workflow => "workflow",
            ExecutionType::ChatCompletion => "chat_completion",
            ExecutionType::Ingestion => "ingestion",
        }
    }
}

impl std::fmt::Display for ExecutionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Execution status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    Pending,
    InProgress,
    Success,
    Failed,
    Timeout,
    Cancelled,
}

impl ExecutionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ExecutionStatus::Pending => "pending",
            ExecutionStatus::InProgress => "in_progress",
            ExecutionStatus::Success => "success",
            ExecutionStatus::Failed => "failed",
            ExecutionStatus::Timeout => "timeout",
            ExecutionStatus::Cancelled => "cancelled",
        }
    }

    pub fn is_success(&self) -> bool {
        matches!(self, ExecutionStatus::Success)
    }

    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            ExecutionStatus::Success
                | ExecutionStatus::Failed
                | ExecutionStatus::Timeout
                | ExecutionStatus::Cancelled
        )
    }
}

impl std::fmt::Display for ExecutionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Executor information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Executor {
    /// User ID (if authenticated via JWT)
    pub user_id: Option<String>,
    /// API key ID (if authenticated via API key)
    pub api_key_id: Option<String>,
    /// IP address of the requester
    pub ip_address: Option<String>,
    /// User agent string
    pub user_agent: Option<String>,
}

impl Executor {
    pub fn from_api_key(api_key_id: impl Into<String>) -> Self {
        Self {
            user_id: None,
            api_key_id: Some(api_key_id.into()),
            ip_address: None,
            user_agent: None,
        }
    }

    pub fn from_user(user_id: impl Into<String>) -> Self {
        Self {
            user_id: Some(user_id.into()),
            api_key_id: None,
            ip_address: None,
            user_agent: None,
        }
    }

    pub fn anonymous() -> Self {
        Self {
            user_id: None,
            api_key_id: None,
            ip_address: None,
            user_agent: None,
        }
    }

    pub fn with_ip(mut self, ip: impl Into<String>) -> Self {
        self.ip_address = Some(ip.into());
        self
    }

    pub fn with_user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.user_agent = Some(user_agent.into());
        self
    }
}

/// Token usage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub total_tokens: u32,
}

impl TokenUsage {
    pub fn new(input_tokens: u32, output_tokens: u32) -> Self {
        Self {
            input_tokens,
            output_tokens,
            total_tokens: input_tokens + output_tokens,
        }
    }
}

/// Workflow step execution log
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStepLog {
    /// Step name/identifier
    pub step_name: String,
    /// Step type (e.g., "chat_completion", "knowledge_base_search")
    pub step_type: String,
    /// Step input data
    pub input: Option<serde_json::Value>,
    /// Step output data
    pub output: Option<serde_json::Value>,
    /// Error if step failed
    pub error: Option<String>,
    /// Step execution time in milliseconds
    pub execution_time_ms: u64,
    /// Step status
    pub status: ExecutionStatus,
}

impl WorkflowStepLog {
    pub fn new(step_name: impl Into<String>, step_type: impl Into<String>) -> Self {
        Self {
            step_name: step_name.into(),
            step_type: step_type.into(),
            input: None,
            output: None,
            error: None,
            execution_time_ms: 0,
            status: ExecutionStatus::Success,
        }
    }

    pub fn with_input(mut self, input: serde_json::Value) -> Self {
        self.input = Some(input);
        self
    }

    pub fn with_output(mut self, output: serde_json::Value) -> Self {
        self.output = Some(output);
        self
    }

    pub fn with_error(mut self, error: impl Into<String>) -> Self {
        self.error = Some(error.into());
        self.status = ExecutionStatus::Failed;
        self
    }

    pub fn with_execution_time(mut self, ms: u64) -> Self {
        self.execution_time_ms = ms;
        self
    }

    pub fn with_status(mut self, status: ExecutionStatus) -> Self {
        self.status = status;
        self
    }
}

/// Execution log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionLog {
    id: ExecutionLogId,
    execution_type: ExecutionType,
    resource_id: String,
    resource_name: Option<String>,
    status: ExecutionStatus,
    input: Option<serde_json::Value>,
    output: Option<serde_json::Value>,
    error: Option<String>,
    cost_micros: Option<i64>,
    token_usage: Option<TokenUsage>,
    execution_time_ms: u64,
    executor: Executor,
    created_at: DateTime<Utc>,
    /// Whether this was an async execution
    #[serde(default)]
    is_async: bool,
    /// Workflow step logs (only for workflow executions)
    #[serde(default)]
    workflow_steps: Option<Vec<WorkflowStepLog>>,
}

impl StorageEntity for ExecutionLog {
    type Key = ExecutionLogId;

    fn key(&self) -> &Self::Key {
        &self.id
    }
}

impl ExecutionLog {
    pub fn new(
        execution_type: ExecutionType,
        resource_id: impl Into<String>,
        status: ExecutionStatus,
        execution_time_ms: u64,
        executor: Executor,
    ) -> Self {
        Self {
            id: ExecutionLogId::generate(),
            execution_type,
            resource_id: resource_id.into(),
            resource_name: None,
            status,
            input: None,
            output: None,
            error: None,
            cost_micros: None,
            token_usage: None,
            execution_time_ms,
            executor,
            created_at: Utc::now(),
            is_async: false,
            workflow_steps: None,
        }
    }

    pub fn success(
        execution_type: ExecutionType,
        resource_id: impl Into<String>,
        execution_time_ms: u64,
        executor: Executor,
    ) -> Self {
        Self::new(
            execution_type,
            resource_id,
            ExecutionStatus::Success,
            execution_time_ms,
            executor,
        )
    }

    pub fn failed(
        execution_type: ExecutionType,
        resource_id: impl Into<String>,
        error: impl Into<String>,
        execution_time_ms: u64,
        executor: Executor,
    ) -> Self {
        let mut log = Self::new(
            execution_type,
            resource_id,
            ExecutionStatus::Failed,
            execution_time_ms,
            executor,
        );
        log.error = Some(error.into());
        log
    }

    /// Create a pending execution log for async operations
    pub fn pending(
        execution_type: ExecutionType,
        resource_id: impl Into<String>,
        executor: Executor,
    ) -> Self {
        Self::new(execution_type, resource_id, ExecutionStatus::Pending, 0, executor)
            .with_async(true)
    }

    // Getters

    pub fn id(&self) -> &ExecutionLogId {
        &self.id
    }

    pub fn execution_type(&self) -> ExecutionType {
        self.execution_type
    }

    pub fn resource_id(&self) -> &str {
        &self.resource_id
    }

    pub fn resource_name(&self) -> Option<&str> {
        self.resource_name.as_deref()
    }

    pub fn status(&self) -> ExecutionStatus {
        self.status
    }

    pub fn input(&self) -> Option<&serde_json::Value> {
        self.input.as_ref()
    }

    pub fn output(&self) -> Option<&serde_json::Value> {
        self.output.as_ref()
    }

    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub fn cost_micros(&self) -> Option<i64> {
        self.cost_micros
    }

    pub fn token_usage(&self) -> Option<&TokenUsage> {
        self.token_usage.as_ref()
    }

    pub fn execution_time_ms(&self) -> u64 {
        self.execution_time_ms
    }

    pub fn executor(&self) -> &Executor {
        &self.executor
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn is_async(&self) -> bool {
        self.is_async
    }

    pub fn workflow_steps(&self) -> Option<&Vec<WorkflowStepLog>> {
        self.workflow_steps.as_ref()
    }

    // Builder methods

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

    pub fn with_error(mut self, error: impl Into<String>) -> Self {
        self.error = Some(error.into());
        self
    }

    pub fn with_cost(mut self, cost_micros: i64) -> Self {
        self.cost_micros = Some(cost_micros);
        self
    }

    pub fn with_token_usage(mut self, usage: TokenUsage) -> Self {
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

    pub fn add_workflow_step(&mut self, step: WorkflowStepLog) {
        if self.workflow_steps.is_none() {
            self.workflow_steps = Some(Vec::new());
        }

        if let Some(steps) = &mut self.workflow_steps {
            steps.push(step);
        }
    }

    // Mutators for updating async operations

    /// Set status to in progress
    pub fn set_in_progress(&mut self) {
        self.status = ExecutionStatus::InProgress;
    }

    /// Mark as successful with execution time and output
    pub fn set_success(&mut self, execution_time_ms: u64, output: Option<serde_json::Value>) {
        self.status = ExecutionStatus::Success;
        self.execution_time_ms = execution_time_ms;
        self.output = output;
    }

    /// Mark as failed with error message
    pub fn set_failed(&mut self, execution_time_ms: u64, error: impl Into<String>) {
        self.status = ExecutionStatus::Failed;
        self.execution_time_ms = execution_time_ms;
        self.error = Some(error.into());
    }
}

/// Execution log validation errors
#[derive(Debug, Clone, thiserror::Error)]
pub enum ExecutionLogValidationError {
    #[error("Invalid execution log ID: {0}")]
    InvalidId(String),
}

fn validate_execution_log_id(id: &str) -> Result<(), ExecutionLogValidationError> {
    if id.is_empty() {
        return Err(ExecutionLogValidationError::InvalidId(
            "ID cannot be empty".to_string(),
        ));
    }

    if id.len() > 50 {
        return Err(ExecutionLogValidationError::InvalidId(
            "ID cannot exceed 50 characters".to_string(),
        ));
    }

    Ok(())
}

/// Query parameters for listing execution logs
#[derive(Debug, Clone, Default)]
pub struct ExecutionLogQuery {
    pub execution_type: Option<ExecutionType>,
    pub resource_id: Option<String>,
    pub status: Option<ExecutionStatus>,
    pub api_key_id: Option<String>,
    pub user_id: Option<String>,
    pub from_date: Option<DateTime<Utc>>,
    pub to_date: Option<DateTime<Utc>>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

impl ExecutionLogQuery {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_execution_type(mut self, execution_type: ExecutionType) -> Self {
        self.execution_type = Some(execution_type);
        self
    }

    pub fn with_resource_id(mut self, resource_id: impl Into<String>) -> Self {
        self.resource_id = Some(resource_id.into());
        self
    }

    pub fn with_status(mut self, status: ExecutionStatus) -> Self {
        self.status = Some(status);
        self
    }

    pub fn with_api_key_id(mut self, api_key_id: impl Into<String>) -> Self {
        self.api_key_id = Some(api_key_id.into());
        self
    }

    pub fn with_user_id(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    pub fn with_date_range(mut self, from: DateTime<Utc>, to: DateTime<Utc>) -> Self {
        self.from_date = Some(from);
        self.to_date = Some(to);
        self
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn with_offset(mut self, offset: usize) -> Self {
        self.offset = Some(offset);
        self
    }
}

/// Aggregated execution statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStats {
    pub total_executions: usize,
    pub successful_executions: usize,
    pub failed_executions: usize,
    pub total_cost_micros: i64,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub avg_execution_time_ms: f64,
    pub executions_by_type: std::collections::HashMap<String, usize>,
    pub executions_by_resource: std::collections::HashMap<String, usize>,
}

impl ExecutionStats {
    pub fn empty() -> Self {
        Self {
            total_executions: 0,
            successful_executions: 0,
            failed_executions: 0,
            total_cost_micros: 0,
            total_input_tokens: 0,
            total_output_tokens: 0,
            avg_execution_time_ms: 0.0,
            executions_by_type: std::collections::HashMap::new(),
            executions_by_resource: std::collections::HashMap::new(),
        }
    }

    pub fn success_rate(&self) -> f64 {
        if self.total_executions == 0 {
            return 0.0;
        }

        (self.successful_executions as f64 / self.total_executions as f64) * 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_log_id() {
        let id = ExecutionLogId::generate();
        assert!(id.as_str().starts_with("log-"));

        let id = ExecutionLogId::new("custom-id").unwrap();
        assert_eq!(id.as_str(), "custom-id");
    }

    #[test]
    fn test_execution_log_success() {
        let executor = Executor::from_api_key("key-123");
        let log = ExecutionLog::success(ExecutionType::Model, "gpt-4", 150, executor);

        assert_eq!(log.execution_type(), ExecutionType::Model);
        assert_eq!(log.resource_id(), "gpt-4");
        assert_eq!(log.status(), ExecutionStatus::Success);
        assert!(log.status().is_success());
        assert_eq!(log.execution_time_ms(), 150);
    }

    #[test]
    fn test_execution_log_failed() {
        let executor = Executor::from_user("user-456");
        let log = ExecutionLog::failed(
            ExecutionType::Workflow,
            "my-workflow",
            "Timeout occurred",
            5000,
            executor,
        );

        assert_eq!(log.execution_type(), ExecutionType::Workflow);
        assert_eq!(log.status(), ExecutionStatus::Failed);
        assert!(!log.status().is_success());
        assert_eq!(log.error(), Some("Timeout occurred"));
    }

    #[test]
    fn test_execution_log_builder() {
        let executor = Executor::from_api_key("key-789").with_ip("192.168.1.1");
        let log = ExecutionLog::success(ExecutionType::ChatCompletion, "gpt-4", 200, executor)
            .with_resource_name("GPT-4")
            .with_input(serde_json::json!({"messages": []}))
            .with_output(serde_json::json!({"content": "Hello"}))
            .with_cost(1500)
            .with_token_usage(TokenUsage::new(100, 50));

        assert_eq!(log.resource_name(), Some("GPT-4"));
        assert!(log.input().is_some());
        assert!(log.output().is_some());
        assert_eq!(log.cost_micros(), Some(1500));
        assert!(log.token_usage().is_some());
        assert_eq!(log.token_usage().unwrap().total_tokens, 150);
    }

    #[test]
    fn test_executor() {
        let executor = Executor::from_api_key("key-1")
            .with_ip("10.0.0.1")
            .with_user_agent("TestClient/1.0");

        assert_eq!(executor.api_key_id, Some("key-1".to_string()));
        assert_eq!(executor.ip_address, Some("10.0.0.1".to_string()));
        assert_eq!(executor.user_agent, Some("TestClient/1.0".to_string()));
        assert!(executor.user_id.is_none());
    }

    #[test]
    fn test_execution_stats() {
        let mut stats = ExecutionStats::empty();
        stats.total_executions = 100;
        stats.successful_executions = 95;

        assert_eq!(stats.success_rate(), 95.0);
    }

    #[test]
    fn test_execution_log_query() {
        let query = ExecutionLogQuery::new()
            .with_execution_type(ExecutionType::Model)
            .with_resource_id("gpt-4")
            .with_status(ExecutionStatus::Success)
            .with_limit(10);

        assert_eq!(query.execution_type, Some(ExecutionType::Model));
        assert_eq!(query.resource_id, Some("gpt-4".to_string()));
        assert_eq!(query.status, Some(ExecutionStatus::Success));
        assert_eq!(query.limit, Some(10));
    }
}
