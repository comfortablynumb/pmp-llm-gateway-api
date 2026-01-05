//! Model management admin endpoints

use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::api::middleware::RequireApiKey;
use crate::api::state::AppState;
use crate::api::types::ApiError;
use crate::domain::credentials::CredentialType;
use crate::domain::model::{Model, ModelConfig};
use crate::infrastructure::services::{CreateModelRequest, UpdateModelRequest};

/// Request to create a new model
#[derive(Debug, Clone, Deserialize)]
pub struct CreateModelApiRequest {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub provider_model: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub config: ModelConfigRequest,
}

fn default_enabled() -> bool {
    true
}

/// Request to update a model
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateModelApiRequest {
    pub name: Option<String>,
    pub provider_model: Option<String>,
    pub enabled: Option<bool>,
    pub config: Option<ModelConfigRequest>,
}

/// Model configuration in request format
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ModelConfigRequest {
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub top_p: Option<f32>,
    pub presence_penalty: Option<f32>,
    pub frequency_penalty: Option<f32>,
}

/// Model response for admin API
#[derive(Debug, Clone, Serialize)]
pub struct ModelResponse {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub provider_model: String,
    pub enabled: bool,
    pub config: ModelConfigResponse,
    pub config_version: u32,
    pub created_at: String,
    pub updated_at: String,
}

/// Model configuration in response format
#[derive(Debug, Clone, Serialize)]
pub struct ModelConfigResponse {
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub top_p: Option<f32>,
    pub presence_penalty: Option<f32>,
    pub frequency_penalty: Option<f32>,
}

fn credential_type_to_string(ct: &CredentialType) -> String {
    match ct {
        CredentialType::OpenAi => "openai".to_string(),
        CredentialType::Anthropic => "anthropic".to_string(),
        CredentialType::AzureOpenAi => "azure_openai".to_string(),
        CredentialType::AwsBedrock => "aws_bedrock".to_string(),
        CredentialType::Custom(s) => s.clone(),
    }
}

fn parse_credential_type(s: &str) -> Result<CredentialType, ApiError> {
    match s.to_lowercase().as_str() {
        "openai" => Ok(CredentialType::OpenAi),
        "anthropic" => Ok(CredentialType::Anthropic),
        "azure_openai" | "azure-openai" | "azureopenai" => Ok(CredentialType::AzureOpenAi),
        "aws_bedrock" | "aws-bedrock" | "awsbedrock" | "bedrock" => Ok(CredentialType::AwsBedrock),
        other => Ok(CredentialType::Custom(other.to_string())),
    }
}

impl From<&Model> for ModelResponse {
    fn from(model: &Model) -> Self {
        let config = model.config();
        Self {
            id: model.id().as_str().to_string(),
            name: model.name().to_string(),
            provider: credential_type_to_string(model.provider()),
            provider_model: model.provider_model().to_string(),
            enabled: model.is_enabled(),
            config: ModelConfigResponse {
                temperature: config.temperature,
                max_tokens: config.max_tokens,
                top_p: config.top_p,
                presence_penalty: config.presence_penalty,
                frequency_penalty: config.frequency_penalty,
            },
            config_version: model.version(),
            created_at: model.created_at().to_rfc3339(),
            updated_at: model.updated_at().to_rfc3339(),
        }
    }
}

/// List models response
#[derive(Debug, Clone, Serialize)]
pub struct ListModelsResponse {
    pub models: Vec<ModelResponse>,
    pub total: usize,
}

fn build_model_config(req: &ModelConfigRequest) -> Option<ModelConfig> {
    let mut config = ModelConfig::new();
    let mut has_value = false;

    if let Some(temp) = req.temperature {
        config = config.with_temperature(temp);
        has_value = true;
    }

    if let Some(max_tokens) = req.max_tokens {
        config = config.with_max_tokens(max_tokens);
        has_value = true;
    }

    if let Some(top_p) = req.top_p {
        config = config.with_top_p(top_p);
        has_value = true;
    }

    if let Some(presence) = req.presence_penalty {
        config = config.with_presence_penalty(presence);
        has_value = true;
    }

    if let Some(frequency) = req.frequency_penalty {
        config = config.with_frequency_penalty(frequency);
        has_value = true;
    }

    if has_value {
        Some(config)
    } else {
        None
    }
}

