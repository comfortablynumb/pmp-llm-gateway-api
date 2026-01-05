//! API key management admin endpoints

use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::api::middleware::RequireApiKey;
use crate::api::state::AppState;
use crate::api::types::ApiError;
use crate::domain::api_key::{ApiKey, ApiKeyPermissions, ApiKeyStatus, ResourcePermission};

/// Request to create a new API key
#[derive(Debug, Clone, Deserialize)]
pub struct CreateApiKeyRequest {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub permissions: PermissionsRequest,
}

/// Permissions in request format
#[derive(Debug, Clone, Default, Deserialize)]
pub struct PermissionsRequest {
    #[serde(default)]
    pub admin: bool,
    #[serde(default)]
    pub models: ResourcePermissionRequest,
    #[serde(default)]
    pub knowledge_bases: ResourcePermissionRequest,
    #[serde(default)]
    pub prompts: ResourcePermissionRequest,
    #[serde(default)]
    pub chains: ResourcePermissionRequest,
}

/// Resource permission in request format
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ResourcePermissionRequest {
    #[default]
    All,
    None,
    Specific(Vec<String>),
}

impl From<ResourcePermissionRequest> for ResourcePermission {
    fn from(req: ResourcePermissionRequest) -> Self {
        match req {
            ResourcePermissionRequest::All => ResourcePermission::All,
            ResourcePermissionRequest::None => ResourcePermission::None,
            ResourcePermissionRequest::Specific(ids) => {
                ResourcePermission::Specific(ids.into_iter().collect())
            }
        }
    }
}

impl From<PermissionsRequest> for ApiKeyPermissions {
    fn from(req: PermissionsRequest) -> Self {
        Self {
            admin: req.admin,
            models: req.models.into(),
            knowledge_bases: req.knowledge_bases.into(),
            prompts: req.prompts.into(),
            chains: req.chains.into(),
        }
    }
}

/// Request to update an API key
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateApiKeyRequest {
    pub permissions: Option<PermissionsRequest>,
}

