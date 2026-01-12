//! External API management admin endpoints

use std::collections::HashMap;

use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::api::middleware::RequireAdmin;
use crate::api::state::AppState;
use crate::api::types::ApiError;
use crate::domain::ExternalApi;

/// Request to create a new external API
#[derive(Debug, Clone, Deserialize)]
pub struct CreateExternalApiRequest {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub base_url: String,
    #[serde(default)]
    pub base_headers: HashMap<String, String>,
}

/// Request to update an external API
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateExternalApiRequest {
    pub name: Option<String>,
    pub description: Option<Option<String>>,
    pub base_url: Option<String>,
    pub base_headers: Option<HashMap<String, String>>,
    pub enabled: Option<bool>,
}

/// External API response
#[derive(Debug, Clone, Serialize)]
pub struct ExternalApiResponse {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub base_url: String,
    pub base_headers: HashMap<String, String>,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl From<&ExternalApi> for ExternalApiResponse {
    fn from(api: &ExternalApi) -> Self {
        Self {
            id: api.id().as_str().to_string(),
            name: api.name().to_string(),
            description: api.description().map(|s| s.to_string()),
            base_url: api.base_url().to_string(),
            base_headers: api.base_headers().clone(),
            enabled: api.is_enabled(),
            created_at: api.created_at().to_rfc3339(),
            updated_at: api.updated_at().to_rfc3339(),
        }
    }
}

/// List external APIs response
#[derive(Debug, Clone, Serialize)]
pub struct ListExternalApisResponse {
    pub external_apis: Vec<ExternalApiResponse>,
    pub total: usize,
}

/// GET /admin/external-apis
/// List all external APIs
pub async fn list_external_apis(
    State(state): State<AppState>,
    RequireAdmin(_): RequireAdmin,
) -> Result<Json<ListExternalApisResponse>, ApiError> {
    debug!("Admin listing all external APIs");

    let apis = state
        .external_api_service
        .list()
        .await
        .map_err(ApiError::from)?;

    let responses: Vec<ExternalApiResponse> = apis.iter().map(ExternalApiResponse::from).collect();
    let total = responses.len();

    Ok(Json(ListExternalApisResponse {
        external_apis: responses,
        total,
    }))
}

/// POST /admin/external-apis
/// Create a new external API
pub async fn create_external_api(
    State(state): State<AppState>,
    RequireAdmin(_): RequireAdmin,
    Json(request): Json<CreateExternalApiRequest>,
) -> Result<Json<ExternalApiResponse>, ApiError> {
    debug!(api_id = %request.id, "Admin creating external API");

    let create_request = crate::infrastructure::external_api::CreateExternalApiRequest {
        id: request.id,
        name: request.name,
        description: request.description,
        base_url: request.base_url,
        base_headers: request.base_headers,
    };

    let api = state
        .external_api_service
        .create(create_request)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(ExternalApiResponse::from(&api)))
}

/// GET /admin/external-apis/:api_id
/// Get a specific external API
pub async fn get_external_api(
    State(state): State<AppState>,
    RequireAdmin(_): RequireAdmin,
    Path(api_id): Path<String>,
) -> Result<Json<ExternalApiResponse>, ApiError> {
    debug!(api_id = %api_id, "Admin getting external API");

    let api = state
        .external_api_service
        .get(&api_id)
        .await
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError::not_found(format!("External API '{}' not found", api_id)))?;

    Ok(Json(ExternalApiResponse::from(&api)))
}

/// PUT /admin/external-apis/:api_id
/// Update an external API
pub async fn update_external_api(
    State(state): State<AppState>,
    RequireAdmin(_): RequireAdmin,
    Path(api_id): Path<String>,
    Json(request): Json<UpdateExternalApiRequest>,
) -> Result<Json<ExternalApiResponse>, ApiError> {
    debug!(api_id = %api_id, "Admin updating external API");

    let update_request = crate::infrastructure::external_api::UpdateExternalApiRequest {
        name: request.name,
        description: request.description,
        base_url: request.base_url,
        base_headers: request.base_headers,
        enabled: request.enabled,
    };

    let api = state
        .external_api_service
        .update(&api_id, update_request)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(ExternalApiResponse::from(&api)))
}

