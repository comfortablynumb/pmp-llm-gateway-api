//! Model management admin endpoints

use std::collections::HashMap;
use std::time::Instant;

use axum::extract::{Path, State};
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use crate::api::middleware::{AdminAuth, RequireAdmin};
use crate::api::state::AppState;
use crate::api::types::{ApiError, Json};
use crate::domain::credentials::CredentialType;
use crate::domain::llm::{LlmJsonSchema, LlmProvider, LlmRequest, LlmResponseFormat, Message};
use crate::domain::model::{Model, ModelConfig};
use crate::domain::usage::default_model_pricing;
use crate::domain::{ExecutionTokenUsage, Executor};
use crate::infrastructure::services::{CreateModelRequest, RecordExecutionParams, UpdateModelRequest};

/// Request to create a new model
#[derive(Debug, Clone, Deserialize)]
pub struct CreateModelApiRequest {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub provider_model: String,
    pub credential_id: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub config: ModelConfigRequest,
}

fn default_enabled() -> bool {
    true
}

/// Request to update a model
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateModelApiRequest {
    pub name: Option<String>,
    pub provider_model: Option<String>,
    pub credential_id: Option<String>,
    pub enabled: Option<bool>,
    pub config: Option<ModelConfigRequest>,
}

/// Model configuration in request format
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ModelConfigRequest {
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub top_p: Option<f32>,
    pub presence_penalty: Option<f32>,
    pub frequency_penalty: Option<f32>,
    pub timeout_ms: Option<u64>,
    pub max_retries: Option<u32>,
    pub retry_delay_ms: Option<u64>,
    pub fallback_model_id: Option<String>,
}

/// Model response for admin API
#[derive(Debug, Clone, Serialize)]
pub struct ModelResponse {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub provider_model: String,
    pub credential_id: String,
    pub enabled: bool,
    pub config: ModelConfigResponse,
    pub config_version: u32,
    pub created_at: String,
    pub updated_at: String,
}

/// Model configuration in response format
#[derive(Debug, Clone, Serialize)]
pub struct ModelConfigResponse {
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub top_p: Option<f32>,
    pub presence_penalty: Option<f32>,
    pub frequency_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_retries: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_delay_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback_model_id: Option<String>,
}

fn credential_type_to_string(ct: &CredentialType) -> String {
    match ct {
        CredentialType::OpenAi => "openai".to_string(),
        CredentialType::Anthropic => "anthropic".to_string(),
        CredentialType::AzureOpenAi => "azure_openai".to_string(),
        CredentialType::AwsBedrock => "aws_bedrock".to_string(),
        CredentialType::Pgvector => "pgvector".to_string(),
        CredentialType::AwsKnowledgeBase => "aws_knowledge_base".to_string(),
        CredentialType::Pinecone => "pinecone".to_string(),
        CredentialType::HttpApiKey => "http_api_key".to_string(),
        CredentialType::Custom(s) => s.clone(),
    }
}

fn parse_credential_type(s: &str) -> Result<CredentialType, ApiError> {
    match s.to_lowercase().as_str() {
        "openai" => Ok(CredentialType::OpenAi),
        "anthropic" => Ok(CredentialType::Anthropic),
        "azure_openai" | "azure-openai" | "azureopenai" => Ok(CredentialType::AzureOpenAi),
        "aws_bedrock" | "aws-bedrock" | "awsbedrock" | "bedrock" => Ok(CredentialType::AwsBedrock),
        other => Ok(CredentialType::Custom(other.to_string())),
    }
}

