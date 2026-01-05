//! Chat completions endpoint handler

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{
        sse::{Event, Sse},
        IntoResponse, Response,
    },
    Json,
};
use futures::stream::{Stream, StreamExt};
use serde_json::json;
use tokio_stream::wrappers::ReceiverStream;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::api::middleware::RequireApiKey;
use crate::api::state::AppState;
use crate::api::types::{
    ApiError, AsyncOperationCreated, AsyncQueryParams, ChatCompletionRequest,
    ChatCompletionResponse, ChatCompletionStreamResponse, ChatMessage, ChatMessageRole,
};
use crate::domain::llm::{LlmRequest, Message};
use crate::domain::OperationType;

/// POST /v1/chat/completions
pub async fn create_chat_completion(
    State(state): State<AppState>,
    RequireApiKey(_api_key): RequireApiKey,
    Query(async_params): Query<AsyncQueryParams>,
    Json(request): Json<ChatCompletionRequest>,
) -> Result<Response, ApiError> {
    let request_id = Uuid::new_v4().to_string();
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

    // Convert messages to domain format
    let messages = convert_messages(&request.messages, &state).await?;

    // Build LLM request
    let llm_request = build_llm_request(&request, messages)?;

    // Handle async mode
    if async_params.is_async {
        return handle_async_chat_completion(state, request, llm_request, request_id).await;
    }

    if request.stream {
        // Streaming response
        let stream = create_stream_response(state, llm_request, request.model, request_id).await;
        Ok(Sse::new(stream)
            .keep_alive(axum::response::sse::KeepAlive::default())
            .into_response())
    } else {
        // Non-streaming response
        let response = state
            .llm_provider
            .chat(&request.model, llm_request)
            .await
            .map_err(ApiError::from)?;

        let chat_response = ChatCompletionResponse::from_llm_response(
            &response,
            &request.model,
            &request_id,
        );

        Ok(Json(chat_response).into_response())
    }
}

/// Handle async chat completion request
async fn handle_async_chat_completion(
    state: AppState,
    request: ChatCompletionRequest,
    llm_request: LlmRequest,
    request_id: String,
) -> Result<Response, ApiError> {
    let model = request.model.clone();

    // Create pending operation
    let operation = state
        .operation_service
        .create_pending(
            OperationType::ChatCompletion,
            serde_json::to_value(&request).unwrap_or(json!({})),
            json!({ "model": &model, "request_id": &request_id }),
        )
        .await
        .map_err(ApiError::from)?;

    let operation_id = operation.id().to_string();
    info!(
        operation_id = %operation_id,
        model = %model,
        "Created async chat completion operation"
    );

    // Spawn background task
    let op_id = operation_id.clone();
    tokio::spawn(async move {
        execute_async_chat_completion(state, op_id, model, llm_request, request_id).await;
    });

    // Return 202 Accepted
    Ok((
        StatusCode::ACCEPTED,
        Json(AsyncOperationCreated::pending(&operation_id)),
    )
        .into_response())
}

/// Execute chat completion in background and update operation status
async fn execute_async_chat_completion(
    state: AppState,
    operation_id: String,
    model: String,
    llm_request: LlmRequest,
    request_id: String,
) {
    // Mark as running
    if let Err(e) = state.operation_service.mark_running(&operation_id).await {
        warn!(
            operation_id = %operation_id,
            error = %e,
            "Failed to mark operation as running"
        );
        return;
    }

    // Execute chat completion
    match state.llm_provider.chat(&model, llm_request).await {
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

/// Build LLM request from API request
fn build_llm_request(
    request: &ChatCompletionRequest,
    messages: Vec<Message>,
) -> Result<LlmRequest, ApiError> {
    let mut builder = LlmRequest::builder().messages(messages);

    if let Some(temp) = request.temperature {
        if !(0.0..=2.0).contains(&temp) {
            return Err(
                ApiError::bad_request("Temperature must be between 0 and 2")
                    .with_param("temperature"),
            );
        }
        builder = builder.temperature(temp);
    }

    if let Some(top_p) = request.top_p {
        if !(0.0..=1.0).contains(&top_p) {
            return Err(
                ApiError::bad_request("top_p must be between 0 and 1").with_param("top_p"),
            );
        }
        builder = builder.top_p(top_p);
    }

    if let Some(max_tokens) = request.max_tokens {
        builder = builder.max_tokens(max_tokens);
    }

    if let Some(stop) = &request.stop {
        builder = builder.stop(stop.to_vec());
    }

    if let Some(presence_penalty) = request.presence_penalty {
        if !(-2.0..=2.0).contains(&presence_penalty) {
            return Err(
                ApiError::bad_request("presence_penalty must be between -2 and 2")
                    .with_param("presence_penalty"),
            );
        }
        builder = builder.presence_penalty(presence_penalty);
    }

    if let Some(frequency_penalty) = request.frequency_penalty {
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

/// Create streaming response
async fn create_stream_response(
    state: AppState,
    request: LlmRequest,
    model: String,
    request_id: String,
) -> impl Stream<Item = Result<Event, std::convert::Infallible>> {
    let (tx, rx) = tokio::sync::mpsc::channel::<Result<Event, std::convert::Infallible>>(32);

    tokio::spawn(async move {
        // Send initial chunk with role
        let initial = ChatCompletionStreamResponse::initial(&model, &request_id);
        let _ = tx
            .send(Ok(Event::default().data(serde_json::to_string(&initial).unwrap())))
            .await;

        // Get streaming response from provider
        match state.llm_provider.chat_stream(&model, request).await {
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
            }
        }
    });

    ReceiverStream::new(rx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::types::ChatMessageRole;

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
        let result = build_llm_request(&request, messages);
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
        let result = build_llm_request(&request, messages);
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
        let result = build_llm_request(&request, messages);
        assert!(result.is_ok());
    }
}
