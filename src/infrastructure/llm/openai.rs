use async_trait::async_trait;
use bytes::Bytes;
use futures::StreamExt;
use serde::{Deserialize, Serialize};

use super::http_client::HttpClientTrait;
use crate::domain::{
    DomainError, FinishReason, LlmProvider, LlmRequest, LlmResponse, LlmStream, Message,
    MessageRole, StreamChunk, Usage,
};
use crate::domain::llm::LlmResponseFormat;

const DEFAULT_OPENAI_BASE_URL: &str = "https://api.openai.com";

/// OpenAI API provider
#[derive(Debug)]
pub struct OpenAiProvider<C: HttpClientTrait> {
    client: C,
    #[allow(dead_code)]
    api_key: String,
    auth_header: String,
    base_url: String,
}

impl<C: HttpClientTrait> OpenAiProvider<C> {
    pub fn new(client: C, api_key: impl Into<String>) -> Self {
        Self::with_base_url(client, api_key, DEFAULT_OPENAI_BASE_URL)
    }

    pub fn with_base_url(
        client: C,
        api_key: impl Into<String>,
        base_url: impl Into<String>,
    ) -> Self {
        let api_key = api_key.into();
        let auth_header = format!("Bearer {}", api_key);
        let base_url = base_url.into().trim_end_matches('/').to_string();

        Self {
            client,
            api_key,
            auth_header,
            base_url,
        }
    }

    fn chat_completions_url(&self) -> String {
        format!("{}/v1/chat/completions", self.base_url)
    }

    fn build_request(&self, model: &str, request: &LlmRequest) -> serde_json::Value {
        let messages: Vec<OpenAiMessage> = request
            .messages
            .iter()
            .map(|m| OpenAiMessage::from_domain(m))
            .collect();

        let mut body = serde_json::json!({
            "model": model,
            "messages": messages,
            "stream": request.stream,
        });

        if let Some(temp) = request.temperature {
            body["temperature"] = serde_json::json!(temp);
        }

        if let Some(max_tokens) = request.max_tokens {
            body["max_tokens"] = serde_json::json!(max_tokens);
        }

        if let Some(top_p) = request.top_p {
            body["top_p"] = serde_json::json!(top_p);
        }

        if let Some(ref stop) = request.stop {
            body["stop"] = serde_json::json!(stop);
        }

        if let Some(presence_penalty) = request.presence_penalty {
            body["presence_penalty"] = serde_json::json!(presence_penalty);
        }

        if let Some(frequency_penalty) = request.frequency_penalty {
            body["frequency_penalty"] = serde_json::json!(frequency_penalty);
        }

        // Add response_format for structured outputs
        if let Some(ref response_format) = request.response_format {
            match response_format {
                LlmResponseFormat::Text => {
                    body["response_format"] = serde_json::json!({"type": "text"});
                }
                LlmResponseFormat::JsonObject => {
                    body["response_format"] = serde_json::json!({"type": "json_object"});
                }
                LlmResponseFormat::JsonSchema { json_schema } => {
                    body["response_format"] = serde_json::json!({
                        "type": "json_schema",
                        "json_schema": {
                            "name": json_schema.name,
                            "strict": json_schema.strict,
                            "schema": json_schema.schema
                        }
                    });
                }
            }
        }

        body
    }

    fn headers(&self) -> Vec<(&str, &str)> {
        vec![
            ("Authorization", self.auth_header.as_str()),
            ("Content-Type", "application/json"),
        ]
    }

    fn parse_response(&self, json: serde_json::Value) -> Result<LlmResponse, DomainError> {
        let response: OpenAiResponse = serde_json::from_value(json).map_err(|e| {
            DomainError::provider("openai", format!("Failed to parse response: {}", e))
        })?;

        let choice = response
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| DomainError::provider("openai", "No choices in response"))?;

        let message = Message::assistant(choice.message.content.unwrap_or_default());

        let mut llm_response = LlmResponse::new(response.id, response.model, message);

        if let Some(reason) = choice.finish_reason {
            llm_response = llm_response.with_finish_reason(parse_finish_reason(&reason));
        }

        if let Some(usage) = response.usage {
            llm_response = llm_response.with_usage(Usage::new(
                usage.prompt_tokens,
                usage.completion_tokens,
            ));
        }

        Ok(llm_response)
    }
}

#[async_trait]
impl<C: HttpClientTrait> LlmProvider for OpenAiProvider<C> {
    async fn chat(&self, model: &str, request: LlmRequest) -> Result<LlmResponse, DomainError> {
        let mut req = request;
        req.stream = false;

        let url = self.chat_completions_url();
        let body = self.build_request(model, &req);
        let response = self
            .client
            .post_json(&url, self.headers(), &body)
            .await?;

        self.parse_response(response)
    }

    async fn chat_stream(
        &self,
        model: &str,
        request: LlmRequest,
    ) -> Result<LlmStream, DomainError> {
        let mut req = request;
        req.stream = true;

        let url = self.chat_completions_url();
        let body = self.build_request(model, &req);
        let byte_stream = self
            .client
            .post_json_stream(&url, self.headers(), &body)
            .await?;

        let model_clone = model.to_string();
        let stream = byte_stream.filter_map(move |result: Result<Bytes, DomainError>| {
            let model = model_clone.clone();
            async move {
                match result {
                    Ok(bytes) => {
                        let text = String::from_utf8_lossy(&bytes);
                        parse_sse_chunks(&text, &model)
                    }
                    Err(e) => Some(Err(e)),
                }
            }
        });

        Ok(Box::pin(stream))
    }