impl From<&Model> for ModelResponse {
    fn from(model: &Model) -> Self {
        let config = model.config();
        Self {
            id: model.id().as_str().to_string(),
            name: model.name().to_string(),
            provider: credential_type_to_string(model.provider()),
            provider_model: model.provider_model().to_string(),
            credential_id: model.credential_id().to_string(),
            enabled: model.is_enabled(),
            config: ModelConfigResponse {
                temperature: config.temperature,
                max_tokens: config.max_tokens,
                top_p: config.top_p,
                presence_penalty: config.presence_penalty,
                frequency_penalty: config.frequency_penalty,
                timeout_ms: config.timeout_ms,
                max_retries: config.max_retries,
                retry_delay_ms: config.retry_delay_ms,
                fallback_model_id: config.fallback_model_id.clone(),
            },
            config_version: model.version(),
            created_at: model.created_at().to_rfc3339(),
            updated_at: model.updated_at().to_rfc3339(),
        }
    }
}

/// List models response
#[derive(Debug, Clone, Serialize)]
pub struct ListModelsResponse {
    pub models: Vec<ModelResponse>,
    pub total: usize,
}

fn build_model_config(req: &ModelConfigRequest) -> Option<ModelConfig> {
    let mut config = ModelConfig::new();
    let mut has_value = false;

    if let Some(temp) = req.temperature {
        config = config.with_temperature(temp);
        has_value = true;
    }

    if let Some(max_tokens) = req.max_tokens {
        config = config.with_max_tokens(max_tokens);
        has_value = true;
    }

    if let Some(top_p) = req.top_p {
        config = config.with_top_p(top_p);
        has_value = true;
    }

    if let Some(presence) = req.presence_penalty {
        config = config.with_presence_penalty(presence);
        has_value = true;
    }

    if let Some(frequency) = req.frequency_penalty {
        config = config.with_frequency_penalty(frequency);
        has_value = true;
    }

    if let Some(timeout) = req.timeout_ms {
        config = config.with_timeout_ms(timeout);
        has_value = true;
    }

    if let Some(max_retries) = req.max_retries {
        config = config.with_max_retries(max_retries);
        has_value = true;
    }

    if let Some(retry_delay) = req.retry_delay_ms {
        config = config.with_retry_delay_ms(retry_delay);
        has_value = true;
    }

    if let Some(ref fallback) = req.fallback_model_id {
        config = config.with_fallback_model_id(fallback);
        has_value = true;
    }

    if has_value {
        Some(config)
    } else {
        None
    }
}

/// GET /admin/models
pub async fn list_models(
    State(state): State<AppState>,
    RequireAdmin(_): RequireAdmin,
) -> Result<Json<ListModelsResponse>, ApiError> {
    debug!("Admin listing all models");

    let models = state.model_service.list().await.map_err(ApiError::from)?;

    let model_responses: Vec<ModelResponse> = models.iter().map(ModelResponse::from).collect();
    let total = model_responses.len();

    Ok(Json(ListModelsResponse {
        models: model_responses,
        total,
    }))
}

/// POST /admin/models
pub async fn create_model(
    State(state): State<AppState>,
    RequireAdmin(_): RequireAdmin,
    Json(request): Json<CreateModelApiRequest>,
) -> Result<Json<ModelResponse>, ApiError> {
    debug!(model_id = %request.id, "Admin creating model");

    let provider = parse_credential_type(&request.provider)?;

    let create_request = CreateModelRequest {
        id: request.id,
        name: request.name,
        description: None,
        provider,
        provider_model: request.provider_model,
        credential_id: request.credential_id,
        config: build_model_config(&request.config),
        enabled: request.enabled,
    };

    let model = state
        .model_service
        .create(create_request)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(ModelResponse::from(&model)))
}

/// GET /admin/models/:model_id
pub async fn get_model(
    State(state): State<AppState>,
    RequireAdmin(_): RequireAdmin,
    Path(model_id): Path<String>,
) -> Result<Json<ModelResponse>, ApiError> {
    debug!(model_id = %model_id, "Admin getting model");

    let model = state
        .model_service
        .get(&model_id)
        .await
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError::not_found(format!("Model '{}' not found", model_id)))?;

    Ok(Json(ModelResponse::from(&model)))
}

