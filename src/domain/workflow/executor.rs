//! Workflow executor trait and result types

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::entity::Workflow;
use super::error::WorkflowError;

/// Result of executing a workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowResult {
    /// Whether the workflow completed successfully
    pub success: bool,

    /// Final output from the workflow
    pub output: Value,

    /// Results from each executed step
    pub step_results: Vec<StepExecutionResult>,

    /// Total execution time in milliseconds
    pub execution_time_ms: u64,

    /// Error message if workflow failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl WorkflowResult {
    /// Create a successful result
    pub fn success(output: Value, step_results: Vec<StepExecutionResult>, execution_time_ms: u64) -> Self {
        Self {
            success: true,
            output,
            step_results,
            execution_time_ms,
            error: None,
        }
    }

    /// Create a failed result
    pub fn failure(
        error: impl Into<String>,
        step_results: Vec<StepExecutionResult>,
        execution_time_ms: u64,
    ) -> Self {
        Self {
            success: false,
            output: Value::Null,
            step_results,
            execution_time_ms,
            error: Some(error.into()),
        }
    }

    /// Get the last successful step's output
    pub fn last_step_output(&self) -> Option<&Value> {
        self.step_results
            .iter()
            .rev()
            .find(|r| r.success)
            .and_then(|r| r.output.as_ref())
    }
}

/// Result of executing a single step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepExecutionResult {
    /// Step name
    pub step_name: String,

    /// Whether the step executed successfully
    pub success: bool,

    /// Step output if successful
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<Value>,

    /// Error message if failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,

    /// Execution time in milliseconds
    pub execution_time_ms: u64,

    /// Whether step was skipped
    #[serde(default)]
    pub skipped: bool,
}

impl StepExecutionResult {
    /// Create a successful step result
    pub fn success(step_name: impl Into<String>, output: Value, execution_time_ms: u64) -> Self {
        Self {
            step_name: step_name.into(),
            success: true,
            output: Some(output),
            error: None,
            execution_time_ms,
            skipped: false,
        }
    }

    /// Create a failed step result
    pub fn failure(step_name: impl Into<String>, error: impl Into<String>, execution_time_ms: u64) -> Self {
        Self {
            step_name: step_name.into(),
            success: false,
            output: None,
            error: Some(error.into()),
            execution_time_ms,
            skipped: false,
        }
    }

    /// Create a skipped step result
    pub fn skipped(step_name: impl Into<String>) -> Self {
        Self {
            step_name: step_name.into(),
            success: true,
            output: None,
            error: None,
            execution_time_ms: 0,
            skipped: true,
        }
    }
}

/// Trait for workflow execution
#[async_trait]
pub trait WorkflowExecutor: Send + Sync + std::fmt::Debug {
    /// Execute a workflow with the given input
    async fn execute(
        &self,
        workflow: &Workflow,
        input: Value,
    ) -> Result<WorkflowResult, WorkflowError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_workflow_result_success() {
        let step_results = vec![
            StepExecutionResult::success("step1", json!({"result": "ok"}), 100),
            StepExecutionResult::success("step2", json!({"final": true}), 200),
        ];

        let result = WorkflowResult::success(json!({"answer": "42"}), step_results, 300);

        assert!(result.success);
        assert_eq!(result.output, json!({"answer": "42"}));
        assert_eq!(result.step_results.len(), 2);
        assert_eq!(result.execution_time_ms, 300);
        assert!(result.error.is_none());
    }

    #[test]
    fn test_workflow_result_failure() {
        let step_results = vec![StepExecutionResult::failure("step1", "Something went wrong", 50)];

        let result = WorkflowResult::failure("Step failed", step_results, 50);

        assert!(!result.success);
        assert_eq!(result.output, Value::Null);
        assert_eq!(result.error, Some("Step failed".to_string()));
    }

    #[test]
    fn test_step_result_success() {
        let result = StepExecutionResult::success("my-step", json!({"data": 123}), 150);

        assert!(result.success);
        assert_eq!(result.step_name, "my-step");
        assert_eq!(result.output, Some(json!({"data": 123})));
        assert!(result.error.is_none());
        assert!(!result.skipped);
    }

    #[test]
    fn test_step_result_skipped() {
        let result = StepExecutionResult::skipped("skipped-step");

        assert!(result.success);
        assert!(result.skipped);
        assert!(result.output.is_none());
        assert_eq!(result.execution_time_ms, 0);
    }

    #[test]
    fn test_last_step_output() {
        let step_results = vec![
            StepExecutionResult::success("step1", json!({"first": true}), 100),
            StepExecutionResult::failure("step2", "failed", 50),
            StepExecutionResult::success("step3", json!({"last": true}), 100),
        ];

        let result = WorkflowResult::success(json!({}), step_results, 250);
        let last = result.last_step_output().unwrap();
        assert_eq!(last, &json!({"last": true}));
    }

    #[test]
    fn test_serialization() {
        let result = WorkflowResult::success(
            json!({"answer": "42"}),
            vec![StepExecutionResult::success("step1", json!({"ok": true}), 100)],
            100,
        );

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"answer\":\"42\""));

        let deserialized: WorkflowResult = serde_json::from_str(&json).unwrap();
        assert!(deserialized.success);
    }
}
