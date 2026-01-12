//! Operation domain entities

use std::fmt;

use chrono::{DateTime, Utc};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::error::OperationError;
use crate::domain::storage::{StorageEntity, StorageKey};

/// Regex pattern for valid operation IDs: op-{uuid}
static ID_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^op-[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}$").unwrap());

/// Maximum length for operation IDs
pub const MAX_ID_LENGTH: usize = 39; // "op-" + 36 char UUID

/// Validated operation identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct OperationId(String);

impl OperationId {
    /// Create a new validated operation ID
    pub fn new(id: impl Into<String>) -> Result<Self, OperationError> {
        let id = id.into();
        validate_operation_id(&id)?;
        Ok(Self(id))
    }

    /// Generate a new operation ID with UUID
    pub fn generate() -> Self {
        let uuid = uuid::Uuid::new_v4();
        Self(format!("op-{}", uuid))
    }

    /// Get the ID as a string slice
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for OperationId {
    type Error = OperationError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<OperationId> for String {
    fn from(id: OperationId) -> Self {
        id.0
    }
}

impl fmt::Display for OperationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for OperationId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl StorageKey for OperationId {
    fn as_str(&self) -> &str {
        &self.0
    }
}

impl StorageEntity for Operation {
    type Key = OperationId;

    fn key(&self) -> &Self::Key {
        &self.id
    }
}

/// Validate an operation ID string
pub fn validate_operation_id(id: &str) -> Result<(), OperationError> {
    if id.is_empty() {
        return Err(OperationError::invalid_id("Operation ID cannot be empty"));
    }

    if id.len() > MAX_ID_LENGTH {
        return Err(OperationError::invalid_id(format!(
            "Operation ID exceeds maximum length of {} characters",
            MAX_ID_LENGTH
        )));
    }

    if !ID_PATTERN.is_match(id) {
        return Err(OperationError::invalid_id(format!(
            "Invalid operation ID '{}': must be in format op-{{uuid}}",
            id
        )));
    }

    Ok(())
}

/// Status of an async operation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum OperationStatus {
    /// Operation is queued but not yet started
    #[default]
    Pending,

    /// Operation is currently running
    Running,

    /// Operation completed successfully
    Completed,

    /// Operation failed with an error
    Failed,

    /// Operation was cancelled by the user
    Cancelled,
}

impl OperationStatus {
    /// Check if this status represents a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }

    /// Check if this status can transition to another status
    pub fn can_transition_to(&self, target: OperationStatus) -> bool {
        match (self, target) {
            // From Pending
            (Self::Pending, Self::Running) => true,
            (Self::Pending, Self::Cancelled) => true,

            // From Running
            (Self::Running, Self::Completed) => true,
            (Self::Running, Self::Failed) => true,
            (Self::Running, Self::Cancelled) => true,

            // Terminal states cannot transition
            (Self::Completed, _) => false,
            (Self::Failed, _) => false,
            (Self::Cancelled, _) => false,

            // Same state is not a valid transition
            _ => false,
        }
    }
}

impl fmt::Display for OperationStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Running => write!(f, "running"),
            Self::Completed => write!(f, "completed"),
            Self::Failed => write!(f, "failed"),
            Self::Cancelled => write!(f, "cancelled"),
        }
    }
}

/// Type of async operation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationType {
    /// Chat completion request
    ChatCompletion,

    /// Workflow execution
    WorkflowExecution,
}

impl fmt::Display for OperationType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ChatCompletion => write!(f, "chat_completion"),
            Self::WorkflowExecution => write!(f, "workflow_execution"),
        }
    }
}

/// An async operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Operation {
    /// Unique operation identifier
    id: OperationId,

    /// Type of operation
    operation_type: OperationType,

    /// Current status
    status: OperationStatus,

    /// Original request input
    input: Value,

    /// Success result (if completed)
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,

    /// Error message (if failed)
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,

    /// Additional metadata (model_id, workflow_id, etc.)
    metadata: Value,

    /// When the operation was created
    created_at: DateTime<Utc>,

    /// When the operation started running
    #[serde(skip_serializing_if = "Option::is_none")]
    started_at: Option<DateTime<Utc>>,

    /// When the operation completed/failed/cancelled
    #[serde(skip_serializing_if = "Option::is_none")]
    completed_at: Option<DateTime<Utc>>,
}

