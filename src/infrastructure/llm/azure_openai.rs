use async_trait::async_trait;
use bytes::Bytes;
use futures::StreamExt;
use serde::{Deserialize, Serialize};

use super::http_client::HttpClientTrait;
use crate::domain::{
    DomainError, FinishReason, LlmProvider, LlmRequest, LlmResponse, LlmStream, Message,
    MessageRole, StreamChunk, Usage,
};

/// Azure OpenAI API configuration
#[derive(Debug, Clone)]
pub struct AzureOpenAiConfig {
    pub endpoint: String,
    pub api_key: String,
    pub api_version: String,
}

impl AzureOpenAiConfig {
    pub fn new(endpoint: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            api_key: api_key.into(),
            api_version: "2024-02-01".to_string(),
        }
    }

    pub fn with_api_version(mut self, version: impl Into<String>) -> Self {
        self.api_version = version.into();
        self
    }
}

/// Azure OpenAI API provider
#[derive(Debug)]
pub struct AzureOpenAiProvider<C: HttpClientTrait> {
    client: C,
    config: AzureOpenAiConfig,
}

impl<C: HttpClientTrait> AzureOpenAiProvider<C> {
    pub fn new(client: C, config: AzureOpenAiConfig) -> Self {
        Self { client, config }
    }

    fn build_url(&self, deployment: &str) -> String {
        format!(
            "{}/openai/deployments/{}/chat/completions?api-version={}",
            self.config.endpoint.trim_end_matches('/'),
            deployment,
            self.config.api_version
        )
    }

    fn build_request(&self, request: &LlmRequest) -> serde_json::Value {
        let messages: Vec<AzureMessage> = request
            .messages
            .iter()
            .map(|m| AzureMessage::from_domain(m))
            .collect();

        let mut body = serde_json::json!({
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

        body
    }

    fn headers(&self) -> Vec<(&str, &str)> {
        vec![
            ("api-key", self.config.api_key.as_str()),
            ("Content-Type", "application/json"),
        ]
    }

    fn parse_response(&self, json: serde_json::Value) -> Result<LlmResponse, DomainError> {
        let response: AzureResponse = serde_json::from_value(json).map_err(|e| {
            DomainError::provider("azure_openai", format!("Failed to parse response: {}", e))
        })?;

        let choice = response
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| DomainError::provider("azure_openai", "No choices in response"))?;

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
impl<C: HttpClientTrait> LlmProvider for AzureOpenAiProvider<C> {
    async fn chat(&self, model: &str, request: LlmRequest) -> Result<LlmResponse, DomainError> {
        let mut req = request;
        req.stream = false;

        let url = self.build_url(model);
        let body = self.build_request(&req);

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

        let url = self.build_url(model);
        let body = self.build_request(&req);

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
        "azure_openai"
    }

    fn available_models(&self) -> Vec<&'static str> {
        // Azure uses deployment names, not model names
        vec![]
    }
}

fn parse_sse_chunks(text: &str, model: &str) -> Option<Result<StreamChunk, DomainError>> {
    for line in text.lines() {
        if let Some(data) = line.strip_prefix("data: ") {
            if data.trim() == "[DONE]" {
                return Some(Ok(StreamChunk::new("".to_string(), model.to_string())
                    .with_finish_reason(FinishReason::Stop)));
            }

            if let Ok(chunk) = serde_json::from_str::<AzureStreamChunk>(data) {
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

// Azure OpenAI API types (same structure as OpenAI)

#[derive(Debug, Serialize)]
struct AzureMessage {
    role: String,
    content: String,
}

impl AzureMessage {
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
struct AzureResponse {
    id: String,
    model: String,
    choices: Vec<AzureChoice>,
    usage: Option<AzureUsage>,
}

#[derive(Debug, Deserialize)]
struct AzureChoice {
    message: AzureResponseMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AzureResponseMessage {
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AzureUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct AzureStreamChunk {
    id: String,
    model: Option<String>,
    choices: Vec<AzureStreamChoice>,
}

#[derive(Debug, Deserialize)]
struct AzureStreamChoice {
    delta: AzureDelta,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AzureDelta {
    content: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::llm::http_client::mock::MockHttpClient;

    #[tokio::test]
    async fn test_azure_openai_chat() {
        let mock_response = serde_json::json!({
            "id": "chatcmpl-123",
            "model": "gpt-4",
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "Hello from Azure!"
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 5,
                "total_tokens": 15
            }
        });

        let url = "https://myresource.openai.azure.com/openai/deployments/gpt-4/chat/completions?api-version=2024-02-01";
        let client = MockHttpClient::new().with_response(url, mock_response);

        let config =
            AzureOpenAiConfig::new("https://myresource.openai.azure.com", "test-api-key");

        let provider = AzureOpenAiProvider::new(client, config);

        let request = LlmRequest::builder().user("Hello!").build();

        let response = provider.chat("gpt-4", request).await.unwrap();

        assert_eq!(response.content(), Some("Hello from Azure!"));
        assert_eq!(response.finish_reason, Some(FinishReason::Stop));
    }

    #[tokio::test]
    async fn test_azure_openai_url_building() {
        let config = AzureOpenAiConfig::new("https://myresource.openai.azure.com/", "key")
            .with_api_version("2024-06-01");

        let client = MockHttpClient::new();
        let provider = AzureOpenAiProvider::new(client, config);

        let url = provider.build_url("my-deployment");
        assert_eq!(
            url,
            "https://myresource.openai.azure.com/openai/deployments/my-deployment/chat/completions?api-version=2024-06-01"
        );
    }
}
