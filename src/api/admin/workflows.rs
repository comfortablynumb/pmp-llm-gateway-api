//! Workflow management admin endpoints

use std::collections::HashMap;
use std::time::Instant;

use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::debug;

use crate::api::middleware::{AdminAuth, RequireAdmin};
use crate::api::state::AppState;
use crate::api::types::ApiError;
use crate::domain::workflow::{OnErrorAction, Workflow, WorkflowStep, WorkflowStepType};
use crate::domain::{ExecutionStatus, Executor, WorkflowStepLog};
use crate::infrastructure::services::{CreateWorkflowRequest, RecordExecutionParams, UpdateWorkflowRequest};

/// Request to create a new workflow
#[derive(Debug, Clone, Deserialize)]
pub struct CreateWorkflowApiRequest {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub input_schema: Option<serde_json::Value>,
    #[serde(default)]
    pub steps: Vec<WorkflowStepApiRequest>,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

/// Request to update a workflow
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateWorkflowApiRequest {
    pub name: Option<String>,
    pub description: Option<Option<String>>,
    pub input_schema: Option<Option<serde_json::Value>>,
    pub steps: Option<Vec<WorkflowStepApiRequest>>,
    pub enabled: Option<bool>,
}

/// Workflow step in API request
#[derive(Debug, Clone, Deserialize)]
pub struct WorkflowStepApiRequest {
    pub name: String,
    #[serde(flatten)]
    pub step_type: WorkflowStepType,
    #[serde(default)]
    pub output_schema: Option<serde_json::Value>,
    #[serde(default)]
    pub on_error: OnErrorAction,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
}

impl From<WorkflowStepApiRequest> for WorkflowStep {
    fn from(req: WorkflowStepApiRequest) -> Self {
        let mut step = WorkflowStep::new(req.name, req.step_type);

        if let Some(schema) = req.output_schema {
            step = step.with_output_schema(schema);
        }

        step = step.with_on_error(req.on_error);

        if let Some(timeout) = req.timeout_ms {
            step = step.with_timeout_ms(timeout);
        }

        step
    }
}

/// Workflow response for admin API
#[derive(Debug, Clone, Serialize)]
pub struct WorkflowResponse {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub input_schema: Option<serde_json::Value>,
    pub steps: Vec<WorkflowStepResponse>,
    pub version: u32,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// Workflow step response
#[derive(Debug, Clone, Serialize)]
pub struct WorkflowStepResponse {
    pub name: String,
    #[serde(flatten)]
    pub step_type: WorkflowStepType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_schema: Option<serde_json::Value>,
    pub on_error: OnErrorAction,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
}

impl From<&WorkflowStep> for WorkflowStepResponse {
    fn from(step: &WorkflowStep) -> Self {
        Self {
            name: step.name().to_string(),
            step_type: step.step_type().clone(),
            output_schema: step.output_schema().cloned(),
            on_error: step.on_error(),
            timeout_ms: step.timeout_ms(),
        }
    }
}

impl From<&Workflow> for WorkflowResponse {
    fn from(workflow: &Workflow) -> Self {
        Self {
            id: workflow.id().as_str().to_string(),
            name: workflow.name().to_string(),
            description: workflow.description().map(String::from),
            input_schema: workflow.input_schema().cloned(),
            steps: workflow.steps().iter().map(WorkflowStepResponse::from).collect(),
            version: workflow.version(),
            enabled: workflow.is_enabled(),
            created_at: workflow.created_at().to_rfc3339(),
            updated_at: workflow.updated_at().to_rfc3339(),
        }
    }
}

/// List workflows response
#[derive(Debug, Clone, Serialize)]
pub struct ListWorkflowsResponse {
    pub workflows: Vec<WorkflowResponse>,
    pub total: usize,
}

/// GET /admin/workflows
pub async fn list_workflows(
    State(state): State<AppState>,
    RequireAdmin(_): RequireAdmin,
) -> Result<Json<ListWorkflowsResponse>, ApiError> {
    debug!("Admin listing all workflows");

    let workflows = state.workflow_service.list().await.map_err(ApiError::from)?;

    let workflow_responses: Vec<WorkflowResponse> =
        workflows.iter().map(WorkflowResponse::from).collect();
    let total = workflow_responses.len();

    Ok(Json(ListWorkflowsResponse {
        workflows: workflow_responses,
        total,
    }))
}

/// POST /admin/workflows
pub async fn create_workflow(
    State(state): State<AppState>,
    RequireAdmin(_): RequireAdmin,
    Json(request): Json<CreateWorkflowApiRequest>,
) -> Result<Json<WorkflowResponse>, ApiError> {
    debug!(workflow_id = %request.id, "Admin creating workflow");

    let create_request = CreateWorkflowRequest {
        id: request.id,
        name: request.name,
        description: request.description,
        input_schema: request.input_schema,
        steps: request.steps.into_iter().map(WorkflowStep::from).collect(),
        enabled: request.enabled,
    };

    let workflow = state
        .workflow_service
        .create(create_request)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(WorkflowResponse::from(&workflow)))
}

/// GET /admin/workflows/:workflow_id
pub async fn get_workflow(
    State(state): State<AppState>,
    RequireAdmin(_): RequireAdmin,
    Path(workflow_id): Path<String>,
) -> Result<Json<WorkflowResponse>, ApiError> {
    debug!(workflow_id = %workflow_id, "Admin getting workflow");

    let workflow = state
        .workflow_service
        .get(&workflow_id)
        .await
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError::not_found(format!("Workflow '{}' not found", workflow_id)))?;

