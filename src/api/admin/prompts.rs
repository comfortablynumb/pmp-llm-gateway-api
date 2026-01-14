//! Prompt management admin endpoints

use axum::extract::{Path, State};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::debug;

use crate::api::middleware::RequireAdmin;
use crate::api::state::AppState;
use crate::api::types::{ApiError, Json};
use crate::domain::prompt::{Prompt, PromptOutputSchema, PromptVersion};
use crate::infrastructure::services::{CreatePromptRequest, UpdatePromptRequest};

/// Output schema for API requests/responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputSchemaApi {
    pub name: String,
    #[serde(default)]
    pub strict: bool,
    pub schema: serde_json::Value,
}

impl From<&PromptOutputSchema> for OutputSchemaApi {
    fn from(s: &PromptOutputSchema) -> Self {
        Self {
            name: s.name.clone(),
            strict: s.strict,
            schema: s.schema.clone(),
        }
    }
}

impl From<OutputSchemaApi> for PromptOutputSchema {
    fn from(s: OutputSchemaApi) -> Self {
        PromptOutputSchema {
            name: s.name,
            strict: s.strict,
            schema: s.schema,
        }
    }
}

/// Request to create a new prompt
#[derive(Debug, Clone, Deserialize)]
pub struct CreatePromptApiRequest {
    pub id: String,
    pub name: String,
    pub content: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub output_schema: Option<OutputSchemaApi>,
}

/// Request to update a prompt
#[derive(Debug, Clone, Deserialize)]
pub struct UpdatePromptApiRequest {
    pub name: Option<String>,
    pub content: Option<String>,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
    pub output_schema: Option<OutputSchemaApi>,
}

/// Request to render a prompt
#[derive(Debug, Clone, Deserialize)]
pub struct RenderPromptApiRequest {
    #[serde(default)]
    pub variables: HashMap<String, String>,
}

/// Prompt response for admin API
#[derive(Debug, Clone, Serialize)]
pub struct PromptResponse {
    pub id: String,
    pub name: String,
    pub content: String,
    pub description: Option<String>,
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_schema: Option<OutputSchemaApi>,
    pub version: u32,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl From<&Prompt> for PromptResponse {
    fn from(prompt: &Prompt) -> Self {
        Self {
            id: prompt.id().as_str().to_string(),
            name: prompt.name().to_string(),
            content: prompt.content().to_string(),
            description: prompt.description().map(String::from),
            tags: prompt.tags().to_vec(),
            output_schema: prompt.output_schema().map(OutputSchemaApi::from),
            version: prompt.version(),
            enabled: prompt.is_enabled(),
            created_at: prompt.created_at().to_rfc3339(),
            updated_at: prompt.updated_at().to_rfc3339(),
        }
    }
}

/// List prompts response
#[derive(Debug, Clone, Serialize)]
pub struct ListPromptsResponse {
    pub prompts: Vec<PromptResponse>,
    pub total: usize,
}

/// Render prompt response
#[derive(Debug, Clone, Serialize)]
pub struct RenderPromptResponse {
    pub rendered: String,
}

/// Response for a single prompt version
#[derive(Debug, Clone, Serialize)]
pub struct PromptVersionResponse {
    pub version: u32,
    pub content: String,
    pub created_at: String,
    pub message: Option<String>,
}

impl From<&PromptVersion> for PromptVersionResponse {
    fn from(v: &PromptVersion) -> Self {
        Self {
            version: v.version(),
            content: v.content().to_string(),
            created_at: v.created_at().to_rfc3339(),
            message: v.message().map(String::from),
        }
    }
}

/// List versions response
#[derive(Debug, Clone, Serialize)]
pub struct ListVersionsResponse {
    pub current_version: u32,
    pub versions: Vec<PromptVersionResponse>,
    pub total: usize,
}

/// GET /admin/prompts
pub async fn list_prompts(
    State(state): State<AppState>,
    RequireAdmin(_): RequireAdmin,
) -> Result<Json<ListPromptsResponse>, ApiError> {
    debug!("Admin listing all prompts");

    let prompts = state.prompt_service.list().await.map_err(ApiError::from)?;

    let prompt_responses: Vec<PromptResponse> =
        prompts.iter().map(PromptResponse::from).collect();
    let total = prompt_responses.len();

    Ok(Json(ListPromptsResponse {
        prompts: prompt_responses,
        total,
    }))
}

/// POST /admin/prompts
pub async fn create_prompt(
    State(state): State<AppState>,
    RequireAdmin(_): RequireAdmin,
    Json(request): Json<CreatePromptApiRequest>,
) -> Result<Json<PromptResponse>, ApiError> {
    debug!(prompt_id = %request.id, "Admin creating prompt");

    let create_request = CreatePromptRequest {
        id: request.id,
        name: request.name,
        description: request.description,
        content: request.content,
        tags: request.tags,
        enabled: true,
        max_history: None,
        output_schema: request.output_schema.map(Into::into),
    };

    let prompt = state
        .prompt_service
        .create(create_request)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(PromptResponse::from(&prompt)))
}

