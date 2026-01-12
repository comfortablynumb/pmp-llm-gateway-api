//! HTTP metrics middleware for recording request/response metrics

use std::time::Instant;

use axum::{
    body::Body,
    extract::MatchedPath,
    http::Request,
    middleware::Next,
    response::Response,
};

use crate::infrastructure::observability::record_http_request;

/// Middleware to record HTTP request metrics
pub async fn metrics_middleware(request: Request<Body>, next: Next) -> Response {
    let start = Instant::now();
    let method = request.method().clone();
    let path = extract_path(&request);

    let response = next.run(request).await;

    let duration = start.elapsed();
    let status = response.status().as_u16();

    record_http_request(method.as_str(), &path, status, duration);

    response
}

fn extract_path(request: &Request<Body>) -> String {
    // Try to get the matched path pattern first (for consistent cardinality)
    request
        .extensions()
        .get::<MatchedPath>()
        .map(|mp| mp.as_str().to_string())
        .unwrap_or_else(|| request.uri().path().to_string())
}

#[cfg(test)]
mod tests {
    use axum::http::Method;

    #[test]
    fn test_method_str() {
        assert_eq!(Method::GET.as_str(), "GET");
        assert_eq!(Method::POST.as_str(), "POST");
    }
}