    Ok(Json(WorkflowResponse::from(&workflow)))
}

/// PUT /admin/workflows/:workflow_id
pub async fn update_workflow(
    State(state): State<AppState>,
    RequireAdmin(_): RequireAdmin,
    Path(workflow_id): Path<String>,
    Json(request): Json<UpdateWorkflowApiRequest>,
) -> Result<Json<WorkflowResponse>, ApiError> {
    debug!(workflow_id = %workflow_id, "Admin updating workflow");

    let update_request = UpdateWorkflowRequest {
        name: request.name,
        description: request.description,
        input_schema: request.input_schema,
        steps: request.steps.map(|s| s.into_iter().map(WorkflowStep::from).collect()),
        enabled: request.enabled,
    };

    let workflow = state
        .workflow_service
        .update(&workflow_id, update_request)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(WorkflowResponse::from(&workflow)))
}

/// DELETE /admin/workflows/:workflow_id
pub async fn delete_workflow(
    State(state): State<AppState>,
    RequireAdmin(_): RequireAdmin,
    Path(workflow_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    debug!(workflow_id = %workflow_id, "Admin deleting workflow");

    state
        .workflow_service
        .delete(&workflow_id)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(serde_json::json!({
        "deleted": true,
        "id": workflow_id
    })))
}

/// Request to test a workflow with mocked step outputs
#[derive(Debug, Clone, Deserialize)]
pub struct TestWorkflowRequest {
    /// Input data for the workflow
    #[serde(default)]
    pub input: Value,

    /// Mock outputs for each step (step_name -> mock output)
    #[serde(default)]
    pub step_mocks: HashMap<String, Value>,
}

/// Response from testing a workflow
#[derive(Debug, Clone, Serialize)]
pub struct TestWorkflowResponse {
    pub success: bool,
    pub workflow_id: String,
    pub input: Value,
    pub step_results: Vec<MockStepResult>,
    pub final_output: Option<Value>,
    pub execution_time_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Result of a mocked step execution
#[derive(Debug, Clone, Serialize)]
pub struct MockStepResult {
    pub step_name: String,
    pub step_type: String,
    pub mocked: bool,
    pub output: Option<Value>,
    pub skipped: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// POST /admin/workflows/:workflow_id/test
/// Test a workflow execution with mocked step outputs
pub async fn test_workflow(
    State(state): State<AppState>,
    RequireAdmin(_): RequireAdmin,
    Path(workflow_id): Path<String>,
    Json(request): Json<TestWorkflowRequest>,
) -> Result<Json<TestWorkflowResponse>, ApiError> {
    debug!(workflow_id = %workflow_id, "Admin testing workflow with mocks");

    let start = Instant::now();

    let workflow = state
        .workflow_service
        .get(&workflow_id)
        .await
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError::not_found(format!("Workflow '{}' not found", workflow_id)))?;

    if !workflow.is_enabled() {
        return Ok(Json(TestWorkflowResponse {
            success: false,
            workflow_id,
            input: request.input,
            step_results: vec![],
            final_output: None,
            execution_time_ms: start.elapsed().as_millis() as u64,
            error: Some("Workflow is disabled".to_string()),
        }));
    }

