//! Admin API endpoints for managing gateway resources

pub mod api_keys;
pub mod config;
pub mod credentials;
pub mod execution_logs;
pub mod experiments;
pub mod external_apis;
pub mod knowledge_bases;
pub mod models;
pub mod prompts;
pub mod teams;
pub mod test_cases;
pub mod usage;
pub mod webhooks;
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
        .route("/models/{model_id}/execute", post(models::execute_model))
        // Prompt management
        .route("/prompts", get(prompts::list_prompts))
        .route("/prompts", post(prompts::create_prompt))
        .route("/prompts/{prompt_id}", get(prompts::get_prompt))
        .route("/prompts/{prompt_id}", put(prompts::update_prompt))
        .route("/prompts/{prompt_id}", delete(prompts::delete_prompt))
        .route("/prompts/{prompt_id}/render", post(prompts::render_prompt))
        .route(
            "/prompts/{prompt_id}/versions",
            get(prompts::list_versions),
        )
        .route(
            "/prompts/{prompt_id}/revert/{version}",
            post(prompts::revert_to_version),
        )
        // Workflow management
        .route("/workflows", get(workflows::list_workflows))
        .route("/workflows", post(workflows::create_workflow))
        .route("/workflows/{workflow_id}", get(workflows::get_workflow))
        .route("/workflows/{workflow_id}", put(workflows::update_workflow))
        .route("/workflows/{workflow_id}", delete(workflows::delete_workflow))
        .route(
            "/workflows/{workflow_id}/test",
            post(workflows::test_workflow),
        )
        .route(
            "/workflows/{workflow_id}/execute",
            post(workflows::execute_workflow),
        )
        .route(
            "/workflows/{workflow_id}/clone",
            post(workflows::clone_workflow),
        )
        // API key management
        .route("/api-keys", get(api_keys::list_api_keys))
        .route("/api-keys", post(api_keys::create_api_key))
        .route("/api-keys/{key_id}", get(api_keys::get_api_key))
        .route("/api-keys/{key_id}", put(api_keys::update_api_key))
        .route("/api-keys/{key_id}", delete(api_keys::delete_api_key))
        .route("/api-keys/{key_id}/suspend", post(api_keys::suspend_api_key))
        .route("/api-keys/{key_id}/activate", post(api_keys::activate_api_key))
        .route("/api-keys/{key_id}/revoke", post(api_keys::revoke_api_key))
        // Team management
        .route("/teams", get(teams::list_teams))
        .route("/teams", post(teams::create_team))
        .route("/teams/{team_id}", get(teams::get_team))
        .route("/teams/{team_id}", put(teams::update_team))
        .route("/teams/{team_id}", delete(teams::delete_team))
        .route("/teams/{team_id}/suspend", post(teams::suspend_team))
        .route("/teams/{team_id}/activate", post(teams::activate_team))
        // Credential management
        .route("/credentials", get(credentials::list_credentials))
        .route("/credentials", post(credentials::create_credential))
        .route(
            "/credentials/providers",
            get(credentials::list_credential_providers),
        )
        .route(
            "/credentials/{credential_id}",
            get(credentials::get_credential),
        )
        .route(
            "/credentials/{credential_id}",
            put(credentials::update_credential),
        )
        .route(
            "/credentials/{credential_id}",
            delete(credentials::delete_credential),
        )
        .route(
            "/credentials/{credential_id}/test",
            post(credentials::test_credential),
        )
        // External API management
        .route("/external-apis", get(external_apis::list_external_apis))
        .route("/external-apis", post(external_apis::create_external_api))
        .route(
            "/external-apis/{api_id}",
            get(external_apis::get_external_api),
        )
        .route(
            "/external-apis/{api_id}",
            put(external_apis::update_external_api),
        )
        .route(
            "/external-apis/{api_id}",
            delete(external_apis::delete_external_api),
        )
        // Knowledge Base management
        .route(
            "/knowledge-bases",
            get(knowledge_bases::list_knowledge_bases),
        )
        .route(
            "/knowledge-bases",
            post(knowledge_bases::create_knowledge_base),
        )
        .route(
            "/knowledge-bases/types",
            get(knowledge_bases::list_knowledge_base_types),
        )
        .route(
            "/knowledge-bases/{kb_id}",
            get(knowledge_bases::get_knowledge_base),
        )
        .route(
            "/knowledge-bases/{kb_id}",
            put(knowledge_bases::update_knowledge_base),
        )
        .route(
            "/knowledge-bases/{kb_id}",
            delete(knowledge_bases::delete_knowledge_base),
        )
        // Knowledge Base documents
        .route(
            "/knowledge-bases/{kb_id}/documents",
            get(knowledge_bases::list_documents),
        )
        .route(
            "/knowledge-bases/{kb_id}/documents",
            post(knowledge_bases::ingest_document),
        )
        .route(
            "/knowledge-bases/{kb_id}/documents/upload",
            post(knowledge_bases::ingest_files_batch),
        )
        .route(
            "/knowledge-bases/{kb_id}/documents/{document_id}",
            get(knowledge_bases::get_document),
        )
        .route(
            "/knowledge-bases/{kb_id}/documents/{document_id}",
            delete(knowledge_bases::delete_document),
        )
        .route(
            "/knowledge-bases/{kb_id}/documents/{document_id}/chunks",
            get(knowledge_bases::get_document_chunks),
        )
        .route(
            "/knowledge-bases/{kb_id}/documents/{document_id}/disable",
            post(knowledge_bases::disable_document),
        )
        .route(
            "/knowledge-bases/{kb_id}/documents/{document_id}/enable",
            post(knowledge_bases::enable_document),
        )
        .route(
            "/knowledge-bases/{kb_id}/ingestions",
            get(knowledge_bases::list_ingestion_operations),
        )
        .route(
            "/knowledge-bases/{kb_id}/schema",
            post(knowledge_bases::ensure_schema),
        )
        // Usage tracking
        .route("/usage", get(usage::list_usage))
        .route("/usage", delete(usage::delete_usage))
        .route("/usage/aggregate", get(usage::get_usage_aggregate))
        .route("/usage/summary", get(usage::get_usage_summary))
        // Budget management
        .route("/budgets", get(usage::list_budgets))
        .route("/budgets", post(usage::create_budget))
        .route("/budgets/check", post(usage::check_budget))
        .route("/budgets/by-team/{team_id}", get(usage::list_budgets_by_team))
        .route("/budgets/{budget_id}", get(usage::get_budget))
        .route("/budgets/{budget_id}", put(usage::update_budget))
        .route("/budgets/{budget_id}", delete(usage::delete_budget))
        .route("/budgets/{budget_id}/reset", post(usage::reset_budget))
        // Experiment (A/B Testing) management
        .route("/experiments", get(experiments::list_experiments))
        .route("/experiments", post(experiments::create_experiment))
        .route(
            "/experiments/{experiment_id}",
            get(experiments::get_experiment),
        )
        .route(
            "/experiments/{experiment_id}",
            put(experiments::update_experiment),
        )
        .route(
            "/experiments/{experiment_id}",
            delete(experiments::delete_experiment),
        )
        .route(
            "/experiments/{experiment_id}/variants",
            post(experiments::add_variant),
        )
        .route(
            "/experiments/{experiment_id}/variants/{variant_id}",
            delete(experiments::remove_variant),
        )
        .route(
            "/experiments/{experiment_id}/start",
            post(experiments::start_experiment),
        )
        .route(
            "/experiments/{experiment_id}/pause",
            post(experiments::pause_experiment),
        )
        .route(
            "/experiments/{experiment_id}/resume",
            post(experiments::resume_experiment),
        )
        .route(
            "/experiments/{experiment_id}/complete",
            post(experiments::complete_experiment),
        )
        .route(
            "/experiments/{experiment_id}/results",
            get(experiments::get_experiment_results),
        )
        // Test case management
        .route("/test-cases", get(test_cases::list_test_cases))
        .route("/test-cases", post(test_cases::create_test_case))
        .route("/test-cases/{test_case_id}", get(test_cases::get_test_case))
        .route(
            "/test-cases/{test_case_id}",
            put(test_cases::update_test_case),
        )
        .route(
            "/test-cases/{test_case_id}",
            delete(test_cases::delete_test_case),
        )
        .route(
            "/test-cases/{test_case_id}/execute",
            post(test_cases::execute_test_case),
        )
        .route(
            "/test-cases/{test_case_id}/results",
            get(test_cases::get_test_case_results),
        )
        // Configuration management
        .route("/config", get(config::list_config))
        .route("/config", delete(config::reset_config))
        .route("/config/category/{category}", get(config::list_config_by_category))
        .route("/config/{key}", get(config::get_config))
        .route("/config/{key}", put(config::update_config))
        // Execution log management
        .route("/execution-logs", get(execution_logs::list_execution_logs))
        .route("/execution-logs/stats", get(execution_logs::get_execution_stats))
        .route("/execution-logs/cleanup", post(execution_logs::cleanup_execution_logs))
        .route("/execution-logs/{log_id}", get(execution_logs::get_execution_log))
        .route("/execution-logs/{log_id}", delete(execution_logs::delete_execution_log))
        // Webhook management
        .route("/webhooks", get(webhooks::list_webhooks))
        .route("/webhooks", post(webhooks::create_webhook))
        .route("/webhooks/event-types", get(webhooks::list_event_types))
        .route("/webhooks/{webhook_id}", get(webhooks::get_webhook))
        .route("/webhooks/{webhook_id}", put(webhooks::update_webhook))
        .route("/webhooks/{webhook_id}", delete(webhooks::delete_webhook))
        .route("/webhooks/{webhook_id}/reset", post(webhooks::reset_webhook))
        .route(
            "/webhooks/{webhook_id}/deliveries",
            get(webhooks::get_deliveries),
        )
}
