use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
};

use crate::config::LogFormat;

pub struct LoggingConfig {
    pub level: String,
    pub format: LogFormat,
}

pub fn init_logging(config: &LoggingConfig) {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&config.level));

    match config.format {
        LogFormat::Json => {
            tracing_subscriber::registry()
                .with(filter)
                .with(fmt::layer().json().with_span_events(FmtSpan::CLOSE))
                .init();
        }
        LogFormat::Pretty => {
            tracing_subscriber::registry()
                .with(filter)
                .with(
                    fmt::layer()
                        .pretty()
                        .with_target(true)
                        .with_span_events(FmtSpan::CLOSE),
                )
                .init();
        }
    }

    tracing::info!("Logging initialized with level: {}", config.level);
}
