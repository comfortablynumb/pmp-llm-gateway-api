//! Chat completions endpoint handler

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{
        sse::{Event, Sse},
        IntoResponse, Response,
    },
};

use crate::api::types::Json;
use futures::stream::{Stream, StreamExt};
use serde_json::json;
use tokio_stream::wrappers::ReceiverStream;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use std::time::Instant;

use crate::api::middleware::RequireApiKey;
use crate::api::state::AppState;
use crate::api::types::{
    ApiError, AsyncOperationCreated, AsyncQueryParams, ChatCompletionRequest,
    ChatCompletionResponse, ChatCompletionStreamResponse, ChatMessage, ChatMessageRole,
};
use crate::domain::experiment::AssignmentResult;
use crate::domain::llm::{LlmProvider, LlmRequest, Message};
use crate::domain::OperationType;
use crate::infrastructure::services::RecordExperimentParams;

/// POST /v1/chat/completions
pub async fn create_chat_completion(
    State(state): State<AppState>,
    RequireApiKey(api_key): RequireApiKey,
    Query(async_params): Query<AsyncQueryParams>,
    Json(request): Json<ChatCompletionRequest>,
) -> Result<Response, ApiError> {
    let request_id = Uuid::new_v4().to_string();
    let api_key_id = api_key.id().as_str().to_string();

    info!(
        request_id = %request_id,
        model = %request.model,
        stream = request.stream,
        is_async = async_params.is_async,
        "Processing chat completion request"
    );

    // Validate request
    if request.messages.is_empty() {
        return Err(ApiError::bad_request("Messages cannot be empty").with_param("messages"));
    }

    // Streaming and async mode are incompatible
    if request.stream && async_params.is_async {
        return Err(ApiError::bad_request(
            "Streaming mode is not compatible with async mode",
        ));
    }

    // Check for experiment assignment
    let experiment_assignment = state
        .experiment_service
        .assign_variant(&request.model, &api_key_id)
        .await
        .unwrap_or_else(|e| {
            warn!(error = %e, "Failed to check experiment assignment, proceeding without");
            None
        });

    // Determine effective model and config overrides
    let (effective_model, config_overrides) = match &experiment_assignment {
        Some(assignment) => {
            debug!(
                experiment_id = %assignment.experiment_id,
                variant_id = %assignment.variant_id,
                assigned_model = %assignment.model_id,
                "Experiment assignment active"
            );
            (
                assignment.model_id.clone(),
                assignment.config_overrides.clone(),
            )
        }
        None => (request.model.clone(), None),
    };

    // Convert messages to domain format
    let messages = convert_messages(&request.messages, &state).await?;

    // Build LLM request with potential experiment overrides
    let llm_request = build_llm_request_with_overrides(&request, messages, &config_overrides)?;

    // Handle async mode
    if async_params.is_async {
        return handle_async_chat_completion(
            state,
            request,
            llm_request,
            request_id,
            effective_model,
            api_key_id,
            experiment_assignment,
        )
        .await;
    }

    if request.stream {
        // Streaming response - experiment recording is handled inside
        let stream = create_stream_response(
            state,
            llm_request,
            effective_model.clone(),
            request_id,
            api_key_id,
            experiment_assignment,
        )
        .await;
        Ok(Sse::new(stream)
            .keep_alive(axum::response::sse::KeepAlive::default())
            .into_response())
    } else {
        // Non-streaming response with experiment tracking
        let start_time = Instant::now();

        // Get provider using router based on model configuration
        let provider = get_provider_for_model(&state, &effective_model).await;
        let response_result = provider.chat(&effective_model, llm_request).await;

        let latency_ms = start_time.elapsed().as_millis() as u64;

        // Record experiment if assigned
        if let Some(assignment) = &experiment_assignment {
            let (success, error, input_tokens, output_tokens) = match &response_result {
                Ok(response) => {
                    let usage = response.usage.as_ref();
                    (
                        true,
                        None,
                        usage.map_or(0, |u| u.prompt_tokens),
                        usage.map_or(0, |u| u.completion_tokens),
                    )
                }
                Err(e) => (false, Some(e.to_string()), 0, 0),
            };

            record_experiment_result(
                &state,
                assignment,
                &api_key_id,
                input_tokens,
                output_tokens,
                latency_ms,
                success,
                error,
            )
            .await;
        }

        let response = response_result.map_err(ApiError::from)?;

        let chat_response = ChatCompletionResponse::from_llm_response(
            &response,
            &effective_model,
            &request_id,
        );

        Ok(Json(chat_response).into_response())
    }
}

