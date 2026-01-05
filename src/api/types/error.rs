//! OpenAI-compatible error types

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::domain::DomainError;

/// Error types matching OpenAI API
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApiErrorType {
    InvalidRequestError,
    AuthenticationError,
    PermissionError,
    NotFoundError,
    RateLimitError,
    ServerError,
    ServiceUnavailableError,
}

impl std::fmt::Display for ApiErrorType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidRequestError => write!(f, "invalid_request_error"),
            Self::AuthenticationError => write!(f, "authentication_error"),
            Self::PermissionError => write!(f, "permission_error"),
            Self::NotFoundError => write!(f, "not_found_error"),
            Self::RateLimitError => write!(f, "rate_limit_error"),
            Self::ServerError => write!(f, "server_error"),
            Self::ServiceUnavailableError => write!(f, "service_unavailable_error"),
        }
    }
}

/// OpenAI-compatible error response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiErrorResponse {
    pub error: ApiErrorDetail,
}

/// Error detail structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiErrorDetail {
    pub message: String,
    #[serde(rename = "type")]
    pub error_type: ApiErrorType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub param: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

/// API error with status code
#[derive(Debug)]
pub struct ApiError {
    pub status: StatusCode,
    pub response: ApiErrorResponse,
}

impl ApiError {
    /// Create a new API error
    pub fn new(
        status: StatusCode,
        error_type: ApiErrorType,
        message: impl Into<String>,
    ) -> Self {
        Self {
            status,
            response: ApiErrorResponse {
                error: ApiErrorDetail {
                    message: message.into(),
                    error_type,
                    param: None,
                    code: None,
                },
            },
        }
    }

    /// Add parameter info
    pub fn with_param(mut self, param: impl Into<String>) -> Self {
        self.response.error.param = Some(param.into());
        self
    }

    /// Add error code
    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.response.error.code = Some(code.into());
        self
    }

    /// Bad request error
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, ApiErrorType::InvalidRequestError, message)
    }

    /// Authentication error
    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self::new(StatusCode::UNAUTHORIZED, ApiErrorType::AuthenticationError, message)
    }

    /// Permission error
    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::new(StatusCode::FORBIDDEN, ApiErrorType::PermissionError, message)
    }

    /// Not found error
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, ApiErrorType::NotFoundError, message)
    }

    /// Rate limit error
    pub fn rate_limited(message: impl Into<String>) -> Self {
        Self::new(StatusCode::TOO_MANY_REQUESTS, ApiErrorType::RateLimitError, message)
    }

    /// Internal server error
    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, ApiErrorType::ServerError, message)
    }

    /// Service unavailable
    pub fn unavailable(message: impl Into<String>) -> Self {
        Self::new(
            StatusCode::SERVICE_UNAVAILABLE,
            ApiErrorType::ServiceUnavailableError,
            message,
        )
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(self.response)).into_response()
    }
}

impl From<DomainError> for ApiError {
    fn from(err: DomainError) -> Self {
        match &err {
            DomainError::NotFound { message } => Self::not_found(message),
            DomainError::Validation { message } => Self::bad_request(message),
            DomainError::InvalidId { message } => {
                Self::bad_request(message).with_param("id")
            }
            DomainError::Credential { message } => Self::unauthorized(message),
            DomainError::Provider { provider, message } => {
                Self::unavailable(format!("{}: {}", provider, message))
            }
            DomainError::Configuration { message } => Self::internal(message),
            DomainError::Conflict { message } => Self::bad_request(message),
            DomainError::Internal { message } => Self::internal(message),
            DomainError::Storage { message } => Self::internal(message),
            DomainError::Cache { message } => Self::internal(message),
            DomainError::KnowledgeBase(message) => Self::internal(message),
        }
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}: {}",
            self.response.error.error_type, self.response.error.message
        )
    }
}

impl std::error::Error for ApiError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_error_creation() {
        let err = ApiError::bad_request("Invalid model");
        assert_eq!(err.status, StatusCode::BAD_REQUEST);
        assert_eq!(err.response.error.error_type, ApiErrorType::InvalidRequestError);
        assert_eq!(err.response.error.message, "Invalid model");
    }

    #[test]
    fn test_api_error_with_param() {
        let err = ApiError::bad_request("Invalid value")
            .with_param("temperature")
            .with_code("invalid_type");

        assert_eq!(err.response.error.param, Some("temperature".to_string()));
        assert_eq!(err.response.error.code, Some("invalid_type".to_string()));
    }

    #[test]
    fn test_domain_error_conversion() {
        let domain_err = DomainError::not_found("Model not found");
        let api_err: ApiError = domain_err.into();

        assert_eq!(api_err.status, StatusCode::NOT_FOUND);
        assert_eq!(api_err.response.error.error_type, ApiErrorType::NotFoundError);
    }

    #[test]
    fn test_error_serialization() {
        let err = ApiError::unauthorized("Invalid API key");
        let json = serde_json::to_string(&err.response).unwrap();

        assert!(json.contains("authentication_error"));
        assert!(json.contains("Invalid API key"));
    }

    #[test]
    fn test_all_error_types() {
        assert_eq!(ApiError::bad_request("").status, StatusCode::BAD_REQUEST);
        assert_eq!(ApiError::unauthorized("").status, StatusCode::UNAUTHORIZED);
        assert_eq!(ApiError::forbidden("").status, StatusCode::FORBIDDEN);
        assert_eq!(ApiError::not_found("").status, StatusCode::NOT_FOUND);
        assert_eq!(ApiError::rate_limited("").status, StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(ApiError::internal("").status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(ApiError::unavailable("").status, StatusCode::SERVICE_UNAVAILABLE);
    }
}
