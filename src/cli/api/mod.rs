//! API command - runs API server only (no UI)

use std::net::SocketAddr;

use axum::middleware;
use axum::routing::get;
use axum::Router;
use tokio::net::TcpListener;
use tokio::signal;
use tracing::info;

use crate::api::middleware::{logging_middleware, metrics_middleware, security_headers_middleware};
use crate::api::state::AppState;
use crate::api::{admin, auth, health, v1};
use crate::config::AppConfig;
use crate::infrastructure::logging;
use crate::infrastructure::observability::{
    create_metrics_router, init_metrics, init_tracing, shutdown_tracing, PrometheusMetrics,
};

/// Run the API-only server
pub async fn run() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let config = AppConfig::load().unwrap_or_default();
    init_observability(&config);

    let state = crate::create_app_state_with_config(&config).await?;
    let metrics = init_metrics(&config.observability.metrics);
    let app = create_api_router(state, metrics);

    let addr = build_socket_addr(&config)?;
    info!("Starting API server on {}", addr);

    let listener = TcpListener::bind(addr).await?;

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    shutdown_tracing();
    info!("API server shutdown complete");

    Ok(())
}

fn init_observability(config: &AppConfig) {
    init_tracing(
        &logging::LoggingConfig {
            level: config.logging.level.clone(),
            format: config.logging.format.clone(),
        },
        &config.observability.tracing,
    );
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("Received Ctrl+C, initiating graceful shutdown");
        }
        _ = terminate => {
            info!("Received SIGTERM, initiating graceful shutdown");
        }
    }
}

fn build_socket_addr(config: &AppConfig) -> anyhow::Result<SocketAddr> {
    Ok(SocketAddr::from((
        config.server.host.parse::<std::net::IpAddr>()?,
        config.server.port,
    )))
}

/// Create API router (no UI)
fn create_api_router(state: AppState, metrics: Option<PrometheusMetrics>) -> Router {
    let mut router = Router::new()
        // Health endpoints
        .route("/health", get(health::health_check))
        .route("/ready", get(health::ready_check))
        .route("/live", get(health::live_check))
        // Authentication endpoints
        .nest("/auth", auth::create_auth_router())
        // OpenAI-compatible v1 API
        .nest("/v1", v1::create_v1_router())
        // Admin API
        .nest("/admin", admin::create_admin_router())
        // Add state and middleware
        .with_state(state)
        .layer(middleware::from_fn(security_headers_middleware))
        .layer(middleware::from_fn(logging_middleware))
        .layer(middleware::from_fn(metrics_middleware))
        .layer(tower_http::trace::TraceLayer::new_for_http());

    // Add metrics endpoint if enabled
    if let Some(m) = metrics {
        router = router.merge(create_metrics_router(m));
    }

    router
}
