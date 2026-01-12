//! Observability configuration

use serde::Deserialize;

/// Main observability configuration
#[derive(Debug, Clone, Deserialize, Default)]
pub struct ObservabilityConfig {
    #[serde(default)]
    pub tracing: TracingConfig,
    #[serde(default)]
    pub metrics: MetricsConfig,
}

/// Distributed tracing configuration
#[derive(Debug, Clone, Deserialize)]
pub struct TracingConfig {
    /// Enable OpenTelemetry tracing export
    #[serde(default)]
    pub enabled: bool,
    /// OTLP endpoint (e.g., http://localhost:4317)
    #[serde(default = "default_otlp_endpoint")]
    pub otlp_endpoint: String,
    /// Service name for tracing
    #[serde(default = "default_service_name")]
    pub service_name: String,
    /// Sampling ratio (0.0 to 1.0)
    #[serde(default = "default_sampling_ratio")]
    pub sampling_ratio: f64,
}

/// Prometheus metrics configuration
#[derive(Debug, Clone, Deserialize)]
pub struct MetricsConfig {
    /// Enable Prometheus metrics
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Metrics endpoint path
    #[serde(default = "default_metrics_path")]
    pub path: String,
    /// Include default process metrics
    #[serde(default = "default_true")]
    pub include_process_metrics: bool,
}

fn default_otlp_endpoint() -> String {
    "http://localhost:4317".to_string()
}

fn default_service_name() -> String {
    "pmp-llm-gateway".to_string()
}

fn default_sampling_ratio() -> f64 {
    1.0
}

fn default_true() -> bool {
    true
}

fn default_metrics_path() -> String {
    "/metrics".to_string()
}

impl Default for TracingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            otlp_endpoint: default_otlp_endpoint(),
            service_name: default_service_name(),
            sampling_ratio: default_sampling_ratio(),
        }
    }
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            path: default_metrics_path(),
            include_process_metrics: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_observability_config() {
        let config = ObservabilityConfig::default();

        assert!(!config.tracing.enabled);
        assert_eq!(config.tracing.otlp_endpoint, "http://localhost:4317");
        assert_eq!(config.tracing.service_name, "pmp-llm-gateway");
        assert_eq!(config.tracing.sampling_ratio, 1.0);

        assert!(config.metrics.enabled);
        assert_eq!(config.metrics.path, "/metrics");
        assert!(config.metrics.include_process_metrics);
    }

    #[test]
    fn test_tracing_config_defaults() {
        let config = TracingConfig::default();

        assert!(!config.enabled);
        assert_eq!(config.sampling_ratio, 1.0);
    }

    #[test]
    fn test_metrics_config_defaults() {
        let config = MetricsConfig::default();

        assert!(config.enabled);
        assert_eq!(config.path, "/metrics");
    }
}
