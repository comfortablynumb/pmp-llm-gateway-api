//! Prompt management admin endpoints

use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::debug;

use crate::api::middleware::RequireApiKey;
use crate::api::state::AppState;
use crate::api::types::ApiError;
use crate::domain::prompt::Prompt;
use crate::infrastructure::services::{CreatePromptRequest, UpdatePromptRequest};

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
}

/// Request to update a prompt
#[derive(Debug, Clone, Deserialize)]
pub struct UpdatePromptApiRequest {
    pub name: Option<String>,
    pub content: Option<String>,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
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
    pub version: u32,
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
            version: prompt.version(),
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

/// GET /admin/prompts
pub async fn list_prompts(
    State(state): State<AppState>,
    RequireApiKey(api_key): RequireApiKey,
) -> Result<Json<ListPromptsResponse>, ApiError> {
    if !api_key.permissions().admin {
        return Err(ApiError::forbidden("Admin access required"));
    }

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
    RequireApiKey(api_key): RequireApiKey,
    Json(request): Json<CreatePromptApiRequest>,
) -> Result<Json<PromptResponse>, ApiError> {
    if !api_key.permissions().admin {
        return Err(ApiError::forbidden("Admin access required"));
    }

    debug!(prompt_id = %request.id, "Admin creating prompt");

    let create_request = CreatePromptRequest {
        id: request.id,
        name: request.name,
        description: request.description,
        content: request.content,
        tags: request.tags,
        enabled: true,
        max_history: None,
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
    RequireApiKey(api_key): RequireApiKey,
    Path(prompt_id): Path<String>,
) -> Result<Json<PromptResponse>, ApiError> {
    if !api_key.permissions().admin {
        return Err(ApiError::forbidden("Admin access required"));
    }

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
    RequireApiKey(api_key): RequireApiKey,
    Path(prompt_id): Path<String>,
    Json(request): Json<UpdatePromptApiRequest>,
) -> Result<Json<PromptResponse>, ApiError> {
    if !api_key.permissions().admin {
        return Err(ApiError::forbidden("Admin access required"));
    }

    debug!(prompt_id = %prompt_id, "Admin updating prompt");

    let update_request = UpdatePromptRequest {
        name: request.name,
        description: request.description,
        content: request.content,
        content_message: None,
        tags: request.tags,
        enabled: None,
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
    RequireApiKey(api_key): RequireApiKey,
    Path(prompt_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    if !api_key.permissions().admin {
        return Err(ApiError::forbidden("Admin access required"));
    }

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
    RequireApiKey(api_key): RequireApiKey,
    Path(prompt_id): Path<String>,
    Json(request): Json<RenderPromptApiRequest>,
) -> Result<Json<RenderPromptResponse>, ApiError> {
    if !api_key.permissions().admin {
        return Err(ApiError::forbidden("Admin access required"));
    }

    debug!(prompt_id = %prompt_id, "Admin rendering prompt");

    let rendered = state
        .prompt_service
        .render(&prompt_id, &request.variables)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(RenderPromptResponse { rendered }))
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(request.tags.len(), 2);
    }

    #[test]
    fn test_render_prompt_request_deserialization() {
        let json = r#"{
            "variables": {"name": "World", "greeting": "Hello"}
        }"#;

        let request: RenderPromptApiRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.variables.get("name"), Some(&"World".to_string()));
    }
}
