//! OpenTelemetry distributed tracing setup

use opentelemetry::{trace::TracerProvider as _, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{
    runtime,
    trace::{RandomIdGenerator, Sampler, TracerProvider},
    Resource,
};
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
};

use super::config::TracingConfig;
use crate::config::LogFormat;
use crate::infrastructure::logging::LoggingConfig;

/// Initialize tracing with optional OpenTelemetry export
pub fn init_tracing(logging_config: &LoggingConfig, tracing_config: &TracingConfig) {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&logging_config.level));

    match &logging_config.format {
        LogFormat::Json => init_json_tracing(filter, tracing_config),
        LogFormat::Pretty => init_pretty_tracing(filter, tracing_config),
    }
}

fn init_json_tracing(filter: EnvFilter, tracing_config: &TracingConfig) {
    let fmt_layer = fmt::layer()
        .json()
        .with_span_events(FmtSpan::CLOSE)
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true);

    if tracing_config.enabled {
        match init_otel_tracing(tracing_config) {
            Ok(tracer_provider) => {
                let tracer = tracer_provider.tracer("pmp-llm-gateway");
                let telemetry_layer = tracing_opentelemetry::layer().with_tracer(tracer);

                tracing_subscriber::registry()
                    .with(filter)
                    .with(fmt_layer)
                    .with(telemetry_layer)
                    .init();

                tracing::info!(
                    "Tracing initialized with OpenTelemetry export to {}",
                    tracing_config.otlp_endpoint
                );
            }
            Err(e) => {
                tracing_subscriber::registry()
                    .with(filter)
                    .with(fmt_layer)
                    .init();

                tracing::warn!(
                    "Failed to initialize OpenTelemetry: {}. Tracing disabled.",
                    e
                );
            }
        }
    } else {
        tracing_subscriber::registry()
            .with(filter)
            .with(fmt_layer)
            .init();

        tracing::info!("Tracing initialized (OpenTelemetry disabled)");
    }
}

fn init_pretty_tracing(filter: EnvFilter, tracing_config: &TracingConfig) {
    let fmt_layer = fmt::layer()
        .pretty()
        .with_target(true)
        .with_span_events(FmtSpan::CLOSE);

    if tracing_config.enabled {
        match init_otel_tracing(tracing_config) {
            Ok(tracer_provider) => {
                let tracer = tracer_provider.tracer("pmp-llm-gateway");
                let telemetry_layer = tracing_opentelemetry::layer().with_tracer(tracer);

                tracing_subscriber::registry()
                    .with(filter)
                    .with(fmt_layer)
                    .with(telemetry_layer)
                    .init();

                tracing::info!(
                    "Tracing initialized with OpenTelemetry export to {}",
                    tracing_config.otlp_endpoint
                );
            }
            Err(e) => {
                tracing_subscriber::registry()
                    .with(filter)
                    .with(fmt_layer)
                    .init();

                tracing::warn!(
                    "Failed to initialize OpenTelemetry: {}. Tracing disabled.",
                    e
                );
            }
        }
    } else {
        tracing_subscriber::registry()
            .with(filter)
            .with(fmt_layer)
            .init();

        tracing::info!("Tracing initialized (OpenTelemetry disabled)");
    }
}

fn init_otel_tracing(
    config: &TracingConfig,
) -> Result<TracerProvider, opentelemetry::trace::TraceError> {
    let resource = Resource::new(vec![KeyValue::new(
        "service.name",
        config.service_name.clone(),
    )]);

    let sampler = if config.sampling_ratio >= 1.0 {
        Sampler::AlwaysOn
    } else if config.sampling_ratio <= 0.0 {
        Sampler::AlwaysOff
    } else {
        Sampler::TraceIdRatioBased(config.sampling_ratio)
    };

    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(&config.otlp_endpoint)
        .build()?;

    let provider = TracerProvider::builder()
        .with_sampler(sampler)
        .with_id_generator(RandomIdGenerator::default())
        .with_resource(resource)
        .with_batch_exporter(exporter, runtime::Tokio)
        .build();

    Ok(provider)
}

/// Shutdown tracing and flush pending spans
pub fn shutdown_tracing() {
    opentelemetry::global::shutdown_tracer_provider();
    tracing::info!("Tracing shutdown complete");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tracing_config_default() {
        let config = TracingConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.sampling_ratio, 1.0);
    }
}
