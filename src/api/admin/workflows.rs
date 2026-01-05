//! Workflow management admin endpoints

use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::api::middleware::RequireApiKey;
use crate::api::state::AppState;
use crate::api::types::ApiError;
use crate::domain::workflow::{OnErrorAction, Workflow, WorkflowStep, WorkflowStepType};
use crate::infrastructure::services::{CreateWorkflowRequest, UpdateWorkflowRequest};

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
    RequireApiKey(api_key): RequireApiKey,
) -> Result<Json<ListWorkflowsResponse>, ApiError> {
    if !api_key.permissions().admin {
        return Err(ApiError::forbidden("Admin access required"));
    }

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
    RequireApiKey(api_key): RequireApiKey,
    Json(request): Json<CreateWorkflowApiRequest>,
) -> Result<Json<WorkflowResponse>, ApiError> {
    if !api_key.permissions().admin {
        return Err(ApiError::forbidden("Admin access required"));
    }

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
    RequireApiKey(api_key): RequireApiKey,
    Path(workflow_id): Path<String>,
) -> Result<Json<WorkflowResponse>, ApiError> {
    if !api_key.permissions().admin {
        return Err(ApiError::forbidden("Admin access required"));
    }

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
    RequireApiKey(api_key): RequireApiKey,
    Path(workflow_id): Path<String>,
    Json(request): Json<UpdateWorkflowApiRequest>,
) -> Result<Json<WorkflowResponse>, ApiError> {
    if !api_key.permissions().admin {
        return Err(ApiError::forbidden("Admin access required"));
    }

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
    RequireApiKey(api_key): RequireApiKey,
    Path(workflow_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    if !api_key.permissions().admin {
        return Err(ApiError::forbidden("Admin access required"));
    }

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

#[cfg(test)]
mod tests {
    use super::*;

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
}
