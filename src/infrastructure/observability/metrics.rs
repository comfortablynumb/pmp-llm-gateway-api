//! Prometheus metrics infrastructure

use std::sync::Arc;
use std::time::Duration;

use axum::{extract::State, response::IntoResponse, routing::get, Router};
use metrics::{counter, gauge, histogram};
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};

use super::config::MetricsConfig;

/// Prometheus metrics handle for serving metrics endpoint
#[derive(Clone)]
pub struct PrometheusMetrics {
    handle: Arc<PrometheusHandle>,
}

impl PrometheusMetrics {
    /// Get the metrics as a string for the /metrics endpoint
    pub fn render(&self) -> String {
        self.handle.render()
    }
}

/// Initialize Prometheus metrics
pub fn init_metrics(config: &MetricsConfig) -> Option<PrometheusMetrics> {
    if !config.enabled {
        tracing::info!("Prometheus metrics disabled");
        return None;
    }

    let builder = PrometheusBuilder::new();

    match builder.install_recorder() {
        Ok(handle) => {
            register_default_metrics();

            tracing::info!("Prometheus metrics initialized at {}", config.path);

            Some(PrometheusMetrics {
                handle: Arc::new(handle),
            })
        }
        Err(e) => {
            tracing::error!("Failed to initialize Prometheus metrics: {}", e);
            None
        }
    }
}

fn register_default_metrics() {
    // Register default metrics with initial values
    gauge!("llm_gateway_info", "version" => env!("CARGO_PKG_VERSION")).set(1.0);
}

/// Create the metrics router
pub fn create_metrics_router(metrics: PrometheusMetrics) -> Router {
    Router::new()
        .route("/metrics", get(metrics_handler))
        .with_state(metrics)
}

async fn metrics_handler(State(metrics): State<PrometheusMetrics>) -> impl IntoResponse {
    metrics.render()
}

/// Record an HTTP request metric
pub fn record_http_request(method: &str, path: &str, status: u16, duration: Duration) {
    let status_str = status.to_string();
    let labels = [
        ("method", method.to_string()),
        ("path", sanitize_path(path)),
        ("status", status_str),
    ];

    counter!("http_requests_total", &labels).increment(1);
    histogram!("http_request_duration_seconds", &labels).record(duration.as_secs_f64());

    // Track 5xx errors separately
    if status >= 500 {
        counter!("http_server_errors_total", &labels).increment(1);
    }
}

/// Record an LLM request metric
pub fn record_llm_request(params: LlmRequestMetricParams) {
    let labels = [
        ("provider", params.provider.to_string()),
        ("model", params.model.to_string()),
        ("status", if params.success { "success" } else { "error" }.to_string()),
    ];

    counter!("llm_requests_total", &labels).increment(1);
    histogram!("llm_request_duration_seconds", &labels).record(params.duration.as_secs_f64());

    if let Some(tokens) = params.input_tokens {
        counter!("llm_input_tokens_total", &labels).increment(tokens);
    }

    if let Some(tokens) = params.output_tokens {
        counter!("llm_output_tokens_total", &labels).increment(tokens);
    }

    if !params.success {
        counter!("llm_errors_total", &labels).increment(1);
    }
}

/// Parameters for LLM request metrics
pub struct LlmRequestMetricParams<'a> {
    pub provider: &'a str,
    pub model: &'a str,
    pub duration: Duration,
    pub success: bool,
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
}

/// Sanitize URL path for metric labels (remove IDs, limit cardinality)
fn sanitize_path(path: &str) -> String {
    // Replace UUIDs and numeric IDs with placeholders
    let path = regex::Regex::new(r"[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}")
        .unwrap()
        .replace_all(path, "{id}");

    let path = regex::Regex::new(r"/\d+(/|$)")
        .unwrap()
        .replace_all(&path, "/{id}$1");

    // Truncate long paths
    if path.len() > 50 {
        path[..50].to_string()
    } else {
        path.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_path_uuid() {
        let path = "/v1/models/550e8400-e29b-41d4-a716-446655440000";
        let sanitized = sanitize_path(path);
        assert_eq!(sanitized, "/v1/models/{id}");
    }

    #[test]
    fn test_sanitize_path_numeric_id() {
        let path = "/api/users/123/orders";
        let sanitized = sanitize_path(path);
        assert_eq!(sanitized, "/api/users/{id}/orders");
    }

    #[test]
    fn test_sanitize_path_no_id() {
        let path = "/health";
        let sanitized = sanitize_path(path);
        assert_eq!(sanitized, "/health");
    }

    #[test]
    fn test_sanitize_path_truncates_long_paths() {
        let path = "/very/long/path/that/exceeds/the/maximum/allowed/length/for/metrics";
        let sanitized = sanitize_path(path);
        assert!(sanitized.len() <= 50);
    }

    #[test]
    fn test_llm_request_metric_params() {
        let params = LlmRequestMetricParams {
            provider: "openai",
            model: "gpt-4",
            duration: Duration::from_millis(500),
            success: true,
            input_tokens: Some(100),
            output_tokens: Some(50),
        };

        assert_eq!(params.provider, "openai");
        assert_eq!(params.model, "gpt-4");
        assert!(params.success);
    }
}
