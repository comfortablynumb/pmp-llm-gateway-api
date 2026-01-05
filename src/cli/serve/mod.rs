//! Serve command - runs API + UI combined on the same port

use std::net::SocketAddr;

use axum::response::Redirect;
use axum::routing::get;
use axum::Router;
use tokio::net::TcpListener;
use tower_http::services::{ServeDir, ServeFile};
use tracing::info;

use crate::api::state::AppState;
use crate::api::{admin, health, v1};
use crate::config::AppConfig;
use crate::infrastructure::logging;

/// Run the combined API + UI server
pub async fn run() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let config = AppConfig::load().unwrap_or_default();
    init_logging(&config);

    let state = crate::create_app_state().await?;
    let app = create_router_with_ui(state);

    let addr = build_socket_addr(&config)?;
    info!("Starting server (API + UI) on {}", addr);

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

/// Create router with both API and UI endpoints
fn create_router_with_ui(state: AppState) -> Router {
    Router::new()
        // Health endpoints
        .route("/health", get(health::health_check))
        .route("/ready", get(health::ready_check))
        .route("/live", get(health::live_check))
        // OpenAI-compatible v1 API
        .nest("/v1", v1::create_v1_router())
        // Admin API (also exposed at /api for UI consumption)
        .nest("/admin", admin::create_admin_router())
        .nest("/api", admin::create_admin_router())
        // UI static files
        .nest_service(
            "/ui",
            ServeDir::new("public").fallback(ServeFile::new("public/index.html")),
        )
        // Redirect root to UI
        .route("/", get(|| async { Redirect::permanent("/ui/") }))
        // Add state and middleware
        .with_state(state)
        .layer(tower_http::trace::TraceLayer::new_for_http())
}