    fn provider_name(&self) -> &'static str {
        "openai"
    }

    fn available_models(&self) -> Vec<&'static str> {
        vec![
            "gpt-4o",
            "gpt-4o-mini",
            "gpt-4-turbo",
            "gpt-4",
            "gpt-3.5-turbo",
        ]
    }
}

fn parse_sse_chunks(text: &str, model: &str) -> Option<Result<StreamChunk, DomainError>> {
    for line in text.lines() {
        if let Some(data) = line.strip_prefix("data: ") {
            if data.trim() == "[DONE]" {
                return Some(Ok(StreamChunk::new("".to_string(), model.to_string())
                    .with_finish_reason(FinishReason::Stop)));
            }

            if let Ok(chunk) = serde_json::from_str::<OpenAiStreamChunk>(data) {
                if let Some(choice) = chunk.choices.into_iter().next() {
                    let mut stream_chunk =
                        StreamChunk::new(chunk.id, chunk.model.unwrap_or_default());

                    if let Some(delta) = choice.delta.content {
                        stream_chunk = stream_chunk.with_delta(delta);
                    }

                    if let Some(reason) = choice.finish_reason {
                        stream_chunk =
                            stream_chunk.with_finish_reason(parse_finish_reason(&reason));
                    }

                    return Some(Ok(stream_chunk));
                }
            }
        }
    }
    None
}

fn parse_finish_reason(reason: &str) -> FinishReason {
    match reason {
        "stop" => FinishReason::Stop,
        "length" => FinishReason::Length,
        "content_filter" => FinishReason::ContentFilter,
        "tool_calls" | "function_call" => FinishReason::ToolCalls,
        _ => FinishReason::Stop,
    }
}

// OpenAI API types

#[derive(Debug, Serialize)]
struct OpenAiMessage {
    role: String,
    content: String,
}

impl OpenAiMessage {
    fn from_domain(message: &Message) -> Self {
        let role = match message.role {
            MessageRole::System => "system",
            MessageRole::User => "user",
            MessageRole::Assistant => "assistant",
        };

        Self {
            role: role.to_string(),
            content: message.content_text().unwrap_or("").to_string(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct OpenAiResponse {
    id: String,
    model: String,
    choices: Vec<OpenAiChoice>,
    usage: Option<OpenAiUsage>,
}

#[derive(Debug, Deserialize)]
struct OpenAiChoice {
    message: OpenAiResponseMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAiResponseMessage {
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAiUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct OpenAiStreamChunk {
    id: String,
    model: Option<String>,
    choices: Vec<OpenAiStreamChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenAiStreamChoice {
    delta: OpenAiDelta,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAiDelta {
    content: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::llm::http_client::mock::MockHttpClient;

    const TEST_URL: &str = "https://api.openai.com/v1/chat/completions";

    #[tokio::test]
    async fn test_openai_chat() {
        let mock_response = serde_json::json!({
            "id": "chatcmpl-123",
            "model": "gpt-4",
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "Hello! How can I help you?"
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 8,
                "total_tokens": 18
            }
        });

        let client = MockHttpClient::new().with_response(TEST_URL, mock_response);

        let provider = OpenAiProvider::new(client, "test-api-key");

        let request = LlmRequest::builder().user("Hello!").build();

        let response = provider.chat("gpt-4", request).await.unwrap();

        assert_eq!(response.id, "chatcmpl-123");
        assert_eq!(response.model, "gpt-4");
        assert_eq!(response.content(), Some("Hello! How can I help you?"));
        assert_eq!(response.finish_reason, Some(FinishReason::Stop));

        let usage = response.usage.unwrap();
        assert_eq!(usage.prompt_tokens, 10);
        assert_eq!(usage.completion_tokens, 8);
    }

    #[tokio::test]
    async fn test_openai_error_handling() {
        let client = MockHttpClient::new().with_error(TEST_URL, "API key invalid");

        let provider = OpenAiProvider::new(client, "invalid-key");

        let request = LlmRequest::builder().user("Hello!").build();

        let result = provider.chat("gpt-4", request).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_openai_custom_base_url() {
        let custom_url = "http://localhost:8080/v1/chat/completions";
        let mock_response = serde_json::json!({
            "id": "chatcmpl-custom",
            "model": "gpt-4",
            "choices": [{
                "message": { "role": "assistant", "content": "Custom response" },
                "finish_reason": "stop"
            }],
            "usage": { "prompt_tokens": 5, "completion_tokens": 2, "total_tokens": 7 }
        });

        let client = MockHttpClient::new().with_response(custom_url, mock_response);
        let provider = OpenAiProvider::with_base_url(client, "test-key", "http://localhost:8080");

        let request = LlmRequest::builder().user("Test").build();
        let response = provider.chat("gpt-4", request).await.unwrap();

        assert_eq!(response.id, "chatcmpl-custom");
    }
}
