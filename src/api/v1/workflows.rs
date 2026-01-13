//! Workflow execution endpoint

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{debug, error, info, warn};

use crate::api::middleware::RequireApiKey;
use crate::api::state::AppState;
use crate::api::types::{ApiError, AsyncOperationCreated, AsyncQueryParams, Json};
use crate::domain::workflow::StepExecutionResult;
use crate::domain::OperationType;

/// Request to execute a workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowExecuteRequest {
    /// Input data for the workflow
    #[serde(default)]
    pub input: serde_json::Value,
}

/// Response from workflow execution
#[derive(Debug, Clone, Serialize)]
pub struct WorkflowExecuteResponse {
    /// Whether the workflow completed successfully
    pub success: bool,

    /// Final output from the workflow
    pub output: serde_json::Value,

    /// Total execution time in milliseconds
    pub execution_time_ms: u64,

    /// Summary of each step's execution
    pub steps: Vec<StepExecutionSummary>,

    /// Error message if workflow failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Summary of a step's execution
#[derive(Debug, Clone, Serialize)]
pub struct StepExecutionSummary {
    /// Step name
    pub name: String,

    /// Whether the step succeeded
    pub success: bool,

    /// Execution time in milliseconds
    pub execution_time_ms: u64,

    /// Whether step was skipped
    #[serde(skip_serializing_if = "is_false")]
    pub skipped: bool,

