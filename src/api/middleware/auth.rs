//! API key authentication middleware

use axum::{
    extract::FromRequestParts,
    http::{header, request::Parts},
};
use tracing::debug;

use crate::api::state::AppState;
use crate::api::types::ApiError;
use crate::domain::api_key::ApiKey;

/// Extractor that requires a valid API key
///
/// Extracts the API key from either:
/// - Authorization header: `Bearer <api_key>`
/// - X-API-Key header: `<api_key>`
#[derive(Debug, Clone)]
pub struct RequireApiKey(pub ApiKey);

impl FromRequestParts<AppState> for RequireApiKey {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let api_key_value = extract_api_key_from_headers(&parts.headers)?;

        debug!(
            key_prefix = %api_key_value.chars().take(8).collect::<String>(),
            "Validating API key"
        );

        let api_key = state
            .api_key_service
            .validate(&api_key_value)
            .await
            .map_err(|e| ApiError::internal(e.to_string()))?
            .ok_or_else(|| ApiError::unauthorized("Invalid API key"))?;

        if !api_key.is_valid() {
            return Err(ApiError::unauthorized("API key is not active or has expired"));
        }

        Ok(RequireApiKey(api_key))
    }
}

fn extract_api_key_from_headers(
    headers: &axum::http::HeaderMap,
) -> Result<String, ApiError> {
    // Try Authorization header first (Bearer token)
    if let Some(auth_header) = headers.get(header::AUTHORIZATION) {
        let auth_str = auth_header
            .to_str()
            .map_err(|_| ApiError::bad_request("Invalid Authorization header encoding"))?;

        if let Some(token) = auth_str.strip_prefix("Bearer ") {
            return Ok(token.trim().to_string());
        }
    }

    // Try X-API-Key header
    if let Some(api_key_header) = headers.get("x-api-key") {
        let key = api_key_header
            .to_str()
            .map_err(|_| ApiError::bad_request("Invalid X-API-Key header encoding"))?;

        return Ok(key.trim().to_string());
    }

    Err(ApiError::unauthorized(
        "API key required. Provide via 'Authorization: Bearer <key>' or 'X-API-Key: <key>' header",
    ))
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
            "Bearer sk-test-key-12345".parse().unwrap(),
        );

        let result = extract_api_key_from_headers(&headers);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "sk-test-key-12345");
    }

    #[test]
    fn test_extract_x_api_key() {
        let mut headers = HeaderMap::new();
        headers.insert("x-api-key", "sk-test-key-67890".parse().unwrap());

        let result = extract_api_key_from_headers(&headers);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "sk-test-key-67890");
    }

    #[test]
    fn test_bearer_takes_precedence() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            "Bearer sk-bearer-key".parse().unwrap(),
        );
        headers.insert("x-api-key", "sk-x-api-key".parse().unwrap());

        let result = extract_api_key_from_headers(&headers);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "sk-bearer-key");
    }

    #[test]
    fn test_missing_api_key() {
        let headers = HeaderMap::new();

        let result = extract_api_key_from_headers(&headers);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert_eq!(err.status, StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn test_invalid_bearer_format() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            "Basic dXNlcjpwYXNz".parse().unwrap(),
        );

        let result = extract_api_key_from_headers(&headers);
        assert!(result.is_err());
    }

    #[test]
    fn test_trimmed_token() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            "Bearer   sk-with-spaces   ".parse().unwrap(),
        );

        let result = extract_api_key_from_headers(&headers);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "sk-with-spaces");
    }
}