/// API key response for admin API
#[derive(Debug, Clone, Serialize)]
pub struct ApiKeyResponse {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub key_prefix: String,
    pub status: String,
    pub permissions: PermissionsResponse,
    pub last_used_at: Option<String>,
    pub expires_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// API key response with secret (only on creation)
#[derive(Debug, Clone, Serialize)]
pub struct ApiKeyWithSecretResponse {
    #[serde(flatten)]
    pub api_key: ApiKeyResponse,
    pub secret: String,
}

/// Permissions in response format
#[derive(Debug, Clone, Serialize)]
pub struct PermissionsResponse {
    pub admin: bool,
    pub models: ResourcePermissionResponse,
    pub knowledge_bases: ResourcePermissionResponse,
    pub prompts: ResourcePermissionResponse,
    pub chains: ResourcePermissionResponse,
}

/// Resource permission in response format
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ResourcePermissionResponse {
    All,
    None,
    Specific(Vec<String>),
}

impl From<&ResourcePermission> for ResourcePermissionResponse {
    fn from(perm: &ResourcePermission) -> Self {
        match perm {
            ResourcePermission::All => ResourcePermissionResponse::All,
            ResourcePermission::None => ResourcePermissionResponse::None,
            ResourcePermission::Specific(ids) => {
                ResourcePermissionResponse::Specific(ids.iter().cloned().collect())
            }
        }
    }
}

impl From<&ApiKeyPermissions> for PermissionsResponse {
    fn from(perms: &ApiKeyPermissions) -> Self {
        Self {
            admin: perms.admin,
            models: (&perms.models).into(),
            knowledge_bases: (&perms.knowledge_bases).into(),
            prompts: (&perms.prompts).into(),
            chains: (&perms.chains).into(),
        }
    }
}

fn status_to_string(status: ApiKeyStatus) -> String {
    match status {
        ApiKeyStatus::Active => "active".to_string(),
        ApiKeyStatus::Suspended => "suspended".to_string(),
        ApiKeyStatus::Revoked => "revoked".to_string(),
        ApiKeyStatus::Expired => "expired".to_string(),
    }
}

impl From<&ApiKey> for ApiKeyResponse {
    fn from(key: &ApiKey) -> Self {
        Self {
            id: key.id().as_str().to_string(),
            name: key.name().to_string(),
            description: key.description().map(String::from),
            key_prefix: key.key_prefix().to_string(),
            status: status_to_string(key.status()),
            permissions: key.permissions().into(),
            last_used_at: key.last_used_at().map(|dt| dt.to_rfc3339()),
            expires_at: key.expires_at().map(|dt| dt.to_rfc3339()),
            created_at: key.created_at().to_rfc3339(),
            updated_at: key.updated_at().to_rfc3339(),
        }
    }
}

/// List API keys response
#[derive(Debug, Clone, Serialize)]
pub struct ListApiKeysResponse {
    pub api_keys: Vec<ApiKeyResponse>,
    pub total: usize,
}

/// GET /admin/api-keys
pub async fn list_api_keys(
    State(state): State<AppState>,
    RequireApiKey(api_key): RequireApiKey,
) -> Result<Json<ListApiKeysResponse>, ApiError> {
    if !api_key.permissions().admin {
        return Err(ApiError::forbidden("Admin access required"));
    }

    debug!("Admin listing all API keys");

    let keys = state.api_key_service.list().await.map_err(ApiError::from)?;

    let key_responses: Vec<ApiKeyResponse> = keys.iter().map(ApiKeyResponse::from).collect();
    let total = key_responses.len();

    Ok(Json(ListApiKeysResponse {
        api_keys: key_responses,
        total,
    }))
}

/// POST /admin/api-keys
pub async fn create_api_key(
    State(state): State<AppState>,
    RequireApiKey(api_key): RequireApiKey,
    Json(request): Json<CreateApiKeyRequest>,
) -> Result<Json<ApiKeyWithSecretResponse>, ApiError> {
    if !api_key.permissions().admin {
        return Err(ApiError::forbidden("Admin access required"));
    }

    debug!(name = %request.name, "Admin creating API key");

    let permissions: ApiKeyPermissions = request.permissions.into();

    let (created_key, secret) = state
        .api_key_service
        .create(&request.name, permissions)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(ApiKeyWithSecretResponse {
        api_key: ApiKeyResponse::from(&created_key),
        secret,
    }))
}

/// GET /admin/api-keys/:key_id
pub async fn get_api_key(
    State(state): State<AppState>,
    RequireApiKey(api_key): RequireApiKey,
    Path(key_id): Path<String>,
) -> Result<Json<ApiKeyResponse>, ApiError> {
    if !api_key.permissions().admin {
        return Err(ApiError::forbidden("Admin access required"));
    }

    debug!(key_id = %key_id, "Admin getting API key");

    let key = state
        .api_key_service
        .get(&key_id)
        .await
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError::not_found(format!("API key '{}' not found", key_id)))?;

    Ok(Json(ApiKeyResponse::from(&key)))
}

/// PUT /admin/api-keys/:key_id
pub async fn update_api_key(
    State(state): State<AppState>,
    RequireApiKey(api_key): RequireApiKey,
    Path(key_id): Path<String>,
    Json(request): Json<UpdateApiKeyRequest>,
) -> Result<Json<ApiKeyResponse>, ApiError> {
    if !api_key.permissions().admin {
        return Err(ApiError::forbidden("Admin access required"));
    }

    debug!(key_id = %key_id, "Admin updating API key");

    if let Some(permissions_req) = request.permissions {
        let permissions: ApiKeyPermissions = permissions_req.into();
        state
            .api_key_service
            .update_permissions(&key_id, permissions)
            .await
            .map_err(ApiError::from)?;
    }

    let key = state
        .api_key_service
        .get(&key_id)
        .await
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError::not_found(format!("API key '{}' not found", key_id)))?;

    Ok(Json(ApiKeyResponse::from(&key)))
}