    /// Error message if step failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

fn is_false(b: &bool) -> bool {
    !*b
}

impl From<&StepExecutionResult> for StepExecutionSummary {
    fn from(result: &StepExecutionResult) -> Self {
        Self {
            name: result.step_name.clone(),
            success: result.success,
            execution_time_ms: result.execution_time_ms,
            skipped: result.skipped,
            error: result.error.clone(),
        }
    }
}

/// POST /v1/workflows/:workflow_id/execute
pub async fn execute_workflow(
    State(state): State<AppState>,
    RequireApiKey(api_key): RequireApiKey,
    Path(workflow_id): Path<String>,
    Query(async_params): Query<AsyncQueryParams>,
    Json(request): Json<WorkflowExecuteRequest>,
) -> Result<Response, ApiError> {
    debug!(
        workflow_id = %workflow_id,
        api_key_id = %api_key.id().as_str(),
        is_async = async_params.is_async,
        "Executing workflow"
    );

    // Handle async mode
    if async_params.is_async {
        return handle_async_workflow_execution(state, workflow_id, request).await;
    }

    let result = state
        .workflow_service
        .execute(&workflow_id, request.input)
        .await
        .map_err(ApiError::from)?;

    let response = WorkflowExecuteResponse {
        success: result.success,
        output: result.output,
        execution_time_ms: result.execution_time_ms,
        steps: result.step_results.iter().map(StepExecutionSummary::from).collect(),
        error: result.error,
    };

    Ok(Json(response).into_response())
}

/// Handle async workflow execution
async fn handle_async_workflow_execution(
    state: AppState,
    workflow_id: String,
    request: WorkflowExecuteRequest,
) -> Result<Response, ApiError> {
    // Create pending operation
    let operation = state
        .operation_service
        .create_pending(
            OperationType::WorkflowExecution,
            serde_json::to_value(&request).unwrap_or(json!({})),
            json!({ "workflow_id": &workflow_id }),
        )
        .await
        .map_err(ApiError::from)?;

    let operation_id = operation.id().to_string();
    info!(
        operation_id = %operation_id,
        workflow_id = %workflow_id,
        "Created async workflow execution operation"
    );

    // Spawn background task - function returns a boxed future to avoid stack overflow
    let op_id = operation_id.clone();
    let input = request.input;
    tokio::spawn(execute_async_workflow(state, op_id, workflow_id, input));

    // Return 202 Accepted
    Ok((
        StatusCode::ACCEPTED,
        Json(AsyncOperationCreated::pending(&operation_id)),
    )
        .into_response())
}

/// Execute workflow in background and update operation status
///
/// Returns a boxed future to avoid stack overflow from large future sizes
/// caused by trait object indirection in AppState.
fn execute_async_workflow(
    state: AppState,
    operation_id: String,
    workflow_id: String,
    input: serde_json::Value,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> {
    Box::pin(async move {
    // Mark as running
    if let Err(e) = state.operation_service.mark_running(&operation_id).await {
        warn!(
            operation_id = %operation_id,
            error = %e,
            "Failed to mark operation as running"
        );
        return;
    }

    // Execute workflow
    match state.workflow_service.execute(&workflow_id, input).await {
        Ok(result) => {
            let response = WorkflowExecuteResponse {
                success: result.success,
                output: result.output,
                execution_time_ms: result.execution_time_ms,
                steps: result.step_results.iter().map(StepExecutionSummary::from).collect(),
                error: result.error.clone(),
            };

            // If workflow reports failure, mark operation as failed
            if !result.success {
                let error_msg = result
                    .error
                    .unwrap_or_else(|| "Workflow execution failed".to_string());

                if let Err(mark_err) = state
                    .operation_service
                    .mark_failed(&operation_id, error_msg.clone())
                    .await
                {
                    error!(
                        operation_id = %operation_id,
                        error = %mark_err,
                        "Failed to mark operation as failed"
                    );
                } else {
                    warn!(
                        operation_id = %operation_id,
                        error = %error_msg,
                        "Async workflow execution failed"
                    );
                }
            } else {
                let result_value = serde_json::to_value(&response).unwrap_or(json!({}));

                if let Err(e) = state
                    .operation_service
                    .mark_completed(&operation_id, result_value)
                    .await
                {
                    error!(
                        operation_id = %operation_id,
                        error = %e,
                        "Failed to mark operation as completed"
                    );
                } else {
                    info!(operation_id = %operation_id, "Async workflow execution succeeded");
                }
            }
        }
        Err(e) => {
            let error_msg = e.to_string();

            if let Err(mark_err) = state
                .operation_service
                .mark_failed(&operation_id, error_msg.clone())
                .await
            {
                error!(
                    operation_id = %operation_id,
                    error = %mark_err,
                    "Failed to mark operation as failed"
                );
            } else {
                warn!(
                    operation_id = %operation_id,
                    error = %error_msg,
                    "Async workflow execution failed"
                );
            }
        }
    }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execute_request_deserialization() {
        let json = r#"{
            "input": {"question": "What is the capital of France?"}
        }"#;

        let request: WorkflowExecuteRequest = serde_json::from_str(json).unwrap();
        assert_eq!(
            request.input.get("question").unwrap().as_str().unwrap(),
            "What is the capital of France?"
        );
    }

    #[test]
    fn test_execute_request_default_input() {
        let json = r#"{}"#;

        let request: WorkflowExecuteRequest = serde_json::from_str(json).unwrap();
        assert!(request.input.is_null());
    }

    #[test]
    fn test_execute_request_with_array_input() {
        let json = r#"{"input": [1, 2, 3]}"#;

        let request: WorkflowExecuteRequest = serde_json::from_str(json).unwrap();
        assert!(request.input.is_array());
    }

    #[test]
    fn test_execute_request_serialization() {
        let request = WorkflowExecuteRequest {
            input: json!({"key": "value"}),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"input\":"));
        assert!(json.contains("\"key\":\"value\""));
    }

    #[test]
    fn test_step_summary_serialization() {
        let summary = StepExecutionSummary {
            name: "step1".to_string(),
            success: true,
            execution_time_ms: 100,
            skipped: false,
            error: None,
        };

        let json = serde_json::to_string(&summary).unwrap();
        assert!(!json.contains("skipped"));
        assert!(!json.contains("error"));
        assert!(json.contains("\"name\":\"step1\""));
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"execution_time_ms\":100"));
    }