/// PUT /admin/models/:model_id
pub async fn update_model(
    State(state): State<AppState>,
    RequireAdmin(_): RequireAdmin,
    Path(model_id): Path<String>,
    Json(request): Json<UpdateModelApiRequest>,
) -> Result<Json<ModelResponse>, ApiError> {
    debug!(model_id = %model_id, "Admin updating model");

    let update_request = UpdateModelRequest {
        name: request.name,
        description: None,
        provider_model: request.provider_model,
        credential_id: request.credential_id,
        config: request.config.as_ref().and_then(build_model_config),
        enabled: request.enabled,
    };

    let model = state
        .model_service
        .update(&model_id, update_request)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(ModelResponse::from(&model)))
}

/// DELETE /admin/models/:model_id
pub async fn delete_model(
    State(state): State<AppState>,
    RequireAdmin(_): RequireAdmin,
    Path(model_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    debug!(model_id = %model_id, "Admin deleting model");

    state
        .model_service
        .delete(&model_id)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(serde_json::json!({
        "deleted": true,
        "id": model_id
    })))
}

/// Request to execute a model with a prompt
#[derive(Debug, Clone, Deserialize)]
pub struct ExecuteModelRequest {
    /// Prompt ID to use for the system message
    #[serde(default)]
    pub prompt_id: Option<String>,
    /// Variables to substitute in the prompt template
    #[serde(default)]
    pub variables: HashMap<String, String>,
    /// User message to send
    pub user_message: String,
    /// Optional temperature override
    #[serde(default)]
    pub temperature: Option<f32>,
    /// Optional max_tokens override
    #[serde(default)]
    pub max_tokens: Option<u32>,
    /// Optional response format for structured outputs
    #[serde(default)]
    pub response_format: Option<ResponseFormat>,
}

/// Response format specification for structured outputs
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResponseFormat {
    /// Return plain text (default)
    Text,
    /// Return JSON object (model chooses structure)
    JsonObject,
    /// Return JSON matching a specific schema
    JsonSchema {
        json_schema: JsonSchemaSpec,
    },
}

/// JSON Schema specification
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct JsonSchemaSpec {
    /// Name of the schema
    pub name: String,
    /// Whether to enforce strict mode
    #[serde(default)]
    pub strict: bool,
    /// The JSON schema definition
    pub schema: serde_json::Value,
}

/// Variable information for a prompt
#[derive(Debug, Clone, Serialize)]
pub struct PromptVariableInfo {
    pub name: String,
    pub required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
}

/// Response from model execution
#[derive(Debug, Clone, Serialize)]
pub struct ExecuteModelResponse {
    pub model_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_id: Option<String>,
    pub content: String,
    pub usage: ExecuteModelUsage,
    pub execution_time_ms: u64,
}

