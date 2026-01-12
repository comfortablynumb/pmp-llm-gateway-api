//! Admin authentication middleware
//!
//! Allows either:
//! - API key with admin=true permission
//! - Valid JWT token from an authenticated user

use axum::{
    extract::FromRequestParts,
    http::request::Parts,
};
use tracing::debug;

use crate::api::state::AppState;
use crate::api::types::ApiError;
use crate::domain::api_key::ApiKey;
use crate::domain::user::User;

use super::auth::RequireApiKey;
use super::user_auth::try_jwt_auth;

/// Represents the type of admin authentication used
#[derive(Debug, Clone)]
pub enum AdminAuth {
    /// Authenticated via API key with admin permission
    ApiKey(ApiKey),
    /// Authenticated via JWT token
    User(User),
}

impl AdminAuth {
    /// Get the identifier of the authenticated entity
    pub fn identifier(&self) -> String {
        match self {
            AdminAuth::ApiKey(key) => format!("api_key:{}", key.id()),
            AdminAuth::User(user) => format!("user:{}", user.id()),
        }
    }
}

/// Extractor that requires admin access via either API key or JWT
///
/// Authentication methods (tried in order):
/// 1. JWT token from Authorization: Bearer <jwt_token>
/// 2. API key from Authorization: Bearer <api_key> or X-API-Key header
///
/// For API key auth, the key must have admin=true permission.
#[derive(Debug, Clone)]
pub struct RequireAdmin(pub AdminAuth);

impl FromRequestParts<AppState> for RequireAdmin {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        // Try JWT authentication first
        if let Some(user) = try_jwt_auth(&parts.headers, state).await {
            debug!(user_id = %user.id(), "Admin access via JWT");
            return Ok(RequireAdmin(AdminAuth::User(user)));
        }

        // Fall back to API key authentication
        match RequireApiKey::from_request_parts(parts, state).await {
            Ok(RequireApiKey(api_key)) => {
                // Check admin permission
                if !api_key.permissions().admin {
                    return Err(ApiError::forbidden("Admin access required"));
                }

                debug!(api_key_id = %api_key.id(), "Admin access via API key");
                Ok(RequireAdmin(AdminAuth::ApiKey(api_key)))
            }
            Err(_) => {
                Err(ApiError::unauthorized(
                    "Admin access required. Provide JWT token or API key with admin permission",
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::team::{TeamId, TeamRole};

    fn admin_team() -> TeamId {
        TeamId::administrators()
    }

    #[test]
    fn test_admin_auth_identifier_api_key() {
        use crate::domain::api_key::{ApiKeyId, ApiKeyPermissions};

        let key_id = ApiKeyId::new("test-key").unwrap();
        let api_key = ApiKey::new(key_id, "Test Key", "hash", "pk_test_", admin_team())
            .with_permissions(ApiKeyPermissions::full_access());

        let auth = AdminAuth::ApiKey(api_key);
        assert!(auth.identifier().starts_with("api_key:"));
    }

    #[test]
    fn test_admin_auth_identifier_user() {
        use crate::domain::user::UserId;

        let user_id = UserId::new("test-user").unwrap();
        let user = User::new(user_id, "testuser", "hash", admin_team(), TeamRole::Member);

        let auth = AdminAuth::User(user);
        assert!(auth.identifier().starts_with("user:"));
    }
}