    #[test]
    fn test_step_summary_with_skipped() {
        let summary = StepExecutionSummary {
            name: "conditional_step".to_string(),
            success: true,
            execution_time_ms: 0,
            skipped: true,
            error: None,
        };

        let json = serde_json::to_string(&summary).unwrap();
        assert!(json.contains("\"skipped\":true"));
    }

    #[test]
    fn test_step_summary_with_error() {
        let summary = StepExecutionSummary {
            name: "failed_step".to_string(),
            success: false,
            execution_time_ms: 50,
            skipped: false,
            error: Some("Connection timeout".to_string()),
        };

        let json = serde_json::to_string(&summary).unwrap();
        assert!(json.contains("\"success\":false"));
        assert!(json.contains("\"error\":\"Connection timeout\""));
    }

    #[test]
    fn test_workflow_execute_response_success() {
        let response = WorkflowExecuteResponse {
            success: true,
            output: json!({"result": "success"}),
            execution_time_ms: 250,
            steps: vec![
                StepExecutionSummary {
                    name: "step1".to_string(),
                    success: true,
                    execution_time_ms: 100,
                    skipped: false,
                    error: None,
                },
                StepExecutionSummary {
                    name: "step2".to_string(),
                    success: true,
                    execution_time_ms: 150,
                    skipped: false,
                    error: None,
                },
            ],
            error: None,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"execution_time_ms\":250"));
        assert!(json.contains("\"steps\":["));
        assert!(!json.contains("\"error\":"));
    }

    #[test]
    fn test_workflow_execute_response_failure() {
        let response = WorkflowExecuteResponse {
            success: false,
            output: json!(null),
            execution_time_ms: 100,
            steps: vec![
                StepExecutionSummary {
                    name: "step1".to_string(),
                    success: false,
                    execution_time_ms: 100,
                    skipped: false,
                    error: Some("LLM error".to_string()),
                },
            ],
            error: Some("Workflow failed at step1".to_string()),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"success\":false"));
        assert!(json.contains("\"error\":\"Workflow failed at step1\""));
    }

    #[test]
    fn test_is_false_function() {
        assert!(is_false(&false));
        assert!(!is_false(&true));
    }

    #[test]
    fn test_step_execution_summary_from() {
        let result = StepExecutionResult {
            step_name: "test_step".to_string(),
            step_type: "chat_completion".to_string(),
            success: true,
            input: Some(json!({"user_message": "Hello"})),
            output: Some(json!({"data": "test"})),
            execution_time_ms: 75,
            skipped: false,
            error: None,
            token_usage: None,
            cost_micros: None,
        };

        let summary = StepExecutionSummary::from(&result);
        assert_eq!(summary.name, "test_step");
        assert!(summary.success);
        assert_eq!(summary.execution_time_ms, 75);
        assert!(!summary.skipped);
        assert!(summary.error.is_none());
    }

    #[test]
    fn test_step_execution_summary_from_skipped() {
        let result = StepExecutionResult {
            step_name: "skipped_step".to_string(),
            step_type: "chat_completion".to_string(),
            success: true,
            input: None,
            output: None,
            execution_time_ms: 0,
            skipped: true,
            error: None,
            token_usage: None,
            cost_micros: None,
        };

        let summary = StepExecutionSummary::from(&result);
        assert!(summary.skipped);
    }

    #[test]
    fn test_step_execution_summary_from_failed() {
        let result = StepExecutionResult {
            step_name: "failed_step".to_string(),
            step_type: "chat_completion".to_string(),
            success: false,
            input: None,
            output: None,
            execution_time_ms: 100,
            skipped: false,
            error: Some("Error occurred".to_string()),
            token_usage: None,
            cost_micros: None,
        };

        let summary = StepExecutionSummary::from(&result);
        assert!(!summary.success);
        assert_eq!(summary.error, Some("Error occurred".to_string()));
    }
}