/// Usage information from model execution
#[derive(Debug, Clone, Serialize)]
pub struct ExecuteModelUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// POST /admin/models/:model_id/execute
/// Execute a model with an optional prompt and user message
pub async fn execute_model(
    State(state): State<AppState>,
    RequireAdmin(admin): RequireAdmin,
    Path(model_id): Path<String>,
    Json(request): Json<ExecuteModelRequest>,
) -> Result<Json<ExecuteModelResponse>, ApiError> {
    info!(model_id = %model_id, "Admin executing model");
    let start = Instant::now();

    // Create executor from admin auth for logging
    let executor = match &admin {
        AdminAuth::ApiKey(key) => Executor::from_api_key(key.id().as_str()),
        AdminAuth::User(user) => Executor::from_user(user.id().as_str()),
    };

    // Get and validate the model
    let model = state
        .model_service
        .get(&model_id)
        .await
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError::not_found(format!("Model '{}' not found", model_id)))?;

    if !model.is_enabled() {
        return Err(ApiError::bad_request(format!(
            "Model '{}' is disabled",
            model_id
        )));
    }

    // Build messages
    let mut messages = Vec::new();

    // Add system message from prompt if provided
    if let Some(ref prompt_id) = request.prompt_id {
        let rendered = state
            .prompt_service
            .render(prompt_id, &request.variables)
            .await
            .map_err(|e| {
                ApiError::bad_request(format!("Failed to render prompt '{}': {}", prompt_id, e))
            })?;
        messages.push(Message::system(rendered));
    }

    // Add user message
    messages.push(Message::user(&request.user_message));

    // Build LLM request
    let mut llm_request_builder = LlmRequest::builder().messages(messages);

    // Apply model config defaults
    let model_config = model.config();

    if let Some(temp) = request.temperature.or(model_config.temperature) {
        llm_request_builder = llm_request_builder.temperature(temp);
    }

    if let Some(max_tokens) = request.max_tokens.or(model_config.max_tokens) {
        llm_request_builder = llm_request_builder.max_tokens(max_tokens);
    }

    if let Some(top_p) = model_config.top_p {
        llm_request_builder = llm_request_builder.top_p(top_p);
    }

    if let Some(presence_penalty) = model_config.presence_penalty {
        llm_request_builder = llm_request_builder.presence_penalty(presence_penalty);
    }

    if let Some(frequency_penalty) = model_config.frequency_penalty {
        llm_request_builder = llm_request_builder.frequency_penalty(frequency_penalty);
    }

    // Apply response format if provided
    if let Some(ref response_format) = request.response_format {
        let llm_format = match response_format {
            ResponseFormat::Text => LlmResponseFormat::Text,
            ResponseFormat::JsonObject => LlmResponseFormat::JsonObject,
            ResponseFormat::JsonSchema { json_schema } => LlmResponseFormat::JsonSchema {
                json_schema: LlmJsonSchema {
                    name: json_schema.name.clone(),
                    strict: json_schema.strict,
                    schema: json_schema.schema.clone(),
                },
            },
        };
        llm_request_builder = llm_request_builder.response_format(llm_format);
    }

    let llm_request = llm_request_builder.build();

    // Get provider for this model
    let provider = get_provider_for_model(&state, &model).await;

    // Execute
    let response = provider
        .chat(model.provider_model(), llm_request)
        .await
        .map_err(ApiError::from)?;

    let execution_time_ms = start.elapsed().as_millis() as u64;

    // Extract content from response
    let content = response.content().unwrap_or("").to_string();

    // Extract usage
    let usage = response.usage.as_ref().map_or(
        ExecuteModelUsage {
            prompt_tokens: 0,
            completion_tokens: 0,
            total_tokens: 0,
        },
        |u| ExecuteModelUsage {
            prompt_tokens: u.prompt_tokens,
            completion_tokens: u.completion_tokens,
            total_tokens: u.total_tokens,
        },
    );

    // Record execution log
    let mut log_params = RecordExecutionParams::model_success(&model_id, execution_time_ms, executor);
    log_params.token_usage = response.usage.as_ref().map(|u| ExecutionTokenUsage {
        input_tokens: u.prompt_tokens,
        output_tokens: u.completion_tokens,
        total_tokens: u.total_tokens,
    });
    log_params.resource_name = Some(model.name().to_string());

    // Calculate cost from model pricing
    if let Some(usage) = &response.usage {
        let pricing_map = default_model_pricing();

        if let Some(pricing) = pricing_map.get(model.provider_model()) {
            log_params.cost_micros = Some(pricing.calculate_cost(usage.prompt_tokens, usage.completion_tokens));
        }
    }

    if let Err(e) = state.execution_log_service.record(log_params).await {
        debug!(error = %e, "Failed to record execution log");
    }

    Ok(Json(ExecuteModelResponse {
        model_id,
        prompt_id: request.prompt_id,
        content,
        usage,
        execution_time_ms,
    }))
}

