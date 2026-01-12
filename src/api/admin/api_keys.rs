//! API key management admin endpoints

use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::api::middleware::RequireAdmin;
use crate::api::state::AppState;
use crate::api::types::ApiError;
use crate::domain::api_key::{ApiKey, ApiKeyPermissions, ApiKeyStatus, ResourcePermission};

/// Request to create a new API key
#[derive(Debug, Clone, Deserialize)]
pub struct CreateApiKeyRequest {
    pub name: String,
    pub team_id: String,
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
    pub team_id: String,
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
            team_id: key.team_id().as_str().to_string(),
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
    RequireAdmin(_): RequireAdmin,
) -> Result<Json<ListApiKeysResponse>, ApiError> {
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
    RequireAdmin(_): RequireAdmin,
    Json(request): Json<CreateApiKeyRequest>,
) -> Result<Json<ApiKeyWithSecretResponse>, ApiError> {
    debug!(name = %request.name, team_id = %request.team_id, "Admin creating API key");

    let permissions: ApiKeyPermissions = request.permissions.into();

    let (created_key, secret) = state
        .api_key_service
        .create(&request.name, &request.team_id, permissions)
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
    RequireAdmin(_): RequireAdmin,
    Path(key_id): Path<String>,
) -> Result<Json<ApiKeyResponse>, ApiError> {
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
    RequireAdmin(_): RequireAdmin,
    Path(key_id): Path<String>,
    Json(request): Json<UpdateApiKeyRequest>,
) -> Result<Json<ApiKeyResponse>, ApiError> {
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
    RequireAdmin(_): RequireAdmin,
    Path(key_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
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
    RequireAdmin(_): RequireAdmin,
    Path(key_id): Path<String>,
) -> Result<Json<ApiKeyResponse>, ApiError> {
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
    RequireAdmin(_): RequireAdmin,
    Path(key_id): Path<String>,
) -> Result<Json<ApiKeyResponse>, ApiError> {
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
    RequireAdmin(_): RequireAdmin,
    Path(key_id): Path<String>,
) -> Result<Json<ApiKeyResponse>, ApiError> {
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
            "team_id": "administrators",
            "permissions": {
                "is_admin": false,
                "models": "all"
            }
        }"#;

        let request: CreateApiKeyRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.name, "Test API Key");
        assert_eq!(request.team_id, "administrators");
        assert!(!request.permissions.admin);
    }

    #[test]
    fn test_create_api_key_request_minimal() {
        let json = r#"{
            "name": "Minimal Key",
            "team_id": "my-team"
        }"#;

        let request: CreateApiKeyRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.name, "Minimal Key");
        assert_eq!(request.team_id, "my-team");
        assert!(request.description.is_none());
        assert!(!request.permissions.admin);
    }

    #[test]
    fn test_create_api_key_request_with_description() {
        let json = r#"{
            "name": "Full Key",
            "team_id": "team-1",
            "description": "A test API key"
        }"#;

        let request: CreateApiKeyRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.description, Some("A test API key".to_string()));
    }

    #[test]
    fn test_permissions_with_specific_resources() {
        let json = r#"{
            "name": "Limited Key",
            "team_id": "test-team",
            "permissions": {
                "is_admin": false,
                "models": {"specific": ["gpt-4", "gpt-3.5"]},
                "knowledge_bases": "none"
            }
        }"#;

        let request: CreateApiKeyRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.team_id, "test-team");

        match request.permissions.models {
            ResourcePermissionRequest::Specific(ids) => {
                assert_eq!(ids.len(), 2);
            }
            _ => panic!("Expected Specific permission"),
        }
    }

    #[test]
    fn test_permissions_admin_flag() {
        let json = r#"{
            "name": "Admin Key",
            "team_id": "admins",
            "permissions": {
                "admin": true
            }
        }"#;

        let request: CreateApiKeyRequest = serde_json::from_str(json).unwrap();
        assert!(request.permissions.admin);
    }

    #[test]
    fn test_update_api_key_request_empty() {
        let json = r#"{}"#;

        let request: UpdateApiKeyRequest = serde_json::from_str(json).unwrap();
        assert!(request.permissions.is_none());
    }

    #[test]
    fn test_update_api_key_request_with_permissions() {
        let json = r#"{
            "permissions": {
                "admin": true,
                "models": "all"
            }
        }"#;

        let request: UpdateApiKeyRequest = serde_json::from_str(json).unwrap();
        assert!(request.permissions.is_some());
        assert!(request.permissions.unwrap().admin);
    }

    #[test]
    fn test_status_to_string() {
        assert_eq!(status_to_string(ApiKeyStatus::Active), "active");
        assert_eq!(status_to_string(ApiKeyStatus::Suspended), "suspended");
        assert_eq!(status_to_string(ApiKeyStatus::Revoked), "revoked");
        assert_eq!(status_to_string(ApiKeyStatus::Expired), "expired");
    }

    #[test]
    fn test_resource_permission_request_to_domain_all() {
        let req = ResourcePermissionRequest::All;
        let perm: ResourcePermission = req.into();
        assert!(matches!(perm, ResourcePermission::All));
    }

    #[test]
    fn test_resource_permission_request_to_domain_none() {
        let req = ResourcePermissionRequest::None;
        let perm: ResourcePermission = req.into();
        assert!(matches!(perm, ResourcePermission::None));
    }

    #[test]
    fn test_resource_permission_request_to_domain_specific() {
        let req = ResourcePermissionRequest::Specific(vec!["model-1".to_string(), "model-2".to_string()]);
        let perm: ResourcePermission = req.into();

        if let ResourcePermission::Specific(ids) = perm {
            assert_eq!(ids.len(), 2);
            assert!(ids.contains("model-1"));
            assert!(ids.contains("model-2"));
        } else {
            panic!("Expected Specific permission");
        }
    }

    #[test]
    fn test_permissions_request_to_domain() {
        let req = PermissionsRequest {
            admin: true,
            models: ResourcePermissionRequest::All,
            knowledge_bases: ResourcePermissionRequest::None,
            prompts: ResourcePermissionRequest::Specific(vec!["prompt-1".to_string()]),
            chains: ResourcePermissionRequest::All,
        };

        let perms: ApiKeyPermissions = req.into();
        assert!(perms.admin);
        assert!(matches!(perms.models, ResourcePermission::All));
        assert!(matches!(perms.knowledge_bases, ResourcePermission::None));
        assert!(matches!(perms.prompts, ResourcePermission::Specific(_)));
    }

    #[test]
    fn test_resource_permission_response_from_all() {
        let perm = ResourcePermission::All;
        let resp: ResourcePermissionResponse = (&perm).into();
        assert!(matches!(resp, ResourcePermissionResponse::All));
    }

    #[test]
    fn test_resource_permission_response_from_none() {
        let perm = ResourcePermission::None;
        let resp: ResourcePermissionResponse = (&perm).into();
        assert!(matches!(resp, ResourcePermissionResponse::None));
    }

    #[test]
    fn test_resource_permission_response_from_specific() {
        let mut ids = std::collections::HashSet::new();
        ids.insert("id-1".to_string());
        ids.insert("id-2".to_string());
        let perm = ResourcePermission::Specific(ids);
        let resp: ResourcePermissionResponse = (&perm).into();

        if let ResourcePermissionResponse::Specific(vec) = resp {
            assert_eq!(vec.len(), 2);
        } else {
            panic!("Expected Specific response");
        }
    }

    #[test]
    fn test_permissions_response_from() {
        let perms = ApiKeyPermissions {
            admin: false,
            models: ResourcePermission::All,
            knowledge_bases: ResourcePermission::None,
            prompts: ResourcePermission::All,
            chains: ResourcePermission::All,
        };

        let resp: PermissionsResponse = (&perms).into();
        assert!(!resp.admin);
        assert!(matches!(resp.models, ResourcePermissionResponse::All));
        assert!(matches!(resp.knowledge_bases, ResourcePermissionResponse::None));
    }

    #[test]
    fn test_permissions_response_serialization() {
        let resp = PermissionsResponse {
            admin: true,
            models: ResourcePermissionResponse::All,
            knowledge_bases: ResourcePermissionResponse::None,
            prompts: ResourcePermissionResponse::Specific(vec!["p1".to_string()]),
            chains: ResourcePermissionResponse::All,
        };

        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"admin\":true"));
        assert!(json.contains("\"models\":\"all\""));
        assert!(json.contains("\"knowledge_bases\":\"none\""));
    }

    #[test]
    fn test_api_key_response_serialization() {
        let resp = ApiKeyResponse {
            id: "key-1".to_string(),
            name: "Test Key".to_string(),
            team_id: "team-1".to_string(),
            description: Some("A test key".to_string()),
            key_prefix: "pk_test_abc123".to_string(),
            status: "active".to_string(),
            permissions: PermissionsResponse {
                admin: false,
                models: ResourcePermissionResponse::All,
                knowledge_bases: ResourcePermissionResponse::All,
                prompts: ResourcePermissionResponse::All,
                chains: ResourcePermissionResponse::All,
            },
            last_used_at: None,
            expires_at: None,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"id\":\"key-1\""));
        assert!(json.contains("\"team_id\":\"team-1\""));
        assert!(json.contains("\"status\":\"active\""));
        assert!(json.contains("\"key_prefix\":\"pk_test_abc123\""));
    }

    #[test]
    fn test_api_key_with_secret_response_serialization() {
        let resp = ApiKeyWithSecretResponse {
            api_key: ApiKeyResponse {
                id: "key-1".to_string(),
                name: "New Key".to_string(),
                team_id: "team-1".to_string(),
                description: None,
                key_prefix: "pk_live_xyz".to_string(),
                status: "active".to_string(),
                permissions: PermissionsResponse {
                    admin: false,
                    models: ResourcePermissionResponse::All,
                    knowledge_bases: ResourcePermissionResponse::All,
                    prompts: ResourcePermissionResponse::All,
                    chains: ResourcePermissionResponse::All,
                },
                last_used_at: None,
                expires_at: None,
                created_at: "2024-01-01T00:00:00Z".to_string(),
                updated_at: "2024-01-01T00:00:00Z".to_string(),
            },
            secret: "pk_live_xyz_secretkey123".to_string(),
        };

        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"secret\":\"pk_live_xyz_secretkey123\""));
        assert!(json.contains("\"id\":\"key-1\""));
    }

    #[test]
    fn test_list_api_keys_response_serialization() {
        let resp = ListApiKeysResponse {
            api_keys: vec![],
            total: 0,
        };

        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"api_keys\":[]"));
        assert!(json.contains("\"total\":0"));
    }

    #[test]
    fn test_resource_permission_request_deserialization_all() {
        let json = r#""all""#;
        let req: ResourcePermissionRequest = serde_json::from_str(json).unwrap();
        assert!(matches!(req, ResourcePermissionRequest::All));
    }

    #[test]
    fn test_resource_permission_request_deserialization_none() {
        let json = r#""none""#;
        let req: ResourcePermissionRequest = serde_json::from_str(json).unwrap();
        assert!(matches!(req, ResourcePermissionRequest::None));
    }

    #[test]
    fn test_resource_permission_request_deserialization_specific() {
        let json = r#"{"specific": ["a", "b", "c"]}"#;
        let req: ResourcePermissionRequest = serde_json::from_str(json).unwrap();

        if let ResourcePermissionRequest::Specific(ids) = req {
            assert_eq!(ids.len(), 3);
        } else {
            panic!("Expected Specific");
        }
    }
}