/// GET /admin/models
pub async fn list_models(
    State(state): State<AppState>,
    RequireApiKey(api_key): RequireApiKey,
) -> Result<Json<ListModelsResponse>, ApiError> {
    if !api_key.permissions().admin {
        return Err(ApiError::forbidden("Admin access required"));
    }

    debug!("Admin listing all models");

    let models = state.model_service.list().await.map_err(ApiError::from)?;

    let model_responses: Vec<ModelResponse> = models.iter().map(ModelResponse::from).collect();
    let total = model_responses.len();

    Ok(Json(ListModelsResponse {
        models: model_responses,
        total,
    }))
}

/// POST /admin/models
pub async fn create_model(
    State(state): State<AppState>,
    RequireApiKey(api_key): RequireApiKey,
    Json(request): Json<CreateModelApiRequest>,
) -> Result<Json<ModelResponse>, ApiError> {
    if !api_key.permissions().admin {
        return Err(ApiError::forbidden("Admin access required"));
    }

    debug!(model_id = %request.id, "Admin creating model");

    let provider = parse_credential_type(&request.provider)?;

    let create_request = CreateModelRequest {
        id: request.id,
        name: request.name,
        description: None,
        provider,
        provider_model: request.provider_model,
        config: build_model_config(&request.config),
        enabled: request.enabled,
    };

    let model = state
        .model_service
        .create(create_request)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(ModelResponse::from(&model)))
}

/// GET /admin/models/:model_id
pub async fn get_model(
    State(state): State<AppState>,
    RequireApiKey(api_key): RequireApiKey,
    Path(model_id): Path<String>,
) -> Result<Json<ModelResponse>, ApiError> {
    if !api_key.permissions().admin {
        return Err(ApiError::forbidden("Admin access required"));
    }

    debug!(model_id = %model_id, "Admin getting model");

    let model = state
        .model_service
        .get(&model_id)
        .await
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError::not_found(format!("Model '{}' not found", model_id)))?;

    Ok(Json(ModelResponse::from(&model)))
}

/// PUT /admin/models/:model_id
pub async fn update_model(
    State(state): State<AppState>,
    RequireApiKey(api_key): RequireApiKey,
    Path(model_id): Path<String>,
    Json(request): Json<UpdateModelApiRequest>,
) -> Result<Json<ModelResponse>, ApiError> {
    if !api_key.permissions().admin {
        return Err(ApiError::forbidden("Admin access required"));
    }

    debug!(model_id = %model_id, "Admin updating model");

    let update_request = UpdateModelRequest {
        name: request.name,
        description: None,
        provider_model: request.provider_model,
        config: request.config.as_ref().and_then(build_model_config),
        enabled: request.enabled,
    };

    let model = state
        .model_service
        .update(&model_id, update_request)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(ModelResponse::from(&model)))
}

/// DELETE /admin/models/:model_id
pub async fn delete_model(
    State(state): State<AppState>,
    RequireApiKey(api_key): RequireApiKey,
    Path(model_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    if !api_key.permissions().admin {
        return Err(ApiError::forbidden("Admin access required"));
    }

    debug!(model_id = %model_id, "Admin deleting model");

    state
        .model_service
        .delete(&model_id)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(serde_json::json!({
        "deleted": true,
        "id": model_id
    })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_model_request_deserialization() {
        let json = r#"{
            "id": "my-gpt4",
            "name": "My GPT-4",
            "provider": "openai",
            "provider_model": "gpt-4",
            "enabled": true,
            "config": {
                "temperature": 0.7,
                "max_tokens": 1000
            }
        }"#;

        let request: CreateModelApiRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.id, "my-gpt4");
        assert_eq!(request.provider, "openai");
        assert!(request.enabled);
        assert_eq!(request.config.temperature, Some(0.7));
    }

    #[test]
    fn test_parse_credential_type() {
        assert!(matches!(
            parse_credential_type("openai").unwrap(),
            CredentialType::OpenAi
        ));
        assert!(matches!(
            parse_credential_type("anthropic").unwrap(),
            CredentialType::Anthropic
        ));
        assert!(matches!(
            parse_credential_type("azure_openai").unwrap(),
            CredentialType::AzureOpenAi
        ));
    }
}
