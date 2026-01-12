//! Request/response logging middleware with sensitive data redaction

use std::time::Instant;

use axum::{
    body::Body,
    extract::MatchedPath,
    http::Request,
    middleware::Next,
    response::Response,
};
use tracing::info;

/// Middleware to log HTTP requests and responses with sensitive data redaction.
/// Note: This middleware does NOT create its own tracing span since `TraceLayer`
/// from tower-http already handles span creation. Creating duplicate spans
/// causes panics in the tracing registry.
pub async fn logging_middleware(request: Request<Body>, next: Next) -> Response {
    let start = Instant::now();
    let method = request.method().clone();
    let uri = request.uri().clone();
    let path = extract_path(&request);
    let request_id = extract_request_id(&request);

    // Extract and redact headers for logging
    let headers_log = redact_headers(&request);

    info!(
        method = %method,
        path = %path,
        uri = %uri,
        request_id = %request_id,
        headers = %headers_log,
        "Incoming request"
    );

    let response = next.run(request).await;

    let duration = start.elapsed();
    let status = response.status();

    info!(
        method = %method,
        path = %path,
        status = %status.as_u16(),
        duration_ms = %duration.as_millis(),
        request_id = %request_id,
        "Request completed"
    );

    response
}

fn extract_path(request: &Request<Body>) -> String {
    request
        .extensions()
        .get::<MatchedPath>()
        .map(|mp| mp.as_str().to_string())
        .unwrap_or_else(|| request.uri().path().to_string())
}

fn extract_request_id(request: &Request<Body>) -> String {
    request
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string())
}

/// Redact sensitive headers for logging
fn redact_headers(request: &Request<Body>) -> String {
    let mut parts = Vec::new();

    for (name, value) in request.headers() {
        let name_str = name.as_str().to_lowercase();
        let value_str = if is_sensitive_header(&name_str) {
            "[REDACTED]".to_string()
        } else {
            value.to_str().unwrap_or("[invalid]").to_string()
        };

        // Only log relevant headers
        if should_log_header(&name_str) {
            parts.push(format!("{}={}", name_str, value_str));
        }
    }

    parts.join(", ")
}

/// Check if a header contains sensitive information
fn is_sensitive_header(name: &str) -> bool {
    matches!(
        name,
        "authorization"
            | "x-api-key"
            | "cookie"
            | "set-cookie"
            | "x-auth-token"
            | "x-csrf-token"
            | "x-xsrf-token"
            | "proxy-authorization"
    )
}

/// Check if a header should be logged
fn should_log_header(name: &str) -> bool {
    matches!(
        name,
        "content-type"
            | "content-length"
            | "accept"
            | "user-agent"
            | "x-request-id"
            | "x-forwarded-for"
            | "x-real-ip"
            | "authorization"
            | "x-api-key"
    )
}

/// Redact sensitive values in a JSON string
pub fn redact_json_sensitive_fields(json: &str) -> String {
    let sensitive_fields = [
        "password",
        "api_key",
        "apiKey",
        "secret",
        "token",
        "access_token",
        "refresh_token",
        "credentials",
        "authorization",
    ];

    let mut result = json.to_string();

    for field in &sensitive_fields {
        // Redact string values: "field": "value" -> "field": "[REDACTED]"
        let pattern = format!(r#""{}"\s*:\s*"[^"]*""#, field);

        if let Ok(re) = regex::Regex::new(&pattern) {
            result = re
                .replace_all(&result, format!(r#""{}":\s*"[REDACTED]""#, field).as_str())
                .to_string();
        }
    }

    result
}

/// Truncate long strings for logging
pub fn truncate_for_log(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...[truncated {} chars]", &s[..max_len], s.len() - max_len)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_sensitive_header() {
        assert!(is_sensitive_header("authorization"));
        assert!(is_sensitive_header("x-api-key"));
        assert!(is_sensitive_header("cookie"));
        assert!(!is_sensitive_header("content-type"));
        assert!(!is_sensitive_header("accept"));
    }

    #[test]
    fn test_should_log_header() {
        assert!(should_log_header("content-type"));
        assert!(should_log_header("authorization"));
        assert!(should_log_header("user-agent"));
        assert!(!should_log_header("cache-control"));
        assert!(!should_log_header("etag"));
    }

    #[test]
    fn test_redact_json_sensitive_fields() {
        let input = r#"{"username": "test", "password": "secret123"}"#;
        let result = redact_json_sensitive_fields(input);
        assert!(result.contains("[REDACTED]"));
        assert!(!result.contains("secret123"));
    }

    #[test]
    fn test_truncate_for_log() {
        let short = "hello";
        assert_eq!(truncate_for_log(short, 10), "hello");

        let long = "hello world this is a very long string";
        let truncated = truncate_for_log(long, 10);
        assert!(truncated.starts_with("hello worl"));
        assert!(truncated.contains("[truncated"));
    }

    #[test]
    fn test_truncate_for_log_exact_length() {
        let s = "exactly10!";
        assert_eq!(truncate_for_log(s, 10), "exactly10!");
    }
}