/// DELETE /admin/external-apis/:api_id
/// Delete an external API
pub async fn delete_external_api(
    State(state): State<AppState>,
    RequireAdmin(_): RequireAdmin,
    Path(api_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    debug!(api_id = %api_id, "Admin deleting external API");

    // TODO: Check if any workflows are using this external API

    state
        .external_api_service
        .delete(&api_id)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(serde_json::json!({
        "deleted": true,
        "id": api_id
    })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::external_api::ExternalApiId;

    #[test]
    fn test_deserialize_create_request() {
        let json = r#"{
            "id": "my-api",
            "name": "My API",
            "description": "Test API",
            "base_url": "https://api.example.com",
            "base_headers": {"X-Custom": "value"}
        }"#;

        let request: CreateExternalApiRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.id, "my-api");
        assert_eq!(request.name, "My API");
        assert_eq!(request.base_url, "https://api.example.com");
        assert_eq!(
            request.base_headers.get("X-Custom"),
            Some(&"value".to_string())
        );
    }

    #[test]
    fn test_deserialize_create_request_minimal() {
        let json = r#"{
            "id": "my-api",
            "name": "My API",
            "base_url": "https://api.example.com"
        }"#;

        let request: CreateExternalApiRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.id, "my-api");
        assert!(request.description.is_none());
        assert!(request.base_headers.is_empty());
    }

    #[test]
    fn test_deserialize_update_request_full() {
        let json = r#"{
            "name": "Updated API",
            "description": "New description",
            "base_url": "https://new-api.example.com",
            "base_headers": {"Authorization": "Bearer token"},
            "enabled": false
        }"#;

        let request: UpdateExternalApiRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.name, Some("Updated API".to_string()));
        assert_eq!(request.description, Some(Some("New description".to_string())));
        assert_eq!(request.base_url, Some("https://new-api.example.com".to_string()));
        assert!(request.base_headers.is_some());
        assert_eq!(request.enabled, Some(false));
    }

    #[test]
    fn test_deserialize_update_request_partial() {
        let json = r#"{"name": "New Name"}"#;

        let request: UpdateExternalApiRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.name, Some("New Name".to_string()));
        assert!(request.description.is_none());
        assert!(request.base_url.is_none());
        assert!(request.base_headers.is_none());
        assert!(request.enabled.is_none());
    }

    #[test]
    fn test_deserialize_update_request_with_description() {
        let json = r#"{"description": "New description"}"#;

        let request: UpdateExternalApiRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.description, Some(Some("New description".to_string())));
    }

    #[test]
    fn test_external_api_response_from() {
        let id = ExternalApiId::new("test-api").unwrap();
        let api = ExternalApi::builder(id, "Test API", "https://api.example.com")
            .unwrap()
            .description("A test API")
            .header("X-Custom", "value")
            .build();

        let response = ExternalApiResponse::from(&api);

        assert_eq!(response.id, "test-api");
        assert_eq!(response.name, "Test API");
        assert_eq!(response.description, Some("A test API".to_string()));
        assert_eq!(response.base_url, "https://api.example.com");
        assert_eq!(response.base_headers.get("X-Custom"), Some(&"value".to_string()));
        assert!(response.enabled);
    }

    #[test]
    fn test_external_api_response_serialization() {
        let id = ExternalApiId::new("test-api").unwrap();
        let api = ExternalApi::new(id, "Test API", "https://api.example.com").unwrap();
        let response = ExternalApiResponse::from(&api);

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"id\":\"test-api\""));
        assert!(json.contains("\"name\":\"Test API\""));
        assert!(json.contains("\"base_url\":\"https://api.example.com\""));
        assert!(json.contains("\"enabled\":true"));
    }

    #[test]
    fn test_list_external_apis_response_serialization() {
        let id = ExternalApiId::new("api-1").unwrap();
        let api = ExternalApi::new(id, "API One", "https://api1.example.com").unwrap();
        let response = ExternalApiResponse::from(&api);

        let list_response = ListExternalApisResponse {
            external_apis: vec![response],
            total: 1,
        };

        let json = serde_json::to_string(&list_response).unwrap();
        assert!(json.contains("\"external_apis\""));
        assert!(json.contains("\"total\":1"));
        assert!(json.contains("\"id\":\"api-1\""));
    }

    #[test]
    fn test_list_external_apis_response_empty() {
        let list_response = ListExternalApisResponse {
            external_apis: vec![],
            total: 0,
        };

        let json = serde_json::to_string(&list_response).unwrap();
        assert!(json.contains("\"external_apis\":[]"));
        assert!(json.contains("\"total\":0"));
    }
}