/// Handle async chat completion request
///
/// Returns a boxed future to avoid stack overflow from large future sizes
/// caused by trait object indirection in AppState services.
fn handle_async_chat_completion(
    state: AppState,
    request: ChatCompletionRequest,
    llm_request: LlmRequest,
    request_id: String,
    effective_model: String,
    api_key_id: String,
    experiment_assignment: Option<AssignmentResult>,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Response, ApiError>> + Send>> {
    Box::pin(async move {
    // Create pending operation
    let operation = state
        .operation_service
        .create_pending(
            OperationType::ChatCompletion,
            serde_json::to_value(&request).unwrap_or(json!({})),
            json!({ "model": &effective_model, "request_id": &request_id }),
        )
        .await
        .map_err(ApiError::from)?;

    let operation_id = operation.id().to_string();
    info!(
        operation_id = %operation_id,
        model = %effective_model,
        "Created async chat completion operation"
    );

    // Clone values for the background task
    let op_id = operation_id.clone();

    // Spawn the background work - the function returns a boxed future to avoid stack overflow
    tokio::spawn(run_async_chat_completion(
        state,
        op_id,
        effective_model,
        llm_request,
        request_id,
        api_key_id,
        experiment_assignment,
    ));

    // Return 202 Accepted
    Ok((
        StatusCode::ACCEPTED,
        Json(AsyncOperationCreated::pending(&operation_id)),
    )
        .into_response())
    })
}

/// Execute chat completion in background and update operation status
///
/// Returns a boxed future to avoid stack overflow from large future sizes
/// caused by trait object indirection in AppState.
fn run_async_chat_completion(
    state: AppState,
    operation_id: String,
    model: String,
    llm_request: LlmRequest,
    request_id: String,
    api_key_id: String,
    experiment_assignment: Option<AssignmentResult>,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> {
    Box::pin(async move {
    // Mark as running
    if let Err(e) = state.operation_service.mark_running(&operation_id).await {
        warn!(
            operation_id = %operation_id,
            error = %e,
            "Failed to mark operation as running"
        );
        return;
    }

    // Execute chat completion with timing
    let start_time = Instant::now();

    // Get provider using router based on model configuration
    let provider = get_provider_for_model(&state, &model).await;
    let response_result = provider.chat(&model, llm_request).await;
    let latency_ms = start_time.elapsed().as_millis() as u64;

    // Record experiment if assigned
    if let Some(ref assignment) = experiment_assignment {
        let (success, error, input_tokens, output_tokens) = match &response_result {
            Ok(response) => {
                let usage = response.usage.as_ref();
                (
                    true,
                    None,
                    usage.map_or(0, |u| u.prompt_tokens),
                    usage.map_or(0, |u| u.completion_tokens),
                )
            }
            Err(e) => (false, Some(e.to_string()), 0, 0),
        };

        record_experiment_result(
            &state,
            assignment,
            &api_key_id,
            input_tokens,
            output_tokens,
            latency_ms,
            success,
            error,
        )
        .await;
    }

    match response_result {
        Ok(response) => {
            let chat_response =
                ChatCompletionResponse::from_llm_response(&response, &model, &request_id);

            let result = serde_json::to_value(&chat_response).unwrap_or(json!({}));

            if let Err(e) = state
                .operation_service
                .mark_completed(&operation_id, result)
                .await
            {
                error!(
                    operation_id = %operation_id,
                    error = %e,
                    "Failed to mark operation as completed"
                );
            } else {
                info!(operation_id = %operation_id, "Async chat completion succeeded");
            }
        }
        Err(e) => {
            let error_msg = e.to_string();

            if let Err(mark_err) = state
                .operation_service
                .mark_failed(&operation_id, error_msg.clone())
                .await
            {
                error!(
                    operation_id = %operation_id,
                    error = %mark_err,
                    "Failed to mark operation as failed"
                );
            } else {
                warn!(
                    operation_id = %operation_id,
                    error = %error_msg,
                    "Async chat completion failed"
                );
            }
        }
    }
    })
}

/// Convert API messages to domain messages, resolving prompt references
async fn convert_messages(
    messages: &[ChatMessage],
    state: &AppState,
) -> Result<Vec<Message>, ApiError> {
    let mut result = Vec::with_capacity(messages.len());

    for msg in messages {
        let content = if let Some(prompt_id) = &msg.prompt_id {
            // Resolve prompt reference
            debug!(prompt_id = %prompt_id, "Resolving prompt reference");

            let variables = msg.variables.clone().unwrap_or_default();
            let rendered = state
                .prompt_service
                .render(prompt_id, &variables)
                .await
                .map_err(|e| {
                    ApiError::bad_request(format!("Failed to render prompt '{}': {}", prompt_id, e))
                        .with_param("prompt_id")
                })?;

            rendered
        } else if let Some(content) = &msg.content {
            content.to_text()
        } else {
            String::new()
        };

        // Create message based on role
        let message = match msg.role {
            ChatMessageRole::System => Message::system(content),
            ChatMessageRole::User => Message::user(content),
            ChatMessageRole::Assistant => Message::assistant(content),
            ChatMessageRole::Tool | ChatMessageRole::Function => Message::user(content),
        };

        result.push(message);
    }

    Ok(result)
}

/// Build LLM request from API request with experiment config overrides
fn build_llm_request_with_overrides(
    request: &ChatCompletionRequest,
    messages: Vec<Message>,
    config_overrides: &Option<crate::domain::experiment::ConfigOverrides>,
) -> Result<LlmRequest, ApiError> {
    let mut builder = LlmRequest::builder().messages(messages);

    // Get effective temperature (experiment override > request > None)
    let temperature = config_overrides
        .as_ref()
        .and_then(|o| o.temperature)
        .or(request.temperature);

    if let Some(temp) = temperature {
        if !(0.0..=2.0).contains(&temp) {
            return Err(
                ApiError::bad_request("Temperature must be between 0 and 2")
                    .with_param("temperature"),
            );
        }
        builder = builder.temperature(temp);
    }

    // Get effective top_p
    let top_p = config_overrides
        .as_ref()
        .and_then(|o| o.top_p)
        .or(request.top_p);

    if let Some(top_p) = top_p {
        if !(0.0..=1.0).contains(&top_p) {
            return Err(
                ApiError::bad_request("top_p must be between 0 and 1").with_param("top_p"),
            );
        }
        builder = builder.top_p(top_p);
    }

    // Get effective max_tokens
    let max_tokens = config_overrides
        .as_ref()
        .and_then(|o| o.max_tokens)
        .or(request.max_tokens);

    if let Some(max_tokens) = max_tokens {
        builder = builder.max_tokens(max_tokens);
    }

    if let Some(stop) = &request.stop {
        builder = builder.stop(stop.to_vec());
    }

    // Get effective presence_penalty
    let presence_penalty = config_overrides
        .as_ref()
        .and_then(|o| o.presence_penalty)
        .or(request.presence_penalty);

    if let Some(presence_penalty) = presence_penalty {
        if !(-2.0..=2.0).contains(&presence_penalty) {
            return Err(
                ApiError::bad_request("presence_penalty must be between -2 and 2")
                    .with_param("presence_penalty"),
            );
        }
        builder = builder.presence_penalty(presence_penalty);
    }

    // Get effective frequency_penalty
    let frequency_penalty = config_overrides
        .as_ref()
        .and_then(|o| o.frequency_penalty)
        .or(request.frequency_penalty);

    if let Some(frequency_penalty) = frequency_penalty {
        if !(-2.0..=2.0).contains(&frequency_penalty) {
            return Err(
                ApiError::bad_request("frequency_penalty must be between -2 and 2")
                    .with_param("frequency_penalty"),
            );
        }
        builder = builder.frequency_penalty(frequency_penalty);
    }

    if let Some(user) = &request.user {
        builder = builder.user(user);
    }

    Ok(builder.build())
}

/// Record experiment result
async fn record_experiment_result(
    state: &AppState,
    assignment: &AssignmentResult,
    api_key_id: &str,
    input_tokens: u32,
    output_tokens: u32,
    latency_ms: u64,
    success: bool,
    error: Option<String>,
) {
    let params = RecordExperimentParams {
        experiment_id: assignment.experiment_id.clone(),
        variant_id: assignment.variant_id.clone(),
        api_key_id: api_key_id.to_string(),
        model_id: assignment.model_id.clone(),
        input_tokens,
        output_tokens,
        cost_micros: 0, // Cost calculation would need model pricing integration
        latency_ms,
        success,
        error,
    };

    if let Err(e) = state.experiment_service.record(params).await {
        warn!(
            experiment_id = %assignment.experiment_id,
            variant_id = %assignment.variant_id,
            error = %e,
            "Failed to record experiment result"
        );
    } else {
        debug!(
            experiment_id = %assignment.experiment_id,
            variant_id = %assignment.variant_id,
            "Recorded experiment result"
        );
    }
}

/// Create streaming response
async fn create_stream_response(
    state: AppState,
    request: LlmRequest,
    model: String,
    request_id: String,
    api_key_id: String,
    experiment_assignment: Option<AssignmentResult>,
) -> impl Stream<Item = Result<Event, std::convert::Infallible>> {
    let (tx, rx) = tokio::sync::mpsc::channel::<Result<Event, std::convert::Infallible>>(32);

    // Get provider before spawning to avoid lifetime issues
    let provider = get_provider_for_model(&state, &model).await;

    tokio::spawn(Box::pin(async move {
        let start_time = Instant::now();

        // Send initial chunk with role
        let initial = ChatCompletionStreamResponse::initial(&model, &request_id);
        let _ = tx
            .send(Ok(Event::default().data(serde_json::to_string(&initial).unwrap())))
            .await;

        // Track success for experiment recording
        let mut stream_success = true;
        let mut stream_error: Option<String> = None;

        // Get streaming response from provider
        match provider.chat_stream(&model, request).await {
            Ok(mut stream) => {
                while let Some(chunk_result) = stream.next().await {
                    match chunk_result {
                        Ok(chunk) => {
                            if let Some(content) = &chunk.delta {
                                let content_chunk = ChatCompletionStreamResponse::content(
                                    &model,
                                    &request_id,
                                    content,
                                );
                                let data = serde_json::to_string(&content_chunk).unwrap();

                                if tx.send(Ok(Event::default().data(data))).await.is_err() {
                                    break;
                                }
                            }
                        }
                        Err(e) => {
                            error!("Stream error: {}", e);
                            stream_success = false;
                            stream_error = Some(e.to_string());
                            break;
                        }
                    }
                }

                // Send final chunk
                let finish = ChatCompletionStreamResponse::finish(&model, &request_id, None);
                let _ = tx
                    .send(Ok(Event::default().data(serde_json::to_string(&finish).unwrap())))
                    .await;

                // Send [DONE]
                let _ = tx.send(Ok(Event::default().data("[DONE]"))).await;
            }
            Err(e) => {
                error!("Failed to start stream: {}", e);
                stream_success = false;
                stream_error = Some(e.to_string());
            }
        }

        // Record experiment result (note: token counts not available for streaming)
        let latency_ms = start_time.elapsed().as_millis() as u64;

        if let Some(ref assignment) = experiment_assignment {
            record_experiment_result(
                &state,
                assignment,
                &api_key_id,
                0, // Token counts not available for streaming
                0,
                latency_ms,
                stream_success,
                stream_error,
            )
            .await;
        }
    }));

    ReceiverStream::new(rx)
}

/// Get the appropriate LLM provider for a model
///
/// This function tries to use the plugin router to get a provider based on
/// the model's credential configuration. Falls back to the default provider
/// if the model is not found or the router fails.
async fn get_provider_for_model(
    state: &AppState,
    model_id: &str,
) -> std::sync::Arc<dyn LlmProvider> {
    // Try to get the model configuration
    let model = match state.model_service.get(model_id).await {
        Ok(Some(model)) => model,
        Ok(None) => {
            debug!(model_id = %model_id, "Model not found, using default provider");
            return state.llm_provider.clone();
        }
        Err(e) => {
            warn!(model_id = %model_id, error = %e, "Failed to get model, using default provider");
            return state.llm_provider.clone();
        }
    };

    // Get the credential for this model
    let credential_id = model.credential_id();
    let stored_credential = match state.credential_service.get(credential_id).await {
        Ok(Some(cred)) => cred,
        Ok(None) => {
            warn!(
                model_id = %model_id,
                credential_id = %credential_id,
                "Credential not found, using default provider"
            );
            return state.llm_provider.clone();
        }
        Err(e) => {
            warn!(
                model_id = %model_id,
                credential_id = %credential_id,
                error = %e,
                "Failed to get credential, using default provider"
            );
            return state.llm_provider.clone();
        }
    };

    // Check if credential is enabled
    if !stored_credential.is_enabled() {
        warn!(
            model_id = %model_id,
            credential_id = %credential_id,
            "Credential is disabled, using default provider"
        );
        return state.llm_provider.clone();
    }

    // Convert to domain Credential and use router
    let credential = stored_credential.to_credential();

    match state.provider_router.get_provider(&model, &credential).await {
        Ok(provider) => {
            debug!(
                model_id = %model_id,
                credential_id = %credential_id,
                "Using provider from router"
            );
            provider
        }
        Err(e) => {
            warn!(
                model_id = %model_id,
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

    #[test]
    fn test_build_llm_request_basic() {
        let request = ChatCompletionRequest {
            model: "gpt-4".to_string(),
            messages: vec![],
            temperature: None,
            top_p: None,
            n: None,
            stream: false,
            stream_options: None,
            stop: None,
            max_tokens: None,
            presence_penalty: None,
            frequency_penalty: None,
            user: None,
            seed: None,
        };

        let messages = vec![Message::user("Hello")];
        let result = build_llm_request_with_overrides(&request, messages, &None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_llm_request_invalid_temperature() {
        let request = ChatCompletionRequest {
            model: "gpt-4".to_string(),
            messages: vec![],
            temperature: Some(3.0),
            top_p: None,
            n: None,
            stream: false,
            stream_options: None,
            stop: None,
            max_tokens: None,
            presence_penalty: None,
            frequency_penalty: None,
            user: None,
            seed: None,
        };

        let messages = vec![Message::user("Hello")];
        let result = build_llm_request_with_overrides(&request, messages, &None);
        assert!(result.is_err());
    }

    #[test]
    fn test_build_llm_request_with_options() {
        let request = ChatCompletionRequest {
            model: "gpt-4".to_string(),
            messages: vec![],
            temperature: Some(0.7),
            top_p: Some(0.9),
            n: None,
            stream: false,
            stream_options: None,
            stop: Some(crate::api::types::StopSequence::Single("END".to_string())),
            max_tokens: Some(100),
            presence_penalty: Some(0.5),
            frequency_penalty: Some(-0.5),
            user: Some("user123".to_string()),
            seed: None,
        };

        let messages = vec![Message::user("Hello")];
        let result = build_llm_request_with_overrides(&request, messages, &None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_llm_request_with_experiment_overrides() {
        use crate::domain::experiment::ConfigOverrides;

        let request = ChatCompletionRequest {
            model: "gpt-4".to_string(),
            messages: vec![],
            temperature: Some(0.7),
            top_p: None,
            n: None,
            stream: false,
            stream_options: None,
            stop: None,
            max_tokens: Some(100),
            presence_penalty: None,
            frequency_penalty: None,
            user: None,
            seed: None,
        };

        // Experiment overrides should take precedence
        let overrides = Some(ConfigOverrides {
            temperature: Some(0.3),
            max_tokens: Some(200),
            top_p: None,
            presence_penalty: None,
            frequency_penalty: None,
        });

        let messages = vec![Message::user("Hello")];
        let result = build_llm_request_with_overrides(&request, messages, &overrides);
        assert!(result.is_ok());

        let llm_request = result.unwrap();
        // Experiment override should win
        assert_eq!(llm_request.temperature, Some(0.3));
        assert_eq!(llm_request.max_tokens, Some(200));
    }
}