/// GET /admin/prompts/:prompt_id
pub async fn get_prompt(
    State(state): State<AppState>,
    RequireAdmin(_): RequireAdmin,
    Path(prompt_id): Path<String>,
) -> Result<Json<PromptResponse>, ApiError> {
    debug!(prompt_id = %prompt_id, "Admin getting prompt");

    let prompt = state
        .prompt_service
        .get(&prompt_id)
        .await
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError::not_found(format!("Prompt '{}' not found", prompt_id)))?;

    Ok(Json(PromptResponse::from(&prompt)))
}

/// PUT /admin/prompts/:prompt_id
pub async fn update_prompt(
    State(state): State<AppState>,
    RequireAdmin(_): RequireAdmin,
    Path(prompt_id): Path<String>,
    Json(request): Json<UpdatePromptApiRequest>,
) -> Result<Json<PromptResponse>, ApiError> {
    debug!(prompt_id = %prompt_id, "Admin updating prompt");

    let update_request = UpdatePromptRequest {
        name: request.name,
        description: request.description,
        content: request.content,
        content_message: None,
        tags: request.tags,
        enabled: None,
        output_schema: request.output_schema.map(Into::into),
    };

    let prompt = state
        .prompt_service
        .update(&prompt_id, update_request)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(PromptResponse::from(&prompt)))
}

/// DELETE /admin/prompts/:prompt_id
pub async fn delete_prompt(
    State(state): State<AppState>,
    RequireAdmin(_): RequireAdmin,
    Path(prompt_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    debug!(prompt_id = %prompt_id, "Admin deleting prompt");

    state
        .prompt_service
        .delete(&prompt_id)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(serde_json::json!({
        "deleted": true,
        "id": prompt_id
    })))
}

/// POST /admin/prompts/:prompt_id/render
pub async fn render_prompt(
    State(state): State<AppState>,
    RequireAdmin(_): RequireAdmin,
    Path(prompt_id): Path<String>,
    Json(request): Json<RenderPromptApiRequest>,
) -> Result<Json<RenderPromptResponse>, ApiError> {
    debug!(prompt_id = %prompt_id, "Admin rendering prompt");

    let rendered = state
        .prompt_service
        .render(&prompt_id, &request.variables)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(RenderPromptResponse { rendered }))
}

/// GET /admin/prompts/:prompt_id/versions
pub async fn list_versions(
    State(state): State<AppState>,
    RequireAdmin(_): RequireAdmin,
    Path(prompt_id): Path<String>,
) -> Result<Json<ListVersionsResponse>, ApiError> {
    debug!(prompt_id = %prompt_id, "Admin listing prompt versions");

    let prompt = state
        .prompt_service
        .get(&prompt_id)
        .await
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError::not_found(format!("Prompt '{}' not found", prompt_id)))?;

    let versions: Vec<PromptVersionResponse> =
        prompt.history().iter().map(PromptVersionResponse::from).collect();
    let total = versions.len();

    Ok(Json(ListVersionsResponse {
        current_version: prompt.version(),
        versions,
        total,
    }))
}

