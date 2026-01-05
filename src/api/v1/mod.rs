//! OpenAI-compatible v1 API endpoints

pub mod chat;
pub mod models;
pub mod operations;
pub mod workflows;

use axum::{
    routing::{get, post},
    Router,
};

use super::state::AppState;

/// Create v1 API router
pub fn create_v1_router() -> Router<AppState> {
    Router::new()
        .route("/chat/completions", post(chat::create_chat_completion))
        .route("/models", get(models::list_models))
        .route("/models/{model_id}", get(models::get_model))
        .route(
            "/workflows/{workflow_id}/execute",
            post(workflows::execute_workflow),
        )
        .route("/operations", get(operations::list_operations))
        .route(
            "/operations/{operation_id}",
            get(operations::get_operation).delete(operations::cancel_operation),
        )
}
