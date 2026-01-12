//! Models endpoint handlers

use axum::{
    extract::{Path, State},
    Json,
};
use tracing::debug;

use crate::api::middleware::RequireApiKey;
use crate::api::state::AppState;
use crate::api::types::{ApiError, ApiModel, ModelsResponse};

/// GET /v1/models
pub async fn list_models(
    State(state): State<AppState>,
    RequireApiKey(_api_key): RequireApiKey,
) -> Result<Json<ModelsResponse>, ApiError> {
    debug!("Listing all models");

    let models = state
        .model_service
        .list()
        .await
        .map_err(ApiError::from)?;

    let api_models: Vec<ApiModel> = models
        .iter()
        .filter(|m| m.is_enabled())
        .map(ApiModel::from_domain)
        .collect();

    Ok(Json(ModelsResponse::new(api_models)))
}

/// GET /v1/models/:model_id
pub async fn get_model(
    State(state): State<AppState>,
    RequireApiKey(_api_key): RequireApiKey,
    Path(model_id): Path<String>,
) -> Result<Json<ApiModel>, ApiError> {
    debug!(model_id = %model_id, "Getting model");

    let model = state
        .model_service
        .get(&model_id)
        .await
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError::not_found(format!("Model '{}' not found", model_id)))?;

    if !model.is_enabled() {
        return Err(ApiError::not_found(format!("Model '{}' not found", model_id)));
    }

    Ok(Json(ApiModel::from_domain(&model)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_models_response_format() {
        let response = ModelsResponse::new(vec![
            ApiModel {
                id: "gpt-4".to_string(),
                object: "model".to_string(),
                created: 1234567890,
                owned_by: "openai".to_string(),
            },
        ]);

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"object\":\"list\""));
        assert!(json.contains("\"id\":\"gpt-4\""));
    }

    #[test]
    fn test_models_response_empty() {
        let response = ModelsResponse::new(vec![]);

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"object\":\"list\""));
        assert!(json.contains("\"data\":[]"));
    }

    #[test]
    fn test_models_response_multiple() {
        let response = ModelsResponse::new(vec![
            ApiModel {
                id: "gpt-4".to_string(),
                object: "model".to_string(),
                created: 1234567890,
                owned_by: "openai".to_string(),
            },
            ApiModel {
                id: "gpt-3.5-turbo".to_string(),
                object: "model".to_string(),
                created: 1234567800,
                owned_by: "openai".to_string(),
            },
        ]);

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"gpt-4\""));
        assert!(json.contains("\"gpt-3.5-turbo\""));
    }

    #[test]
    fn test_api_model_serialization() {
        let model = ApiModel {
            id: "test-model".to_string(),
            object: "model".to_string(),
            created: 1700000000,
            owned_by: "test-org".to_string(),
        };

        let json = serde_json::to_string(&model).unwrap();
        assert!(json.contains("\"id\":\"test-model\""));
        assert!(json.contains("\"object\":\"model\""));
        assert!(json.contains("\"created\":1700000000"));
        assert!(json.contains("\"owned_by\":\"test-org\""));
    }
}
