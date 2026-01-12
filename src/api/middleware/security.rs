//! Security middleware for HTTP headers and request validation

use axum::{
    body::Body,
    http::{header, Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};

/// Maximum request body size (10 MB)
pub const MAX_BODY_SIZE: usize = 10 * 1024 * 1024;

/// Middleware to add security headers to all responses
pub async fn security_headers_middleware(request: Request<Body>, next: Next) -> Response {
    let is_ui_path = request.uri().path().starts_with("/ui");
    let mut response = next.run(request).await;
    let headers = response.headers_mut();

    // Prevent MIME type sniffing
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        "nosniff".parse().unwrap(),
    );

    // Prevent clickjacking
    headers.insert(
        header::X_FRAME_OPTIONS,
        "DENY".parse().unwrap(),
    );

    // Enable XSS filter (legacy, but still useful)
    headers.insert(
        "X-XSS-Protection",
        "1; mode=block".parse().unwrap(),
    );

    // Referrer policy
    headers.insert(
        header::REFERRER_POLICY,
        "strict-origin-when-cross-origin".parse().unwrap(),
    );

    // Content Security Policy - different for UI vs API
    let csp = if is_ui_path {
        // UI needs scripts and styles from CDN and self
        "default-src 'self'; \
         script-src 'self' https://cdn.tailwindcss.com https://code.jquery.com 'unsafe-inline'; \
         style-src 'self' 'unsafe-inline'; \
         connect-src 'self'; \
         img-src 'self' data:; \
         frame-ancestors 'none'"
    } else {
        // Strict CSP for API responses
        "default-src 'none'; frame-ancestors 'none'"
    };
    headers.insert(header::CONTENT_SECURITY_POLICY, csp.parse().unwrap());

    // Strict Transport Security (HSTS)
    // Only effective over HTTPS, but safe to include
    headers.insert(
        header::STRICT_TRANSPORT_SECURITY,
        "max-age=31536000; includeSubDomains".parse().unwrap(),
    );

    // Cache control for API responses (no caching by default)
    if !headers.contains_key(header::CACHE_CONTROL) {
        headers.insert(
            header::CACHE_CONTROL,
            "no-store, no-cache, must-revalidate".parse().unwrap(),
        );
    }

    response
}

/// Validate content length to prevent oversized requests
pub fn validate_content_length(content_length: Option<usize>) -> Result<(), ContentLengthError> {
    if let Some(len) = content_length {
        if len > MAX_BODY_SIZE {
            return Err(ContentLengthError::TooLarge(len));
        }
    }
    Ok(())
}

/// Error for content length validation
#[derive(Debug)]
pub enum ContentLengthError {
    TooLarge(usize),
}

impl IntoResponse for ContentLengthError {
    fn into_response(self) -> Response {
        match self {
            ContentLengthError::TooLarge(size) => {
                let body = format!(
                    r#"{{"error":{{"message":"Request body too large: {} bytes (max: {} bytes)","type":"invalid_request_error"}}}}"#,
                    size, MAX_BODY_SIZE
                );
                (
                    StatusCode::PAYLOAD_TOO_LARGE,
                    [(header::CONTENT_TYPE, "application/json")],
                    body,
                )
                    .into_response()
            }
        }
    }
}

/// Validate request for common security issues
pub fn validate_request_security(path: &str, _method: &str) -> Result<(), SecurityValidationError> {
    // Check for path traversal attempts
    if path.contains("..") || path.contains("//") {
        return Err(SecurityValidationError::PathTraversal);
    }

    // Check for null bytes (potential injection)
    if path.contains('\0') {
        return Err(SecurityValidationError::InvalidCharacters);
    }

    Ok(())
}

/// Security validation error
#[derive(Debug)]
pub enum SecurityValidationError {
    PathTraversal,
    InvalidCharacters,
}

impl IntoResponse for SecurityValidationError {
    fn into_response(self) -> Response {
        let (message, error_type) = match self {
            SecurityValidationError::PathTraversal => {
                ("Invalid path: path traversal detected", "invalid_request_error")
            }
            SecurityValidationError::InvalidCharacters => {
                ("Invalid request: prohibited characters", "invalid_request_error")
            }
        };

        let body = format!(
            r#"{{"error":{{"message":"{}","type":"{}"}}}}"#,
            message, error_type
        );

        (
            StatusCode::BAD_REQUEST,
            [(header::CONTENT_TYPE, "application/json")],
            body,
        )
            .into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_content_length_ok() {
        assert!(validate_content_length(Some(1000)).is_ok());
        assert!(validate_content_length(Some(MAX_BODY_SIZE)).is_ok());
        assert!(validate_content_length(None).is_ok());
    }

    #[test]
    fn test_validate_content_length_too_large() {
        let result = validate_content_length(Some(MAX_BODY_SIZE + 1));
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_request_security_ok() {
        assert!(validate_request_security("/v1/models", "GET").is_ok());
        assert!(validate_request_security("/admin/api-keys/123", "DELETE").is_ok());
    }

    #[test]
    fn test_validate_request_security_path_traversal() {
        let result = validate_request_security("/v1/../admin/secrets", "GET");
        assert!(matches!(result, Err(SecurityValidationError::PathTraversal)));
    }

    #[test]
    fn test_validate_request_security_double_slash() {
        let result = validate_request_security("/v1//models", "GET");
        assert!(matches!(result, Err(SecurityValidationError::PathTraversal)));
    }

    #[test]
    fn test_validate_request_security_null_byte() {
        let result = validate_request_security("/v1/models\0.json", "GET");
        assert!(matches!(result, Err(SecurityValidationError::InvalidCharacters)));
    }

    #[test]
    fn test_max_body_size() {
        // 10 MB
        assert_eq!(MAX_BODY_SIZE, 10 * 1024 * 1024);
    }
}
