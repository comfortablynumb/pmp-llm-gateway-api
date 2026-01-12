//! User authentication middleware using JWT tokens

use axum::{
    extract::FromRequestParts,
    http::{header, request::Parts},
};
use tracing::debug;

use crate::api::state::AppState;
use crate::api::types::ApiError;
use crate::domain::user::User;

/// Extractor that requires a valid JWT token
///
/// Extracts the JWT token from:
/// - Authorization header: `Bearer <jwt_token>`
#[derive(Debug, Clone)]
pub struct RequireUser(pub User);

impl FromRequestParts<AppState> for RequireUser {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let token = extract_jwt_token(&parts.headers)?;

        debug!("Validating JWT token");

        // Validate the JWT token
        let claims = state
            .jwt_service
            .validate(&token)
            .map_err(|e| ApiError::unauthorized(format!("Invalid token: {}", e)))?;

        // Look up the user
        let user = state
            .user_service
            .get(claims.user_id())
            .await
            .map_err(|e| ApiError::internal(e.to_string()))?
            .ok_or_else(|| ApiError::unauthorized("User not found"))?;

        // Check if user is still active
        if !user.is_active() {
            return Err(ApiError::unauthorized("User account is suspended"));
        }

        Ok(RequireUser(user))
    }
}

/// Extract JWT token from Authorization header
pub fn extract_jwt_token(headers: &axum::http::HeaderMap) -> Result<String, ApiError> {
    if let Some(auth_header) = headers.get(header::AUTHORIZATION) {
        let auth_str = auth_header
            .to_str()
            .map_err(|_| ApiError::bad_request("Invalid Authorization header encoding"))?;

        if let Some(token) = auth_str.strip_prefix("Bearer ") {
            return Ok(token.trim().to_string());
        }
    }

    Err(ApiError::unauthorized(
        "Authentication required. Provide JWT token via 'Authorization: Bearer <token>' header",
    ))
}

/// Try to extract and validate a JWT token, returns None if not present or invalid
pub async fn try_jwt_auth(
    headers: &axum::http::HeaderMap,
    state: &AppState,
) -> Option<User> {
    let token = extract_jwt_token(headers).ok()?;

    let claims = state.jwt_service.validate(&token).ok()?;

    let user = state
        .user_service
        .get(claims.user_id())
        .await
        .ok()
        .flatten()?;

    if user.is_active() {
        Some(user)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{HeaderMap, StatusCode};

    #[test]
    fn test_extract_bearer_token() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            "Bearer eyJhbGciOiJIUzI1NiJ9.test".parse().unwrap(),
        );

        let result = extract_jwt_token(&headers);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "eyJhbGciOiJIUzI1NiJ9.test");
    }

    #[test]
    fn test_missing_token() {
        let headers = HeaderMap::new();

        let result = extract_jwt_token(&headers);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert_eq!(err.status, StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn test_invalid_auth_scheme() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            "Basic dXNlcjpwYXNz".parse().unwrap(),
        );

        let result = extract_jwt_token(&headers);
        assert!(result.is_err());
    }

    #[test]
    fn test_trimmed_token() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            "Bearer   token-with-spaces   ".parse().unwrap(),
        );

        let result = extract_jwt_token(&headers);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "token-with-spaces");
    }
}
