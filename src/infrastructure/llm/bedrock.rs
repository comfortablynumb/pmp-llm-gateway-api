//! AWS Bedrock LLM provider implementation

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::domain::{
    DomainError, FinishReason, LlmProvider, LlmRequest, LlmResponse, LlmStream, Message,
    MessageRole, Usage,
};

/// AWS Bedrock client trait for dependency injection
#[async_trait]
pub trait BedrockClientTrait: Send + Sync + std::fmt::Debug {
    async fn invoke_model(
        &self,
        model_id: &str,
        body: Vec<u8>,
    ) -> Result<Vec<u8>, DomainError>;
}

/// AWS Bedrock API provider
#[derive(Debug)]
pub struct BedrockProvider<C: BedrockClientTrait> {
    client: C,
}

impl<C: BedrockClientTrait> BedrockProvider<C> {
    pub fn new(client: C) -> Self {
        Self { client }
    }

    fn build_claude_request(&self, request: &LlmRequest) -> serde_json::Value {
        let (system, messages) = split_system_messages(&request.messages);

        let anthropic_messages: Vec<BedrockMessage> = messages
            .iter()
            .map(|m| BedrockMessage::from_domain(m))
            .collect();

        let mut body = serde_json::json!({
            "anthropic_version": "bedrock-2023-05-31",
            "messages": anthropic_messages,
            "max_tokens": request.max_tokens.unwrap_or(4096),
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

    fn build_titan_request(&self, request: &LlmRequest) -> serde_json::Value {
        let prompt = build_titan_prompt(&request.messages);

        let mut config = serde_json::json!({
            "maxTokenCount": request.max_tokens.unwrap_or(4096),
        });

        if let Some(temp) = request.temperature {
            config["temperature"] = serde_json::json!(temp);
        }

        if let Some(top_p) = request.top_p {
            config["topP"] = serde_json::json!(top_p);
        }

        if let Some(ref stop) = request.stop {
            config["stopSequences"] = serde_json::json!(stop);
        }

        serde_json::json!({
            "inputText": prompt,
            "textGenerationConfig": config
        })
    }

    fn parse_claude_response(
        &self,
        model: &str,
        bytes: &[u8],
    ) -> Result<LlmResponse, DomainError> {
        let response: BedrockClaudeResponse =
            serde_json::from_slice(bytes).map_err(|e| {
                DomainError::provider("bedrock", format!("Failed to parse response: {}", e))
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
        let id = format!("bedrock-{}", uuid::Uuid::new_v4());

        let mut llm_response = LlmResponse::new(id, model.to_string(), message);
        llm_response = llm_response.with_finish_reason(parse_claude_stop_reason(&response.stop_reason));
        llm_response = llm_response.with_usage(Usage::new(
            response.usage.input_tokens,
            response.usage.output_tokens,
        ));

        Ok(llm_response)
    }

    fn parse_titan_response(
        &self,
        model: &str,
        bytes: &[u8],
    ) -> Result<LlmResponse, DomainError> {
        let response: BedrockTitanResponse =
            serde_json::from_slice(bytes).map_err(|e| {
                DomainError::provider("bedrock", format!("Failed to parse response: {}", e))
            })?;

        let content = response
            .results
            .into_iter()
            .next()
            .map(|r| r.output_text)
            .unwrap_or_default();

        let message = Message::assistant(content);
        let id = format!("bedrock-{}", uuid::Uuid::new_v4());

        let mut llm_response = LlmResponse::new(id, model.to_string(), message);
        llm_response = llm_response.with_finish_reason(FinishReason::Stop);

        if let Some(usage) = response.usage {
            llm_response = llm_response.with_usage(Usage::new(
                usage.input_token_count,
                usage.output_token_count,
            ));
        }

        Ok(llm_response)
    }

    fn is_claude_model(&self, model: &str) -> bool {
        model.contains("anthropic") || model.contains("claude")
    }
}

#[async_trait]
impl<C: BedrockClientTrait> LlmProvider for BedrockProvider<C> {
    async fn chat(&self, model: &str, request: LlmRequest) -> Result<LlmResponse, DomainError> {
        let body = if self.is_claude_model(model) {
            self.build_claude_request(&request)
        } else {
            self.build_titan_request(&request)
        };

        let body_bytes = serde_json::to_vec(&body).map_err(|e| {
            DomainError::provider("bedrock", format!("Failed to serialize request: {}", e))
        })?;

        let response_bytes = self.client.invoke_model(model, body_bytes).await?;

        if self.is_claude_model(model) {
            self.parse_claude_response(model, &response_bytes)
        } else {
            self.parse_titan_response(model, &response_bytes)
        }
    }

    async fn chat_stream(
        &self,
        model: &str,
        _request: LlmRequest,
    ) -> Result<LlmStream, DomainError> {
        // Bedrock streaming requires InvokeModelWithResponseStream
        // For now, return an error indicating streaming is not yet implemented
        Err(DomainError::provider(
            "bedrock",
            format!("Streaming not yet implemented for model: {}", model),
        ))
    }

    fn provider_name(&self) -> &'static str {
        "bedrock"
    }

    fn available_models(&self) -> Vec<&'static str> {
        vec![
            "anthropic.claude-3-5-sonnet-20241022-v2:0",
            "anthropic.claude-3-5-haiku-20241022-v1:0",
            "anthropic.claude-3-opus-20240229-v1:0",
            "anthropic.claude-3-sonnet-20240229-v1:0",
            "anthropic.claude-3-haiku-20240307-v1:0",
            "amazon.titan-text-express-v1",
            "amazon.titan-text-lite-v1",
        ]
    }
}

fn split_system_messages(messages: &[Message]) -> (Option<String>, Vec<&Message>) {
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

fn build_titan_prompt(messages: &[Message]) -> String {
    let mut prompt = String::new();

    for msg in messages {
        let prefix = match msg.role {
            MessageRole::System => "System: ",
            MessageRole::User => "User: ",
            MessageRole::Assistant => "Bot: ",
        };

        if let Some(text) = msg.content_text() {
            prompt.push_str(prefix);
            prompt.push_str(text);
            prompt.push('\n');
        }
    }

    prompt
}

fn parse_claude_stop_reason(reason: &Option<String>) -> FinishReason {
    match reason.as_deref() {
        Some("end_turn") | Some("stop_sequence") => FinishReason::Stop,
        Some("max_tokens") => FinishReason::Length,
        Some("tool_use") => FinishReason::ToolCalls,
        _ => FinishReason::Stop,
    }
}

// Bedrock API types

#[derive(Debug, Serialize)]
struct BedrockMessage {
    role: String,
    content: String,
}

impl BedrockMessage {
    fn from_domain(message: &Message) -> Self {
        let role = match message.role {
            MessageRole::User => "user",
            MessageRole::Assistant => "assistant",
            MessageRole::System => "user",
        };

        Self {
            role: role.to_string(),
            content: message.content_text().unwrap_or("").to_string(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct BedrockClaudeResponse {
    content: Vec<ContentBlock>,
    stop_reason: Option<String>,
    usage: ClaudeUsage,
}

#[derive(Debug, Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    content_type: String,
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ClaudeUsage {
    input_tokens: u32,
    output_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct BedrockTitanResponse {
    results: Vec<TitanResult>,
    #[serde(rename = "inputTextTokenCount")]
    #[allow(dead_code)]
    input_text_token_count: Option<u32>,
    usage: Option<TitanUsage>,
}

#[derive(Debug, Deserialize)]
struct TitanResult {
    #[serde(rename = "outputText")]
    output_text: String,
}

#[derive(Debug, Deserialize)]
struct TitanUsage {
    #[serde(rename = "inputTokenCount")]
    input_token_count: u32,
    #[serde(rename = "outputTokenCount")]
    output_token_count: u32,
}

/// Real AWS Bedrock client implementation
#[derive(Debug, Clone)]
pub struct BedrockClient {
    client: aws_sdk_bedrockruntime::Client,
}

impl BedrockClient {
    pub async fn new(config: &aws_config::SdkConfig) -> Self {
        let client = aws_sdk_bedrockruntime::Client::new(config);
        Self { client }
    }

    pub fn from_client(client: aws_sdk_bedrockruntime::Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl BedrockClientTrait for BedrockClient {
    async fn invoke_model(
        &self,
        model_id: &str,
        body: Vec<u8>,
    ) -> Result<Vec<u8>, DomainError> {
        let blob = aws_sdk_bedrockruntime::primitives::Blob::new(body);

        let response = self
            .client
            .invoke_model()
            .model_id(model_id)
            .body(blob)
            .content_type("application/json")
            .send()
            .await
            .map_err(|e| DomainError::provider("bedrock", format!("API error: {}", e)))?;

        Ok(response.body.into_inner())
    }
}

#[cfg(test)]
pub mod mock {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    #[derive(Debug, Default)]
    pub struct MockBedrockClient {
        responses: Mutex<HashMap<String, Vec<u8>>>,
        errors: Mutex<HashMap<String, String>>,
    }

    impl MockBedrockClient {
        pub fn new() -> Self {
            Self::default()
        }

        pub fn with_response(self, model_id: &str, response: serde_json::Value) -> Self {
            let bytes = serde_json::to_vec(&response).unwrap();
            self.responses
                .lock()
                .unwrap()
                .insert(model_id.to_string(), bytes);
            self
        }

        pub fn with_error(self, model_id: &str, error: &str) -> Self {
            self.errors
                .lock()
                .unwrap()
                .insert(model_id.to_string(), error.to_string());
            self
        }
    }

    #[async_trait]
    impl BedrockClientTrait for MockBedrockClient {
        async fn invoke_model(
            &self,
            model_id: &str,
            _body: Vec<u8>,
        ) -> Result<Vec<u8>, DomainError> {
            if let Some(error) = self.errors.lock().unwrap().get(model_id) {
                return Err(DomainError::provider("bedrock", error.clone()));
            }

            self.responses
                .lock()
                .unwrap()
                .get(model_id)
                .cloned()
                .ok_or_else(|| {
                    DomainError::provider("bedrock", format!("No mock response for {}", model_id))
                })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mock::MockBedrockClient;

    #[tokio::test]
    async fn test_bedrock_claude_chat() {
        let mock_response = serde_json::json!({
            "id": "msg_123",
            "type": "message",
            "role": "assistant",
            "content": [{
                "type": "text",
                "text": "Hello from Bedrock Claude!"
            }],
            "stop_reason": "end_turn",
            "usage": {
                "input_tokens": 15,
                "output_tokens": 8
            }
        });

        let model_id = "anthropic.claude-3-sonnet-20240229-v1:0";
        let client = MockBedrockClient::new().with_response(model_id, mock_response);
        let provider = BedrockProvider::new(client);

        let request = LlmRequest::builder()
            .system("You are helpful")
            .user("Hello!")
            .build();

        let response = provider.chat(model_id, request).await.unwrap();

        assert_eq!(response.content(), Some("Hello from Bedrock Claude!"));
        assert_eq!(response.finish_reason, Some(FinishReason::Stop));

        let usage = response.usage.unwrap();
        assert_eq!(usage.prompt_tokens, 15);
        assert_eq!(usage.completion_tokens, 8);
    }

    #[tokio::test]
    async fn test_bedrock_titan_chat() {
        let mock_response = serde_json::json!({
            "results": [{
                "outputText": "Hello from Titan!",
                "completionReason": "FINISH"
            }],
            "inputTextTokenCount": 10,
            "usage": {
                "inputTokenCount": 10,
                "outputTokenCount": 5
            }
        });

        let model_id = "amazon.titan-text-express-v1";
        let client = MockBedrockClient::new().with_response(model_id, mock_response);
        let provider = BedrockProvider::new(client);

        let request = LlmRequest::builder().user("Hello!").build();

        let response = provider.chat(model_id, request).await.unwrap();

        assert_eq!(response.content(), Some("Hello from Titan!"));

        let usage = response.usage.unwrap();
        assert_eq!(usage.prompt_tokens, 10);
        assert_eq!(usage.completion_tokens, 5);
    }

    #[tokio::test]
    async fn test_bedrock_error_handling() {
        let model_id = "anthropic.claude-3-sonnet-20240229-v1:0";
        let client = MockBedrockClient::new().with_error(model_id, "Access denied");
        let provider = BedrockProvider::new(client);

        let request = LlmRequest::builder().user("Hello!").build();

        let result = provider.chat(model_id, request).await;
        assert!(result.is_err());
    }
}