impl Operation {
    /// Create a new pending operation
    pub fn new(operation_type: OperationType, input: Value, metadata: Value) -> Self {
        Self {
            id: OperationId::generate(),
            operation_type,
            status: OperationStatus::Pending,
            input,
            result: None,
            error: None,
            metadata,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
        }
    }

    /// Create operation with specific ID (for testing)
    pub fn with_id(id: OperationId, operation_type: OperationType, input: Value) -> Self {
        Self {
            id,
            operation_type,
            status: OperationStatus::Pending,
            input,
            result: None,
            error: None,
            metadata: Value::Null,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
        }
    }

    // Getters

    pub fn id(&self) -> &OperationId {
        &self.id
    }

    pub fn operation_type(&self) -> OperationType {
        self.operation_type
    }

    pub fn status(&self) -> OperationStatus {
        self.status
    }

    pub fn input(&self) -> &Value {
        &self.input
    }

    pub fn result(&self) -> Option<&Value> {
        self.result.as_ref()
    }

    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub fn metadata(&self) -> &Value {
        &self.metadata
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn started_at(&self) -> Option<DateTime<Utc>> {
        self.started_at
    }

    pub fn completed_at(&self) -> Option<DateTime<Utc>> {
        self.completed_at
    }

    pub fn is_terminal(&self) -> bool {
        self.status.is_terminal()
    }

    /// Mark operation as running
    pub fn mark_running(&mut self) -> Result<(), OperationError> {
        if !self.status.can_transition_to(OperationStatus::Running) {
            return Err(OperationError::invalid_transition(
                &self.status.to_string(),
                "running",
                "Operation is not in pending state",
            ));
        }
        self.status = OperationStatus::Running;
        self.started_at = Some(Utc::now());
        Ok(())
    }

    /// Mark operation as completed with result
    pub fn mark_completed(&mut self, result: Value) -> Result<(), OperationError> {
        if !self.status.can_transition_to(OperationStatus::Completed) {
            return Err(OperationError::invalid_transition(
                &self.status.to_string(),
                "completed",
                "Operation is not in running state",
            ));
        }
        self.status = OperationStatus::Completed;
        self.result = Some(result);
        self.completed_at = Some(Utc::now());
        Ok(())
    }

    /// Mark operation as failed with error
    pub fn mark_failed(&mut self, error: impl Into<String>) -> Result<(), OperationError> {
        if !self.status.can_transition_to(OperationStatus::Failed) {
            return Err(OperationError::invalid_transition(
                &self.status.to_string(),
                "failed",
                "Operation is not in running state",
            ));
        }
        self.status = OperationStatus::Failed;
        self.error = Some(error.into());
        self.completed_at = Some(Utc::now());
        Ok(())
    }

    /// Mark operation as cancelled
    pub fn mark_cancelled(&mut self) -> Result<(), OperationError> {
        if !self.status.can_transition_to(OperationStatus::Cancelled) {
            return Err(OperationError::cannot_cancel(format!(
                "Operation in '{}' state cannot be cancelled",
                self.status
            )));
        }
        self.status = OperationStatus::Cancelled;
        self.completed_at = Some(Utc::now());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_operation_id_generate() {
        let id = OperationId::generate();
        assert!(id.as_str().starts_with("op-"));
        assert_eq!(id.as_str().len(), 39); // "op-" + 36 char UUID
    }

    #[test]
    fn test_operation_id_valid() {
        let id = OperationId::new("op-12345678-1234-1234-1234-123456789abc");
        assert!(id.is_ok());
    }

    #[test]
    fn test_operation_id_invalid() {
        assert!(OperationId::new("").is_err());
        assert!(OperationId::new("invalid").is_err());
        assert!(OperationId::new("op-invalid").is_err());
        assert!(OperationId::new("12345678-1234-1234-1234-123456789abc").is_err());
    }

    #[test]
    fn test_operation_id_serialization() {
        let id = OperationId::generate();
        let json = serde_json::to_string(&id).unwrap();
        assert!(json.starts_with("\"op-"));

        let deserialized: OperationId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, deserialized);
    }

    #[test]
    fn test_operation_status_terminal() {
        assert!(!OperationStatus::Pending.is_terminal());
        assert!(!OperationStatus::Running.is_terminal());
        assert!(OperationStatus::Completed.is_terminal());
        assert!(OperationStatus::Failed.is_terminal());
        assert!(OperationStatus::Cancelled.is_terminal());
    }

    #[test]
    fn test_operation_status_transitions() {
        // Valid transitions from Pending
        assert!(OperationStatus::Pending.can_transition_to(OperationStatus::Running));
        assert!(OperationStatus::Pending.can_transition_to(OperationStatus::Cancelled));
        assert!(!OperationStatus::Pending.can_transition_to(OperationStatus::Completed));

        // Valid transitions from Running
        assert!(OperationStatus::Running.can_transition_to(OperationStatus::Completed));
        assert!(OperationStatus::Running.can_transition_to(OperationStatus::Failed));
        assert!(OperationStatus::Running.can_transition_to(OperationStatus::Cancelled));

        // No transitions from terminal states
        assert!(!OperationStatus::Completed.can_transition_to(OperationStatus::Running));
        assert!(!OperationStatus::Failed.can_transition_to(OperationStatus::Running));
        assert!(!OperationStatus::Cancelled.can_transition_to(OperationStatus::Running));
    }

    #[test]
    fn test_operation_creation() {
        let op = Operation::new(
            OperationType::ChatCompletion,
            json!({"model": "gpt-4"}),
            json!({"request_id": "123"}),
        );

        assert!(op.id().as_str().starts_with("op-"));
        assert_eq!(op.operation_type(), OperationType::ChatCompletion);
        assert_eq!(op.status(), OperationStatus::Pending);
        assert!(op.result().is_none());
        assert!(op.error().is_none());
        assert!(op.started_at().is_none());
        assert!(op.completed_at().is_none());
    }

    #[test]
    fn test_operation_lifecycle() {
        let mut op = Operation::new(
            OperationType::WorkflowExecution,
            json!({"workflow_id": "test"}),
            json!({}),
        );

        // Start running
        assert!(op.mark_running().is_ok());
        assert_eq!(op.status(), OperationStatus::Running);
        assert!(op.started_at().is_some());

        // Complete
        assert!(op.mark_completed(json!({"answer": "42"})).is_ok());
        assert_eq!(op.status(), OperationStatus::Completed);
        assert!(op.completed_at().is_some());
        assert_eq!(op.result(), Some(&json!({"answer": "42"})));
    }

    #[test]
    fn test_operation_failure() {
        let mut op = Operation::new(
            OperationType::ChatCompletion,
            json!({}),
            json!({}),
        );

        op.mark_running().unwrap();
        assert!(op.mark_failed("Something went wrong").is_ok());
        assert_eq!(op.status(), OperationStatus::Failed);
        assert_eq!(op.error(), Some("Something went wrong"));
    }

    #[test]
    fn test_operation_cancellation() {
        let mut op = Operation::new(
            OperationType::ChatCompletion,
            json!({}),
            json!({}),
        );

        // Cancel from pending
        assert!(op.mark_cancelled().is_ok());
        assert_eq!(op.status(), OperationStatus::Cancelled);
    }

    #[test]
    fn test_invalid_state_transitions() {
        let mut op = Operation::new(
            OperationType::ChatCompletion,
            json!({}),
            json!({}),
        );

        // Cannot complete from pending
        assert!(op.mark_completed(json!({})).is_err());

        // Cannot fail from pending
        assert!(op.mark_failed("error").is_err());

        // Start and complete
        op.mark_running().unwrap();
        op.mark_completed(json!({})).unwrap();

        // Cannot transition from completed
        assert!(op.mark_running().is_err());
        assert!(op.mark_cancelled().is_err());
    }

    #[test]
    fn test_operation_serialization() {
        let op = Operation::new(
            OperationType::ChatCompletion,
            json!({"model": "gpt-4"}),
            json!({"user": "test"}),
        );

        let json = serde_json::to_string(&op).unwrap();
        assert!(json.contains("\"status\":\"pending\""));
        assert!(json.contains("\"operation_type\":\"chat_completion\""));

        let deserialized: Operation = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id(), op.id());
        assert_eq!(deserialized.status(), op.status());
    }
}
