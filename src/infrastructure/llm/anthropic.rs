use async_trait::async_trait;
use bytes::Bytes;
use futures::StreamExt;
use serde::{Deserialize, Serialize};

use super::http_client::HttpClientTrait;
use crate::domain::{
    DomainError, FinishReason, LlmProvider, LlmRequest, LlmResponse, LlmStream, Message,
    MessageRole, StreamChunk, Usage,
};

const DEFAULT_ANTHROPIC_BASE_URL: &str = "https://api.anthropic.com";
const ANTHROPIC_VERSION: &str = "2023-06-01";

/// Anthropic API provider
#[derive(Debug)]
pub struct AnthropicProvider<C: HttpClientTrait> {
    client: C,
    api_key: String,
    base_url: String,
}

impl<C: HttpClientTrait> AnthropicProvider<C> {
    pub fn new(client: C, api_key: impl Into<String>) -> Self {
        Self::with_base_url(client, api_key, DEFAULT_ANTHROPIC_BASE_URL)
    }

    pub fn with_base_url(
        client: C,
        api_key: impl Into<String>,
        base_url: impl Into<String>,
    ) -> Self {
        let base_url = base_url.into().trim_end_matches('/').to_string();

        Self {
            client,
            api_key: api_key.into(),
            base_url,
        }
    }

    fn messages_url(&self) -> String {
        format!("{}/v1/messages", self.base_url)
    }

    fn build_request(&self, model: &str, request: &LlmRequest) -> serde_json::Value {
        let (system, messages) = self.split_system_messages(&request.messages);

        let anthropic_messages: Vec<AnthropicMessage> = messages
            .iter()
            .map(|m| AnthropicMessage::from_domain(m))
            .collect();

        let mut body = serde_json::json!({
            "model": model,
            "messages": anthropic_messages,
            "max_tokens": request.max_tokens.unwrap_or(4096),
            "stream": request.stream,
        });

        if let Some(system_content) = system {
            body["system"] = serde_json::json!(system_content);
        }

        if let Some(temp) = request.temperature {
            body["temperature"] = serde_json::json!(temp);
        }

        if let Some(top_p) = request.top_p {
            body["top_p"] = serde_json::json!(top_p);
        }

        if let Some(ref stop) = request.stop {
            body["stop_sequences"] = serde_json::json!(stop);
        }

        body
    }

    fn split_system_messages<'a>(
        &self,
        messages: &'a [Message],
    ) -> (Option<String>, Vec<&'a Message>) {
        let mut system_content = String::new();
        let mut other_messages = Vec::new();

        for msg in messages {
            if msg.role == MessageRole::System {
                if !system_content.is_empty() {
                    system_content.push('\n');
                }

                if let Some(text) = msg.content_text() {
                    system_content.push_str(text);
                }
            } else {
                other_messages.push(msg);
            }
        }

        let system = if system_content.is_empty() {
            None
        } else {
            Some(system_content)
        };

        (system, other_messages)
    }

    fn headers(&self) -> Vec<(&str, &str)> {
        vec![
            ("x-api-key", self.api_key.as_str()),
            ("anthropic-version", ANTHROPIC_VERSION),
            ("Content-Type", "application/json"),
        ]
    }

    fn parse_response(&self, json: serde_json::Value) -> Result<LlmResponse, DomainError> {
        let response: AnthropicResponse = serde_json::from_value(json).map_err(|e| {
            DomainError::provider("anthropic", format!("Failed to parse response: {}", e))
        })?;

        let content = response
            .content
            .into_iter()
            .filter_map(|block| {
                if block.content_type == "text" {
                    block.text
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("");

        let message = Message::assistant(content);

        let mut llm_response = LlmResponse::new(response.id, response.model, message);

        llm_response = llm_response.with_finish_reason(parse_stop_reason(&response.stop_reason));

        llm_response = llm_response.with_usage(Usage::new(
            response.usage.input_tokens,
            response.usage.output_tokens,
        ));

        Ok(llm_response)
    }
}

#[async_trait]
impl<C: HttpClientTrait> LlmProvider for AnthropicProvider<C> {
    async fn chat(&self, model: &str, request: LlmRequest) -> Result<LlmResponse, DomainError> {
        let mut req = request;
        req.stream = false;

        let url = self.messages_url();
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

        let url = self.messages_url();
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
                        parse_sse_event(&text, &model)
                    }
                    Err(e) => Some(Err(e)),
                }
            }
        });

        Ok(Box::pin(stream))
    }

    fn provider_name(&self) -> &'static str {
        "anthropic"
    }

    fn available_models(&self) -> Vec<&'static str> {
        vec![
            "claude-opus-4-5-20251101",
            "claude-sonnet-4-20250514",
            "claude-3-5-sonnet-20241022",
            "claude-3-5-haiku-20241022",
            "claude-3-opus-20240229",
        ]
    }
}

