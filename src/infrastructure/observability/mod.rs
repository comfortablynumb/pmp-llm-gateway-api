//! Observability infrastructure - Tracing, Metrics, and Logging

mod config;
mod metrics;
mod tracing_setup;

pub use config::{MetricsConfig, ObservabilityConfig, TracingConfig};
pub use metrics::{
    create_metrics_router, init_metrics, record_http_request, record_llm_request,
    PrometheusMetrics,
};
pub use tracing_setup::{init_tracing, shutdown_tracing};