    let mut step_results = Vec::new();
    let mut context_values: HashMap<String, Value> = HashMap::new();
    context_values.insert("request".to_string(), request.input.clone());

    let mut last_output: Option<Value> = None;

    for step in workflow.steps() {
        let step_name = step.name().to_string();
        let step_type = get_step_type_name(step.step_type());

        if let Some(mock_output) = request.step_mocks.get(&step_name) {
            context_values.insert(format!("step:{}", step_name), mock_output.clone());
            last_output = Some(mock_output.clone());

            step_results.push(MockStepResult {
                step_name,
                step_type,
                mocked: true,
                output: Some(mock_output.clone()),
                skipped: false,
                reason: None,
            });
        } else {
            step_results.push(MockStepResult {
                step_name,
                step_type,
                mocked: false,
                output: None,
                skipped: true,
                reason: Some("No mock provided - step would execute in real run".to_string()),
            });
        }
    }

    Ok(Json(TestWorkflowResponse {
        success: true,
        workflow_id,
        input: request.input,
        step_results,
        final_output: last_output,
        execution_time_ms: start.elapsed().as_millis() as u64,
        error: None,
    }))
}

fn get_step_type_name(step_type: &WorkflowStepType) -> String {
    match step_type {
        WorkflowStepType::ChatCompletion(_) => "chat_completion".to_string(),
        WorkflowStepType::KnowledgeBaseSearch(_) => "knowledge_base_search".to_string(),
        WorkflowStepType::CragScoring(_) => "crag_scoring".to_string(),
        WorkflowStepType::Conditional(_) => "conditional".to_string(),
        WorkflowStepType::HttpRequest(_) => "http_request".to_string(),
    }
}

/// Request to clone a workflow
#[derive(Debug, Clone, Deserialize)]
pub struct CloneWorkflowRequest {
    /// New ID for the cloned workflow
    pub new_id: String,
    /// New name for the cloned workflow (optional, defaults to "Copy of {original_name}")
    #[serde(default)]
    pub new_name: Option<String>,
}

/// POST /admin/workflows/:workflow_id/clone
/// Clone a workflow with a new ID
pub async fn clone_workflow(
    State(state): State<AppState>,
    RequireAdmin(_): RequireAdmin,
    Path(workflow_id): Path<String>,
    Json(request): Json<CloneWorkflowRequest>,
) -> Result<Json<WorkflowResponse>, ApiError> {
    debug!(workflow_id = %workflow_id, new_id = %request.new_id, "Admin cloning workflow");

    // Get the original workflow
    let original = state
        .workflow_service
        .get(&workflow_id)
        .await
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError::not_found(format!("Workflow '{}' not found", workflow_id)))?;

    // Create the clone request
    let new_name = request
        .new_name
        .unwrap_or_else(|| format!("Copy of {}", original.name()));

    let create_request = CreateWorkflowRequest {
        id: request.new_id,
        name: new_name,
        description: original.description().map(String::from),
        input_schema: original.input_schema().cloned(),
        steps: original.steps().to_vec(),
        enabled: true, // Cloned workflows start enabled for immediate testing
    };

    let cloned = state
        .workflow_service
        .create(create_request)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(WorkflowResponse::from(&cloned)))
}

/// Request to execute a workflow
#[derive(Debug, Clone, Deserialize)]
pub struct ExecuteWorkflowRequest {
    /// Input data for the workflow (must match input_schema if defined)
    #[serde(default)]
    pub input: Value,
}

