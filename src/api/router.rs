use axum::{routing::get, Router};
use tower_http::trace::TraceLayer;

use super::admin;
use super::auth;
use super::health;
use super::state::AppState;
use super::v1;

/// Create a minimal router without state (for testing/backward compatibility)
/// Note: /ready endpoint is not available without state
pub fn create_router() -> Router {
    Router::new()
        .route("/health", get(health::health_check))
        .route("/live", get(health::live_check))
        .layer(TraceLayer::new_for_http())
}

/// Create the full router with application state
pub fn create_router_with_state(state: AppState) -> Router {
    Router::new()
        // Health endpoints (no state needed)
        .route("/health", get(health::health_check))
        .route("/ready", get(health::ready_check))
        .route("/live", get(health::live_check))
        // Authentication endpoints (no auth required for login)
        .nest("/auth", auth::create_auth_router())
        // OpenAI-compatible v1 API
        .nest("/v1", v1::create_v1_router())
        // Admin API
        .nest("/admin", admin::create_admin_router())
        // Add state and middleware
        .with_state(state)
        .layer(TraceLayer::new_for_http())
}
