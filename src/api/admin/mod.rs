//! Admin API endpoints for managing gateway resources

pub mod api_keys;
pub mod credentials;
pub mod models;
pub mod prompts;
pub mod workflows;

use axum::{
    routing::{delete, get, post, put},
    Router,
};

use super::state::AppState;

/// Create admin API router
pub fn create_admin_router() -> Router<AppState> {
    Router::new()
        // Model management
        .route("/models", get(models::list_models))
        .route("/models", post(models::create_model))
        .route("/models/{model_id}", get(models::get_model))
        .route("/models/{model_id}", put(models::update_model))
        .route("/models/{model_id}", delete(models::delete_model))
        // Prompt management
        .route("/prompts", get(prompts::list_prompts))
        .route("/prompts", post(prompts::create_prompt))
        .route("/prompts/{prompt_id}", get(prompts::get_prompt))
        .route("/prompts/{prompt_id}", put(prompts::update_prompt))
        .route("/prompts/{prompt_id}", delete(prompts::delete_prompt))
        .route("/prompts/{prompt_id}/render", post(prompts::render_prompt))
        // Workflow management
        .route("/workflows", get(workflows::list_workflows))
        .route("/workflows", post(workflows::create_workflow))
        .route("/workflows/{workflow_id}", get(workflows::get_workflow))
        .route("/workflows/{workflow_id}", put(workflows::update_workflow))
        .route("/workflows/{workflow_id}", delete(workflows::delete_workflow))
        // API key management
        .route("/api-keys", get(api_keys::list_api_keys))
        .route("/api-keys", post(api_keys::create_api_key))
        .route("/api-keys/{key_id}", get(api_keys::get_api_key))
        .route("/api-keys/{key_id}", put(api_keys::update_api_key))
        .route("/api-keys/{key_id}", delete(api_keys::delete_api_key))
        .route("/api-keys/{key_id}/suspend", post(api_keys::suspend_api_key))
        .route("/api-keys/{key_id}/activate", post(api_keys::activate_api_key))
        .route("/api-keys/{key_id}/revoke", post(api_keys::revoke_api_key))
        // Credential providers (read-only info)
        .route(
            "/credentials/providers",
            get(credentials::list_credential_providers),
        )
}