fn parse_sse_event(text: &str, model: &str) -> Option<Result<StreamChunk, DomainError>> {
    for line in text.lines() {
        if let Some(data) = line.strip_prefix("data: ") {
            if let Ok(event) = serde_json::from_str::<AnthropicStreamEvent>(data) {
                match event.event_type.as_str() {
                    "content_block_delta" => {
                        if let Some(delta) = event.delta {
                            if delta.delta_type == "text_delta" {
                                if let Some(text) = delta.text {
                                    return Some(Ok(
                                        StreamChunk::new("".to_string(), model.to_string())
                                            .with_delta(text),
                                    ));
                                }
                            }
                        }
                    }
                    "message_stop" => {
                        return Some(Ok(
                            StreamChunk::new("".to_string(), model.to_string())
                                .with_finish_reason(FinishReason::Stop),
                        ));
                    }
                    "message_delta" => {
                        if let Some(delta) = event.delta {
                            if let Some(reason) = delta.stop_reason {
                                return Some(Ok(
                                    StreamChunk::new("".to_string(), model.to_string())
                                        .with_finish_reason(parse_stop_reason(&Some(reason))),
                                ));
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    None
}

fn parse_stop_reason(reason: &Option<String>) -> FinishReason {
    match reason.as_deref() {
        Some("end_turn") | Some("stop_sequence") => FinishReason::Stop,
        Some("max_tokens") => FinishReason::Length,
        Some("tool_use") => FinishReason::ToolCalls,
        _ => FinishReason::Stop,
    }
}

// Anthropic API types

#[derive(Debug, Serialize)]
struct AnthropicMessage {
    role: String,
    content: String,
}

impl AnthropicMessage {
    fn from_domain(message: &Message) -> Self {
        let role = match message.role {
            MessageRole::User => "user",
            MessageRole::Assistant => "assistant",
            MessageRole::System => "user", // System handled separately
        };

        Self {
            role: role.to_string(),
            content: message.content_text().unwrap_or("").to_string(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    id: String,
    model: String,
    content: Vec<ContentBlock>,
    stop_reason: Option<String>,
    usage: AnthropicUsage,
}

#[derive(Debug, Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    content_type: String,
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AnthropicUsage {
    input_tokens: u32,
    output_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct AnthropicStreamEvent {
    #[serde(rename = "type")]
    event_type: String,
    delta: Option<StreamDelta>,
}

#[derive(Debug, Deserialize)]
struct StreamDelta {
    #[serde(rename = "type", default)]
    delta_type: String,
    text: Option<String>,
    stop_reason: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::llm::http_client::mock::MockHttpClient;

    const TEST_URL: &str = "https://api.anthropic.com/v1/messages";

    #[tokio::test]
    async fn test_anthropic_chat() {
        let mock_response = serde_json::json!({
            "id": "msg_123",
            "type": "message",
            "role": "assistant",
            "model": "claude-3-5-sonnet-20241022",
            "content": [{
                "type": "text",
                "text": "Hello! How can I assist you today?"
            }],
            "stop_reason": "end_turn",
            "usage": {
                "input_tokens": 12,
                "output_tokens": 10
            }
        });

        let client = MockHttpClient::new().with_response(TEST_URL, mock_response);

        let provider = AnthropicProvider::new(client, "test-api-key");

        let request = LlmRequest::builder()
            .system("You are helpful")
            .user("Hello!")
            .build();

        let response = provider
            .chat("claude-3-5-sonnet-20241022", request)
            .await
            .unwrap();

        assert_eq!(response.id, "msg_123");
        assert_eq!(
            response.content(),
            Some("Hello! How can I assist you today?")
        );
        assert_eq!(response.finish_reason, Some(FinishReason::Stop));
    }

    #[tokio::test]
    async fn test_anthropic_system_message_handling() {
        let mock_response = serde_json::json!({
            "id": "msg_123",
            "type": "message",
            "role": "assistant",
            "model": "claude-3-5-sonnet-20241022",
            "content": [{"type": "text", "text": "Response"}],
            "stop_reason": "end_turn",
            "usage": {"input_tokens": 10, "output_tokens": 5}
        });

        let client = MockHttpClient::new().with_response(TEST_URL, mock_response);

        let provider = AnthropicProvider::new(client, "test-key");

        let request = LlmRequest::builder()
            .system("System prompt 1")
            .system("System prompt 2")
            .user("Hello")
            .build();

        let response = provider
            .chat("claude-3-5-sonnet-20241022", request)
            .await
            .unwrap();

        assert!(response.content().is_some());
    }

    #[tokio::test]
    async fn test_anthropic_custom_base_url() {
        let custom_url = "http://localhost:8081/v1/messages";
        let mock_response = serde_json::json!({
            "id": "msg_custom",
            "type": "message",
            "role": "assistant",
            "model": "claude-3-5-sonnet-20241022",
            "content": [{"type": "text", "text": "Custom response"}],
            "stop_reason": "end_turn",
            "usage": {"input_tokens": 5, "output_tokens": 2}
        });

        let client = MockHttpClient::new().with_response(custom_url, mock_response);
        let provider =
            AnthropicProvider::with_base_url(client, "test-key", "http://localhost:8081");

        let request = LlmRequest::builder().user("Test").build();
        let response = provider
            .chat("claude-3-5-sonnet-20241022", request)
            .await
            .unwrap();

        assert_eq!(response.id, "msg_custom");
    }
}