/// Response from workflow execution
#[derive(Debug, Clone, Serialize)]
pub struct ExecuteWorkflowResponse {
    pub workflow_id: String,
    pub success: bool,
    pub output: Value,
    pub step_results: Vec<StepResultResponse>,
    pub execution_time_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Step result in execution response
#[derive(Debug, Clone, Serialize)]
pub struct StepResultResponse {
    pub step_name: String,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub execution_time_ms: u64,
}

/// POST /admin/workflows/:workflow_id/execute
/// Execute a workflow with the given input
pub async fn execute_workflow(
    State(state): State<AppState>,
    RequireAdmin(admin): RequireAdmin,
    Path(workflow_id): Path<String>,
    Json(request): Json<ExecuteWorkflowRequest>,
) -> Result<Json<ExecuteWorkflowResponse>, ApiError> {
    debug!(workflow_id = %workflow_id, "Admin executing workflow");

    // Create executor from admin auth for logging
    let executor = match &admin {
        AdminAuth::ApiKey(key) => Executor::from_api_key(key.id().as_str()),
        AdminAuth::User(user) => Executor::from_user(user.id().as_str()),
    };

    // Clone input for logging before moving it to execute
    let input_for_log = request.input.clone();

    // Execute the workflow
    let result = state
        .workflow_service
        .execute(&workflow_id, request.input)
        .await
        .map_err(ApiError::from)?;

    // Convert step results to WorkflowStepLog for execution logging
    let workflow_step_logs: Vec<WorkflowStepLog> = result
        .step_results
        .iter()
        .map(|sr| {
            let status = if sr.success {
                ExecutionStatus::Success
            } else {
                ExecutionStatus::Failed
            };

            let mut step_log = WorkflowStepLog::new(&sr.step_name, "workflow_step")
                .with_execution_time(sr.execution_time_ms)
                .with_status(status);

            if let Some(output) = &sr.output {
                step_log = step_log.with_output(output.clone());
            }

            if let Some(error) = &sr.error {
                step_log = step_log.with_error(error.clone());
            }

            step_log
        })
        .collect();

    // Record execution log with input, output, and workflow steps
    let log_params = if result.success {
        RecordExecutionParams::workflow_success(
            &workflow_id,
            result.execution_time_ms,
            executor,
        )
        .with_input(input_for_log)
        .with_output(result.output.clone())
        .with_workflow_steps(workflow_step_logs)
    } else {
        RecordExecutionParams::workflow_failed(
            &workflow_id,
            result.error.clone().unwrap_or_default(),
            result.execution_time_ms,
            executor,
        )
        .with_input(input_for_log)
        .with_workflow_steps(workflow_step_logs)
    };

    if let Err(e) = state.execution_log_service.record(log_params).await {
        debug!(error = %e, "Failed to record execution log");
    }

    // Convert step results for API response
    let step_results: Vec<StepResultResponse> = result
        .step_results
        .into_iter()
        .map(|sr| StepResultResponse {
            step_name: sr.step_name,
            success: sr.success,
            output: sr.output,
            error: sr.error,
            execution_time_ms: sr.execution_time_ms,
        })
        .collect();

    Ok(Json(ExecuteWorkflowResponse {
        workflow_id,
        success: result.success,
        output: result.output,
        step_results,
        execution_time_ms: result.execution_time_ms,
        error: result.error,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::workflow::ChatCompletionStep;

    #[test]
    fn test_create_workflow_request_deserialization() {
        let json = r#"{
            "id": "my-workflow",
            "name": "My Workflow",
            "description": "A test workflow",
            "steps": [
                {
                    "name": "chat",
                    "type": "chat_completion",
                    "model_id": "gpt-4",
                    "prompt_id": "system-prompt",
                    "user_message": "Hello"
                }
            ]
        }"#;

        let request: CreateWorkflowApiRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.id, "my-workflow");
        assert_eq!(request.name, "My Workflow");
        assert_eq!(request.steps.len(), 1);
        assert!(request.enabled);
    }

    #[test]
    fn test_create_workflow_request_minimal() {
        let json = r#"{
            "id": "minimal-workflow",
            "name": "Minimal",
            "steps": []
        }"#;

        let request: CreateWorkflowApiRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.id, "minimal-workflow");
        assert!(request.description.is_none());
        assert!(request.input_schema.is_none());
        assert!(request.steps.is_empty());
        assert!(request.enabled);
    }

    #[test]
    fn test_create_workflow_request_with_input_schema() {
        let json = r#"{
            "id": "schema-workflow",
            "name": "With Schema",
            "input_schema": {
                "type": "object",
                "properties": {
                    "query": {"type": "string"}
                },
                "required": ["query"]
            },
            "steps": []
        }"#;