/// Get the appropriate LLM provider for a model
async fn get_provider_for_model(
    state: &AppState,
    model: &Model,
) -> std::sync::Arc<dyn LlmProvider> {
    let credential_id = model.credential_id();

    // Get the credential for this model
    let stored_credential = match state.credential_service.get(credential_id).await {
        Ok(Some(cred)) => cred,
        Ok(None) | Err(_) => {
            debug!(
                model_id = %model.id(),
                credential_id = %credential_id,
                "Credential not found, using default provider"
            );
            return state.llm_provider.clone();
        }
    };

    if !stored_credential.is_enabled() {
        debug!(
            model_id = %model.id(),
            credential_id = %credential_id,
            "Credential is disabled, using default provider"
        );
        return state.llm_provider.clone();
    }

    // Convert to domain Credential and use router
    let credential = stored_credential.to_credential();

    match state.provider_router.get_provider(model, &credential).await {
        Ok(provider) => {
            debug!(
                model_id = %model.id(),
                credential_id = %credential_id,
                "Using provider from router"
            );
            provider
        }
        Err(e) => {
            debug!(
                model_id = %model.id(),
                error = %e,
                "Failed to get provider from router, using default"
            );
            state.llm_provider.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::model::ModelId;

    #[test]
    fn test_create_model_request_deserialization() {
        let json = r#"{
            "id": "my-gpt4",
            "name": "My GPT-4",
            "provider": "openai",
            "provider_model": "gpt-4",
            "credential_id": "openai-prod",
            "enabled": true,
            "config": {
                "temperature": 0.7,
                "max_tokens": 1000
            }
        }"#;

        let request: CreateModelApiRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.id, "my-gpt4");
        assert_eq!(request.provider, "openai");
        assert_eq!(request.credential_id, "openai-prod");
        assert!(request.enabled);
        assert_eq!(request.config.temperature, Some(0.7));
    }

    #[test]
    fn test_create_model_request_minimal() {
        let json = r#"{
            "id": "test-model",
            "name": "Test",
            "provider": "openai",
            "provider_model": "gpt-4",
            "credential_id": "cred-1"
        }"#;

        let request: CreateModelApiRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.id, "test-model");
        assert!(request.enabled);
        assert!(request.config.temperature.is_none());
    }

    #[test]
    fn test_update_model_request_partial() {
        let json = r#"{"name": "New Name"}"#;

        let request: UpdateModelApiRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.name, Some("New Name".to_string()));
        assert!(request.provider_model.is_none());
        assert!(request.credential_id.is_none());
        assert!(request.enabled.is_none());
        assert!(request.config.is_none());
    }

    #[test]
    fn test_update_model_request_full() {
        let json = r#"{
            "name": "Updated Model",
            "provider_model": "gpt-4-turbo",
            "credential_id": "new-cred",
            "enabled": false,
            "config": {
                "temperature": 0.9,
                "max_tokens": 2000
            }
        }"#;

        let request: UpdateModelApiRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.name, Some("Updated Model".to_string()));
        assert_eq!(request.provider_model, Some("gpt-4-turbo".to_string()));
        assert_eq!(request.credential_id, Some("new-cred".to_string()));
        assert_eq!(request.enabled, Some(false));
        assert!(request.config.is_some());
    }

    #[test]
    fn test_parse_credential_type() {
        assert!(matches!(
            parse_credential_type("openai").unwrap(),
            CredentialType::OpenAi
        ));
        assert!(matches!(
            parse_credential_type("anthropic").unwrap(),
            CredentialType::Anthropic
        ));
        assert!(matches!(
            parse_credential_type("azure_openai").unwrap(),
            CredentialType::AzureOpenAi
        ));
    }

    #[test]
    fn test_parse_credential_type_variants() {
        assert!(matches!(
            parse_credential_type("azure-openai").unwrap(),
            CredentialType::AzureOpenAi
        ));
        assert!(matches!(
            parse_credential_type("azureopenai").unwrap(),
            CredentialType::AzureOpenAi
        ));
        assert!(matches!(
            parse_credential_type("aws_bedrock").unwrap(),
            CredentialType::AwsBedrock
        ));
        assert!(matches!(
            parse_credential_type("bedrock").unwrap(),
            CredentialType::AwsBedrock
        ));
    }

    #[test]
    fn test_parse_credential_type_custom() {
        let result = parse_credential_type("my-custom-provider").unwrap();
        assert!(matches!(result, CredentialType::Custom(s) if s == "my-custom-provider"));
    }

    #[test]
    fn test_credential_type_to_string() {
        assert_eq!(credential_type_to_string(&CredentialType::OpenAi), "openai");
        assert_eq!(credential_type_to_string(&CredentialType::Anthropic), "anthropic");
        assert_eq!(credential_type_to_string(&CredentialType::AzureOpenAi), "azure_openai");
        assert_eq!(credential_type_to_string(&CredentialType::AwsBedrock), "aws_bedrock");
        assert_eq!(credential_type_to_string(&CredentialType::Pgvector), "pgvector");
        assert_eq!(credential_type_to_string(&CredentialType::HttpApiKey), "http_api_key");
        assert_eq!(
            credential_type_to_string(&CredentialType::Custom("custom".to_string())),
            "custom"
        );
    }

    #[test]
    fn test_default_enabled() {
        assert!(default_enabled());
    }

    #[test]
    fn test_build_model_config_empty() {
        let req = ModelConfigRequest::default();
        assert!(build_model_config(&req).is_none());
    }

    #[test]
    fn test_build_model_config_with_values() {
        let req = ModelConfigRequest {
            temperature: Some(0.7),
            max_tokens: Some(1000),
            top_p: Some(0.9),
            presence_penalty: Some(0.1),
            frequency_penalty: Some(0.2),
            timeout_ms: Some(30000),
            max_retries: Some(3),
            retry_delay_ms: Some(1000),
            fallback_model_id: Some("fallback".to_string()),
        };

        let config = build_model_config(&req).unwrap();
        assert_eq!(config.temperature, Some(0.7));
        assert_eq!(config.max_tokens, Some(1000));
        assert_eq!(config.top_p, Some(0.9));
    }

    #[test]
    fn test_build_model_config_partial() {
        let req = ModelConfigRequest {
            temperature: Some(0.5),
            ..Default::default()
        };

        let config = build_model_config(&req).unwrap();
        assert_eq!(config.temperature, Some(0.5));
        assert!(config.max_tokens.is_none());
    }

    #[test]
    fn test_model_response_from() {
        let id = ModelId::new("test-model").unwrap();
        let model = Model::new(
            id,
            "Test Model",
            CredentialType::OpenAi,
            "gpt-4",
            "openai-cred",
        );

        let response = ModelResponse::from(&model);

        assert_eq!(response.id, "test-model");
        assert_eq!(response.name, "Test Model");
        assert_eq!(response.provider, "openai");
        assert_eq!(response.provider_model, "gpt-4");
        assert_eq!(response.credential_id, "openai-cred");
        assert!(response.enabled);
        assert_eq!(response.config_version, 1);
    }

    #[test]
    fn test_model_config_response_serialization() {
        let config = ModelConfigResponse {
            temperature: Some(0.7),
            max_tokens: Some(1000),
            top_p: None,
            presence_penalty: None,
            frequency_penalty: None,
            timeout_ms: None,
            max_retries: None,
            retry_delay_ms: None,
            fallback_model_id: None,
        };

        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("\"temperature\":0.7"));
        assert!(json.contains("\"max_tokens\":1000"));
        assert!(!json.contains("timeout_ms"));
    }

    #[test]
    fn test_list_models_response_serialization() {
        let list_response = ListModelsResponse {
            models: vec![],
            total: 0,
        };

        let json = serde_json::to_string(&list_response).unwrap();
        assert!(json.contains("\"models\":[]"));
        assert!(json.contains("\"total\":0"));
    }

    #[test]
    fn test_execute_model_request_deserialization_minimal() {
        let json = r#"{
            "user_message": "Hello, world!"
        }"#;

        let request: ExecuteModelRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.user_message, "Hello, world!");
        assert!(request.prompt_id.is_none());
        assert!(request.variables.is_empty());
        assert!(request.temperature.is_none());
        assert!(request.max_tokens.is_none());
    }

    #[test]
    fn test_execute_model_request_deserialization_full() {
        let json = r#"{
            "prompt_id": "system-prompt",
            "variables": {"name": "Alice", "role": "assistant"},
            "user_message": "What is your name?",
            "temperature": 0.5,
            "max_tokens": 500
        }"#;

        let request: ExecuteModelRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.prompt_id, Some("system-prompt".to_string()));
        assert_eq!(request.variables.get("name"), Some(&"Alice".to_string()));
        assert_eq!(request.variables.get("role"), Some(&"assistant".to_string()));
        assert_eq!(request.user_message, "What is your name?");
        assert_eq!(request.temperature, Some(0.5));
        assert_eq!(request.max_tokens, Some(500));
    }

    #[test]
    fn test_execute_model_response_serialization() {
        let response = ExecuteModelResponse {
            model_id: "gpt-4".to_string(),
            prompt_id: Some("test-prompt".to_string()),
            content: "Hello! I'm an AI assistant.".to_string(),
            usage: ExecuteModelUsage {
                prompt_tokens: 10,
                completion_tokens: 20,
                total_tokens: 30,
            },
            execution_time_ms: 150,
        };

        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["model_id"], "gpt-4");
        assert_eq!(json["prompt_id"], "test-prompt");
        assert_eq!(json["content"], "Hello! I'm an AI assistant.");
        assert_eq!(json["usage"]["prompt_tokens"], 10);
        assert_eq!(json["usage"]["completion_tokens"], 20);
        assert_eq!(json["usage"]["total_tokens"], 30);
        assert_eq!(json["execution_time_ms"], 150);
    }

    #[test]
    fn test_execute_model_response_without_prompt() {
        let response = ExecuteModelResponse {
            model_id: "gpt-4".to_string(),
            prompt_id: None,
            content: "Response without prompt".to_string(),
            usage: ExecuteModelUsage {
                prompt_tokens: 5,
                completion_tokens: 10,
                total_tokens: 15,
            },
            execution_time_ms: 100,
        };

        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["model_id"], "gpt-4");
        assert!(json.get("prompt_id").is_none());
    }

    #[test]
    fn test_response_format_text() {
        let json = r#"{"type": "text"}"#;
        let format: ResponseFormat = serde_json::from_str(json).unwrap();
        assert!(matches!(format, ResponseFormat::Text));
    }

    #[test]
    fn test_response_format_json_object() {
        let json = r#"{"type": "json_object"}"#;
        let format: ResponseFormat = serde_json::from_str(json).unwrap();
        assert!(matches!(format, ResponseFormat::JsonObject));
    }

    #[test]
    fn test_response_format_json_schema() {
        let json = r#"{
            "type": "json_schema",
            "json_schema": {
                "name": "response",
                "strict": true,
                "schema": {"type": "object"}
            }
        }"#;
        let format: ResponseFormat = serde_json::from_str(json).unwrap();

        if let ResponseFormat::JsonSchema { json_schema } = format {
            assert_eq!(json_schema.name, "response");
            assert!(json_schema.strict);
        } else {
            panic!("Expected JsonSchema variant");
        }
    }

    #[test]
    fn test_prompt_variable_info_serialization() {
        let info = PromptVariableInfo {
            name: "user_name".to_string(),
            required: true,
            default: Some("Guest".to_string()),
        };

        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"name\":\"user_name\""));
        assert!(json.contains("\"required\":true"));
        assert!(json.contains("\"default\":\"Guest\""));
    }
}