/// DELETE /admin/api-keys/:key_id
pub async fn delete_api_key(
    State(state): State<AppState>,
    RequireApiKey(api_key): RequireApiKey,
    Path(key_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    if !api_key.permissions().admin {
        return Err(ApiError::forbidden("Admin access required"));
    }

    debug!(key_id = %key_id, "Admin deleting API key");

    state
        .api_key_service
        .delete(&key_id)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(serde_json::json!({
        "deleted": true,
        "id": key_id
    })))
}

/// POST /admin/api-keys/:key_id/suspend
pub async fn suspend_api_key(
    State(state): State<AppState>,
    RequireApiKey(api_key): RequireApiKey,
    Path(key_id): Path<String>,
) -> Result<Json<ApiKeyResponse>, ApiError> {
    if !api_key.permissions().admin {
        return Err(ApiError::forbidden("Admin access required"));
    }

    debug!(key_id = %key_id, "Admin suspending API key");

    state
        .api_key_service
        .suspend(&key_id)
        .await
        .map_err(ApiError::from)?;

    let key = state
        .api_key_service
        .get(&key_id)
        .await
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError::not_found(format!("API key '{}' not found", key_id)))?;

    Ok(Json(ApiKeyResponse::from(&key)))
}

/// POST /admin/api-keys/:key_id/activate
pub async fn activate_api_key(
    State(state): State<AppState>,
    RequireApiKey(api_key): RequireApiKey,
    Path(key_id): Path<String>,
) -> Result<Json<ApiKeyResponse>, ApiError> {
    if !api_key.permissions().admin {
        return Err(ApiError::forbidden("Admin access required"));
    }

    debug!(key_id = %key_id, "Admin activating API key");

    state
        .api_key_service
        .activate(&key_id)
        .await
        .map_err(ApiError::from)?;

    let key = state
        .api_key_service
        .get(&key_id)
        .await
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError::not_found(format!("API key '{}' not found", key_id)))?;

    Ok(Json(ApiKeyResponse::from(&key)))
}

/// POST /admin/api-keys/:key_id/revoke
pub async fn revoke_api_key(
    State(state): State<AppState>,
    RequireApiKey(api_key): RequireApiKey,
    Path(key_id): Path<String>,
) -> Result<Json<ApiKeyResponse>, ApiError> {
    if !api_key.permissions().admin {
        return Err(ApiError::forbidden("Admin access required"));
    }

    debug!(key_id = %key_id, "Admin revoking API key");

    state
        .api_key_service
        .revoke(&key_id)
        .await
        .map_err(ApiError::from)?;

    let key = state
        .api_key_service
        .get(&key_id)
        .await
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError::not_found(format!("API key '{}' not found", key_id)))?;

    Ok(Json(ApiKeyResponse::from(&key)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_api_key_request_deserialization() {
        let json = r#"{
            "name": "Test API Key",
            "permissions": {
                "is_admin": false,
                "models": "all"
            }
        }"#;

        let request: CreateApiKeyRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.name, "Test API Key");
        assert!(!request.permissions.admin);
    }

    #[test]
    fn test_permissions_with_specific_resources() {
        let json = r#"{
            "name": "Limited Key",
            "permissions": {
                "is_admin": false,
                "models": {"specific": ["gpt-4", "gpt-3.5"]},
                "knowledge_bases": "none"
            }
        }"#;

        let request: CreateApiKeyRequest = serde_json::from_str(json).unwrap();
        match request.permissions.models {
            ResourcePermissionRequest::Specific(ids) => {
                assert_eq!(ids.len(), 2);
            }
            _ => panic!("Expected Specific permission"),
        }
    }
}