/// POST /admin/prompts/:prompt_id/revert/:version
pub async fn revert_to_version(
    State(state): State<AppState>,
    RequireAdmin(_): RequireAdmin,
    Path((prompt_id, version)): Path<(String, u32)>,
) -> Result<Json<PromptResponse>, ApiError> {
    debug!(prompt_id = %prompt_id, version = version, "Admin reverting prompt to version");

    let prompt = state
        .prompt_service
        .revert(&prompt_id, version)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(PromptResponse::from(&prompt)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::prompt::PromptId;

    #[test]
    fn test_create_prompt_request_deserialization() {
        let json = r#"{
            "id": "greeting",
            "name": "Greeting Prompt",
            "content": "Hello, ${var:name}!",
            "description": "A greeting prompt",
            "tags": ["greeting", "template"]
        }"#;

        let request: CreatePromptApiRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.id, "greeting");
        assert_eq!(request.name, "Greeting Prompt");
        assert_eq!(request.content, "Hello, ${var:name}!");
        assert_eq!(request.description, Some("A greeting prompt".to_string()));
        assert_eq!(request.tags.len(), 2);
    }

    #[test]
    fn test_create_prompt_request_minimal() {
        let json = r#"{
            "id": "test-prompt",
            "name": "Test",
            "content": "Content"
        }"#;

        let request: CreatePromptApiRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.id, "test-prompt");
        assert!(request.description.is_none());
        assert!(request.tags.is_empty());
    }

    #[test]
    fn test_update_prompt_request_full() {
        let json = r#"{
            "name": "Updated Name",
            "content": "Updated content",
            "description": "Updated description",
            "tags": ["new-tag"]
        }"#;

        let request: UpdatePromptApiRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.name, Some("Updated Name".to_string()));
        assert_eq!(request.content, Some("Updated content".to_string()));
        assert_eq!(request.description, Some("Updated description".to_string()));
        assert_eq!(request.tags, Some(vec!["new-tag".to_string()]));
    }

    #[test]
    fn test_update_prompt_request_partial() {
        let json = r#"{"name": "New Name"}"#;

        let request: UpdatePromptApiRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.name, Some("New Name".to_string()));
        assert!(request.content.is_none());
        assert!(request.description.is_none());
        assert!(request.tags.is_none());
    }

    #[test]
    fn test_render_prompt_request_deserialization() {
        let json = r#"{
            "variables": {"name": "World", "greeting": "Hello"}
        }"#;

        let request: RenderPromptApiRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.variables.get("name"), Some(&"World".to_string()));
        assert_eq!(request.variables.get("greeting"), Some(&"Hello".to_string()));
    }

    #[test]
    fn test_render_prompt_request_empty() {
        let json = r#"{}"#;

        let request: RenderPromptApiRequest = serde_json::from_str(json).unwrap();
        assert!(request.variables.is_empty());
    }

    #[test]
    fn test_prompt_response_from() {
        let id = PromptId::new("test-prompt").unwrap();
        let prompt = Prompt::new(id, "Test Prompt", "Test content");

        let response = PromptResponse::from(&prompt);

        assert_eq!(response.id, "test-prompt");
        assert_eq!(response.name, "Test Prompt");
        assert_eq!(response.content, "Test content");
        assert!(response.description.is_none());
        assert!(response.tags.is_empty());
        assert_eq!(response.version, 1);
        assert!(response.enabled);
    }

    #[test]
    fn test_prompt_response_with_all_fields() {
        let id = PromptId::new("full-prompt").unwrap();
        let prompt = Prompt::new(id, "Full Prompt", "Content with ${var:name}")
            .with_description("A prompt with all fields")
            .with_tags(vec!["tag1".to_string(), "tag2".to_string()]);

        let response = PromptResponse::from(&prompt);

        assert_eq!(response.id, "full-prompt");
        assert_eq!(response.name, "Full Prompt");
        assert_eq!(response.description, Some("A prompt with all fields".to_string()));
        assert_eq!(response.tags, vec!["tag1", "tag2"]);
    }

    #[test]
    fn test_prompt_response_serialization() {
        let id = PromptId::new("test-prompt").unwrap();
        let prompt = Prompt::new(id, "Test Prompt", "Content");
        let response = PromptResponse::from(&prompt);

        let json = serde_json::to_string(&response).unwrap();

        assert!(json.contains("\"id\":\"test-prompt\""));
        assert!(json.contains("\"name\":\"Test Prompt\""));
        assert!(json.contains("\"content\":\"Content\""));
        assert!(json.contains("\"version\":1"));
        assert!(json.contains("\"enabled\":true"));
    }

    #[test]
    fn test_list_prompts_response_serialization() {
        let id = PromptId::new("prompt-1").unwrap();
        let prompt = Prompt::new(id, "Prompt One", "Content");
        let response = PromptResponse::from(&prompt);

        let list_response = ListPromptsResponse {
            prompts: vec![response],
            total: 1,
        };

        let json = serde_json::to_string(&list_response).unwrap();

        assert!(json.contains("\"prompts\":"));
        assert!(json.contains("\"total\":1"));
        assert!(json.contains("\"id\":\"prompt-1\""));
    }

    #[test]
    fn test_render_prompt_response_serialization() {
        let response = RenderPromptResponse {
            rendered: "Hello, World!".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();

        assert!(json.contains("\"rendered\":\"Hello, World!\""));
    }

    #[test]
    fn test_list_versions_response_serialization() {
        let list_response = ListVersionsResponse {
            current_version: 2,
            versions: vec![],
            total: 0,
        };

        let json = serde_json::to_string(&list_response).unwrap();

        assert!(json.contains("\"current_version\":2"));
        assert!(json.contains("\"versions\":[]"));
        assert!(json.contains("\"total\":0"));
    }
}