        let request: CreateWorkflowApiRequest = serde_json::from_str(json).unwrap();
        assert!(request.input_schema.is_some());
        let schema = request.input_schema.unwrap();
        assert_eq!(schema["type"], "object");
    }

    #[test]
    fn test_create_workflow_disabled() {
        let json = r#"{
            "id": "disabled-workflow",
            "name": "Disabled",
            "steps": [],
            "enabled": false
        }"#;

        let request: CreateWorkflowApiRequest = serde_json::from_str(json).unwrap();
        assert!(!request.enabled);
    }

    #[test]
    fn test_update_workflow_request_deserialization() {
        let json = r#"{
            "name": "Updated Name",
            "enabled": false
        }"#;

        let request: UpdateWorkflowApiRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.name, Some("Updated Name".to_string()));
        assert_eq!(request.enabled, Some(false));
        assert!(request.steps.is_none());
    }

    #[test]
    fn test_update_workflow_request_empty() {
        let json = r#"{}"#;

        let request: UpdateWorkflowApiRequest = serde_json::from_str(json).unwrap();
        assert!(request.name.is_none());
        assert!(request.description.is_none());
        assert!(request.input_schema.is_none());
        assert!(request.steps.is_none());
        assert!(request.enabled.is_none());
    }

    #[test]
    fn test_update_workflow_request_with_steps() {
        let json = r#"{
            "steps": [
                {
                    "name": "new-step",
                    "type": "chat_completion",
                    "model_id": "gpt-4",
                    "prompt_id": "prompt-1",
                    "user_message": "Test"
                }
            ]
        }"#;

        let request: UpdateWorkflowApiRequest = serde_json::from_str(json).unwrap();
        assert!(request.steps.is_some());
        assert_eq!(request.steps.unwrap().len(), 1);
    }

    #[test]
    fn test_workflow_step_api_request_basic() {
        let json = r#"{
            "name": "chat-step",
            "type": "chat_completion",
            "model_id": "gpt-4",
            "prompt_id": "system",
            "user_message": "Hello"
        }"#;

        let step: WorkflowStepApiRequest = serde_json::from_str(json).unwrap();
        assert_eq!(step.name, "chat-step");
        assert!(step.output_schema.is_none());
        assert!(step.timeout_ms.is_none());
    }

    #[test]
    fn test_workflow_step_api_request_with_options() {
        let json = r#"{
            "name": "step-with-opts",
            "type": "chat_completion",
            "model_id": "gpt-4",
            "prompt_id": "prompt",
            "user_message": "Test",
            "output_schema": {"type": "string"},
            "on_error": "skip_step",
            "timeout_ms": 30000
        }"#;

        let step: WorkflowStepApiRequest = serde_json::from_str(json).unwrap();
        assert!(step.output_schema.is_some());
        assert!(matches!(step.on_error, OnErrorAction::SkipStep));
        assert_eq!(step.timeout_ms, Some(30000));
    }

    #[test]
    fn test_workflow_step_from_api_request() {
        let api_req = WorkflowStepApiRequest {
            name: "test-step".to_string(),
            step_type: WorkflowStepType::ChatCompletion(ChatCompletionStep {
                model_id: "gpt-4".to_string(),
                prompt_id: "prompt".to_string(),
                user_message: "Hello".to_string(),
                temperature: None,
                max_tokens: None,
                top_p: None,
            }),
            output_schema: Some(serde_json::json!({"type": "string"})),
            on_error: OnErrorAction::SkipStep,
            timeout_ms: Some(5000),
        };

        let step: WorkflowStep = api_req.into();
        assert_eq!(step.name(), "test-step");
        assert!(step.output_schema().is_some());
        assert!(matches!(step.on_error(), OnErrorAction::SkipStep));
        assert_eq!(step.timeout_ms(), Some(5000));
    }

    #[test]
    fn test_default_true() {
        assert!(default_true());
    }

    #[test]
    fn test_get_step_type_name() {
        assert_eq!(
            get_step_type_name(&WorkflowStepType::ChatCompletion(ChatCompletionStep {
                model_id: "m".to_string(),
                prompt_id: "p".to_string(),
                user_message: "u".to_string(),
                temperature: None,
                max_tokens: None,
                top_p: None,
            })),
            "chat_completion"
        );
    }

    #[test]
    fn test_execute_workflow_request_deserialization() {
        let json = r#"{
            "input": {"query": "What is Rust?", "max_results": 5}
        }"#;

        let request: ExecuteWorkflowRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.input["query"], "What is Rust?");
        assert_eq!(request.input["max_results"], 5);
    }

    #[test]
    fn test_execute_workflow_request_empty_input() {
        let json = r#"{}"#;

        let request: ExecuteWorkflowRequest = serde_json::from_str(json).unwrap();
        assert!(request.input.is_null());
    }

    #[test]
    fn test_execute_workflow_response_serialization() {
        let response = ExecuteWorkflowResponse {
            workflow_id: "test-workflow".to_string(),
            success: true,
            output: serde_json::json!({"result": "success"}),
            step_results: vec![
                StepResultResponse {
                    step_name: "step1".to_string(),
                    success: true,
                    output: Some(serde_json::json!({"data": "test"})),
                    error: None,
                    execution_time_ms: 50,
                },
            ],
            execution_time_ms: 100,
            error: None,
        };

        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["workflow_id"], "test-workflow");
        assert_eq!(json["success"], true);
        assert_eq!(json["output"]["result"], "success");
        assert_eq!(json["step_results"].as_array().unwrap().len(), 1);
        assert_eq!(json["step_results"][0]["step_name"], "step1");
        assert_eq!(json["execution_time_ms"], 100);
        assert!(json.get("error").is_none());
    }

    #[test]
    fn test_execute_workflow_response_with_error() {
        let response = ExecuteWorkflowResponse {
            workflow_id: "failed-workflow".to_string(),
            success: false,
            output: serde_json::json!(null),
            step_results: vec![
                StepResultResponse {
                    step_name: "failed-step".to_string(),
                    success: false,
                    output: None,
                    error: Some("Step failed".to_string()),
                    execution_time_ms: 25,
                },
            ],
            execution_time_ms: 50,
            error: Some("Workflow failed at step 1".to_string()),
        };

        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["success"], false);
        assert_eq!(json["error"], "Workflow failed at step 1");
        assert_eq!(json["step_results"][0]["error"], "Step failed");
    }

    #[test]
    fn test_list_workflows_response_serialization() {
        let response = ListWorkflowsResponse {
            workflows: vec![],
            total: 0,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"workflows\":[]"));
        assert!(json.contains("\"total\":0"));
    }

    #[test]
    fn test_test_workflow_request_deserialization() {
        let json = r#"{
            "input": {"query": "test"},
            "step_mocks": {
                "step1": {"result": "mocked"}
            }
        }"#;

        let request: TestWorkflowRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.input["query"], "test");
        assert!(request.step_mocks.contains_key("step1"));
    }

    #[test]
    fn test_test_workflow_request_empty() {
        let json = r#"{}"#;

        let request: TestWorkflowRequest = serde_json::from_str(json).unwrap();
        assert!(request.input.is_null());
        assert!(request.step_mocks.is_empty());
    }

    #[test]
    fn test_test_workflow_response_serialization() {
        let response = TestWorkflowResponse {
            success: true,
            workflow_id: "test-wf".to_string(),
            input: serde_json::json!({"q": "test"}),
            step_results: vec![MockStepResult {
                step_name: "step1".to_string(),
                step_type: "chat_completion".to_string(),
                mocked: true,
                output: Some(serde_json::json!({"result": "mock"})),
                skipped: false,
                reason: None,
            }],
            final_output: Some(serde_json::json!({"result": "mock"})),
            execution_time_ms: 10,
            error: None,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"mocked\":true"));
    }

    #[test]
    fn test_mock_step_result_skipped() {
        let result = MockStepResult {
            step_name: "skipped-step".to_string(),
            step_type: "http_request".to_string(),
            mocked: false,
            output: None,
            skipped: true,
            reason: Some("No mock provided".to_string()),
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"skipped\":true"));
        assert!(json.contains("\"reason\":\"No mock provided\""));
    }

    #[test]
    fn test_clone_workflow_request_deserialization() {
        let json = r#"{
            "new_id": "cloned-workflow"
        }"#;

        let request: CloneWorkflowRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.new_id, "cloned-workflow");
        assert!(request.new_name.is_none());
    }

    #[test]
    fn test_clone_workflow_request_with_name() {
        let json = r#"{
            "new_id": "clone-1",
            "new_name": "My Cloned Workflow"
        }"#;

        let request: CloneWorkflowRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.new_id, "clone-1");
        assert_eq!(request.new_name, Some("My Cloned Workflow".to_string()));
    }

    #[test]
    fn test_step_result_response_serialization() {
        let result = StepResultResponse {
            step_name: "step1".to_string(),
            success: true,
            output: Some(serde_json::json!({"key": "value"})),
            error: None,
            execution_time_ms: 100,
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"step_name\":\"step1\""));
        assert!(json.contains("\"success\":true"));
        assert!(!json.contains("\"error\""));
    }
}
