//! Workflow error types

use thiserror::Error;

/// Errors that can occur during workflow operations
#[derive(Debug, Clone, Error, PartialEq)]
pub enum WorkflowError {
    #[error("Workflow not found: {0}")]
    NotFound(String),

    #[error("Step not found: {0}")]
    StepNotFound(String),

    #[error("Variable resolution failed: {0}")]
    VariableResolution(String),

    #[error("Step execution failed in '{step}': {message}")]
    StepExecution { step: String, message: String },

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Circular reference detected: {0}")]
    CircularReference(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Workflow is disabled: {0}")]
    Disabled(String),

    #[error("Workflow has no steps: {0}")]
    EmptyWorkflow(String),

    #[error("Schema validation failed: {0}")]
    SchemaValidation(String),

    #[error("Timeout in step '{step}' after {timeout_ms}ms")]
    Timeout { step: String, timeout_ms: u64 },

    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),
}

impl WorkflowError {
    pub fn not_found(id: impl Into<String>) -> Self {
        Self::NotFound(id.into())
    }

    pub fn step_not_found(name: impl Into<String>) -> Self {
        Self::StepNotFound(name.into())
    }

    pub fn variable_resolution(message: impl Into<String>) -> Self {
        Self::VariableResolution(message.into())
    }

    pub fn step_execution(step: impl Into<String>, message: impl Into<String>) -> Self {
        Self::StepExecution {
            step: step.into(),
            message: message.into(),
        }
    }

    pub fn validation(message: impl Into<String>) -> Self {
        Self::Validation(message.into())
    }

    pub fn circular_reference(message: impl Into<String>) -> Self {
        Self::CircularReference(message.into())
    }

    pub fn invalid_input(message: impl Into<String>) -> Self {
        Self::InvalidInput(message.into())
    }

    pub fn disabled(id: impl Into<String>) -> Self {
        Self::Disabled(id.into())
    }

    pub fn empty_workflow(id: impl Into<String>) -> Self {
        Self::EmptyWorkflow(id.into())
    }

    pub fn schema_validation(message: impl Into<String>) -> Self {
        Self::SchemaValidation(message.into())
    }

    pub fn timeout(step: impl Into<String>, timeout_ms: u64) -> Self {
        Self::Timeout {
            step: step.into(),
            timeout_ms,
        }
    }

    pub fn service_unavailable(message: impl Into<String>) -> Self {
        Self::ServiceUnavailable(message.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = WorkflowError::not_found("test-workflow");
        assert_eq!(err.to_string(), "Workflow not found: test-workflow");

        let err = WorkflowError::step_execution("step1", "Connection failed");
        assert_eq!(
            err.to_string(),
            "Step execution failed in 'step1': Connection failed"
        );

        let err = WorkflowError::timeout("slow-step", 5000);
        assert_eq!(
            err.to_string(),
            "Timeout in step 'slow-step' after 5000ms"
        );
    }

    #[test]
    fn test_error_equality() {
        let err1 = WorkflowError::not_found("test");
        let err2 = WorkflowError::not_found("test");
        assert_eq!(err1, err2);

        let err3 = WorkflowError::not_found("other");
        assert_ne!(err1, err3);
    }
}
