//! API command - runs API server only (no UI)

use std::net::SocketAddr;

use tokio::net::TcpListener;
use tracing::info;

use crate::api::create_router_with_state;
use crate::config::AppConfig;
use crate::infrastructure::logging;

/// Run the API-only server
pub async fn run() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let config = AppConfig::load().unwrap_or_default();
    init_logging(&config);

    let state = crate::create_app_state().await?;
    let app = create_router_with_state(state);

    let addr = build_socket_addr(&config)?;
    info!("Starting API server on {}", addr);

    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

fn init_logging(config: &AppConfig) {
    logging::init_logging(&logging::LoggingConfig {
        level: config.logging.level.clone(),
        format: config.logging.format.clone(),
    });
}

fn build_socket_addr(config: &AppConfig) -> anyhow::Result<SocketAddr> {
    Ok(SocketAddr::from((
        config.server.host.parse::<std::net::IpAddr>()?,
        config.server.port,
    )))
}
