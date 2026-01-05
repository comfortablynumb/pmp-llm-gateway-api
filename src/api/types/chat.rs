//! OpenAI-compatible chat completion types

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Role of a chat message
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChatMessageRole {
    System,
    User,
    Assistant,
    Tool,
    Function,
}

impl From<ChatMessageRole> for crate::domain::llm::MessageRole {
    fn from(role: ChatMessageRole) -> Self {
        match role {
            ChatMessageRole::System => Self::System,
            ChatMessageRole::User => Self::User,
            ChatMessageRole::Assistant => Self::Assistant,
            // Tool and Function are mapped to User for processing
            ChatMessageRole::Tool | ChatMessageRole::Function => Self::User,
        }
    }
}

impl From<crate::domain::llm::MessageRole> for ChatMessageRole {
    fn from(role: crate::domain::llm::MessageRole) -> Self {
        match role {
            crate::domain::llm::MessageRole::System => Self::System,
            crate::domain::llm::MessageRole::User => Self::User,
            crate::domain::llm::MessageRole::Assistant => Self::Assistant,
        }
    }
}

/// Content part for multimodal messages
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentPart {
    Text { text: String },
    ImageUrl { image_url: ImageUrl },
}

/// Image URL content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageUrl {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// A chat message in OpenAI format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: ChatMessageRole,

    /// Text content or array of content parts
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<MessageContent>,

    /// Name of the participant (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Tool calls made by the assistant
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,

    /// Tool call ID (for tool messages)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,

    /// Function call (deprecated, use tool_calls)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_call: Option<FunctionCall>,

    // Gateway extension: reference a prompt by ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_id: Option<String>,

    // Gateway extension: variables for prompt templating
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variables: Option<HashMap<String, String>>,
}

/// Message content - can be text or array of content parts
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    Text(String),
    Parts(Vec<ContentPart>),
}

impl MessageContent {
    /// Get the text content, concatenating parts if needed
    pub fn to_text(&self) -> String {
        match self {
            Self::Text(s) => s.clone(),
            Self::Parts(parts) => parts
                .iter()
                .filter_map(|p| match p {
                    ContentPart::Text { text } => Some(text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("\n"),
        }
    }
}

/// Tool call made by the assistant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub call_type: String,
    pub function: FunctionCall,
}

/// Function call details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

/// Stream options for chat completions
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StreamOptions {
    /// Include usage in the final stream chunk
    #[serde(default)]
    pub include_usage: bool,
}

/// Chat completion request (OpenAI format)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionRequest {
    /// Model ID (can be gateway model ID or chain ID)
    pub model: String,

    /// Messages in the conversation
    pub messages: Vec<ChatMessage>,

    /// Temperature (0.0 - 2.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,

    /// Top-p nucleus sampling
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,

    /// Number of completions to generate
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n: Option<u32>,

    /// Whether to stream responses
    #[serde(default)]
    pub stream: bool,

    /// Stream options
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_options: Option<StreamOptions>,

    /// Stop sequences
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<StopSequence>,

    /// Maximum tokens to generate
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,

    /// Presence penalty (-2.0 to 2.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f32>,

    /// Frequency penalty (-2.0 to 2.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f32>,

    /// User identifier for tracking
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,

    /// Seed for deterministic sampling
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<i64>,
}

/// Stop sequence - can be string or array
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum StopSequence {
    Single(String),
    Multiple(Vec<String>),
}

impl StopSequence {
    pub fn to_vec(&self) -> Vec<String> {
        match self {
            Self::Single(s) => vec![s.clone()],
            Self::Multiple(v) => v.clone(),
        }
    }
}

/// Reason for completion finish
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FinishReason {
    Stop,
    Length,
    ToolCalls,
    ContentFilter,
    FunctionCall,
}

/// Token usage statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

impl From<crate::domain::llm::Usage> for Usage {
    fn from(usage: crate::domain::llm::Usage) -> Self {
        Self {
            prompt_tokens: usage.prompt_tokens,
            completion_tokens: usage.completion_tokens,
            total_tokens: usage.total_tokens,
        }
    }
}

/// A choice in the chat completion response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionChoice {
    pub index: u32,
    pub message: ChatMessage,
    pub finish_reason: Option<FinishReason>,
}

/// Chat completion response (OpenAI format)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionResponse {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub model: String,
    pub choices: Vec<ChatCompletionChoice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<Usage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_fingerprint: Option<String>,
}

impl ChatCompletionResponse {
    /// Create a new response from an LLM response
    pub fn from_llm_response(
        response: &crate::domain::llm::LlmResponse,
        model: &str,
        request_id: &str,
    ) -> Self {
        let content_text = response.content().unwrap_or("").to_string();

        Self {
            id: format!("chatcmpl-{}", request_id),
            object: "chat.completion".to_string(),
            created: chrono::Utc::now().timestamp(),
            model: model.to_string(),
            choices: vec![ChatCompletionChoice {
                index: 0,
                message: ChatMessage {
                    role: ChatMessageRole::Assistant,
                    content: Some(MessageContent::Text(content_text)),
                    name: None,
                    tool_calls: None,
                    tool_call_id: None,
                    function_call: None,
                    prompt_id: None,
                    variables: None,
                },
                finish_reason: Some(FinishReason::Stop),
            }],
            usage: response.usage.as_ref().map(|u| Usage::from(u.clone())),
            system_fingerprint: None,
        }
    }
}

/// Delta content for streaming
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DeltaContent {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<ChatMessageRole>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
}

/// A choice in a streaming response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionStreamChoice {
    pub index: u32,
    pub delta: DeltaContent,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<FinishReason>,
}

/// Streaming chat completion response chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionStreamResponse {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub model: String,
    pub choices: Vec<ChatCompletionStreamChoice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<Usage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_fingerprint: Option<String>,
}

impl ChatCompletionStreamResponse {
    /// Create an initial chunk with role
    pub fn initial(model: &str, request_id: &str) -> Self {
        Self {
            id: format!("chatcmpl-{}", request_id),
            object: "chat.completion.chunk".to_string(),
            created: chrono::Utc::now().timestamp(),
            model: model.to_string(),
            choices: vec![ChatCompletionStreamChoice {
                index: 0,
                delta: DeltaContent {
                    role: Some(ChatMessageRole::Assistant),
                    content: None,
                    tool_calls: None,
                },
                finish_reason: None,
            }],
            usage: None,
            system_fingerprint: None,
        }
    }

    /// Create a content chunk
    pub fn content(model: &str, request_id: &str, content: &str) -> Self {
        Self {
            id: format!("chatcmpl-{}", request_id),
            object: "chat.completion.chunk".to_string(),
            created: chrono::Utc::now().timestamp(),
            model: model.to_string(),
            choices: vec![ChatCompletionStreamChoice {
                index: 0,
                delta: DeltaContent {
                    role: None,
                    content: Some(content.to_string()),
                    tool_calls: None,
                },
                finish_reason: None,
            }],
            usage: None,
            system_fingerprint: None,
        }
    }

    /// Create a final chunk with finish reason
    pub fn finish(model: &str, request_id: &str, usage: Option<Usage>) -> Self {
        Self {
            id: format!("chatcmpl-{}", request_id),
            object: "chat.completion.chunk".to_string(),
            created: chrono::Utc::now().timestamp(),
            model: model.to_string(),
            choices: vec![ChatCompletionStreamChoice {
                index: 0,
                delta: DeltaContent::default(),
                finish_reason: Some(FinishReason::Stop),
            }],
            usage,
            system_fingerprint: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_message_role_conversion() {
        assert_eq!(
            crate::domain::llm::MessageRole::from(ChatMessageRole::User),
            crate::domain::llm::MessageRole::User
        );
        assert_eq!(
            ChatMessageRole::from(crate::domain::llm::MessageRole::Assistant),
            ChatMessageRole::Assistant
        );
    }

    #[test]
    fn test_message_content_to_text() {
        let text = MessageContent::Text("Hello".to_string());
        assert_eq!(text.to_text(), "Hello");

        let parts = MessageContent::Parts(vec![
            ContentPart::Text {
                text: "Hello".to_string(),
            },
            ContentPart::Text {
                text: "World".to_string(),
            },
        ]);
        assert_eq!(parts.to_text(), "Hello\nWorld");
    }

    #[test]
    fn test_stop_sequence_to_vec() {
        let single = StopSequence::Single("stop".to_string());
        assert_eq!(single.to_vec(), vec!["stop"]);

        let multiple = StopSequence::Multiple(vec!["a".to_string(), "b".to_string()]);
        assert_eq!(multiple.to_vec(), vec!["a", "b"]);
    }

    #[test]
    fn test_chat_request_deserialization() {
        let json = r#"{
            "model": "gpt-4",
            "messages": [
                {"role": "user", "content": "Hello"}
            ],
            "stream": false
        }"#;

        let request: ChatCompletionRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.model, "gpt-4");
        assert_eq!(request.messages.len(), 1);
        assert!(!request.stream);
    }

    #[test]
    fn test_chat_response_serialization() {
        let response = ChatCompletionResponse {
            id: "chatcmpl-123".to_string(),
            object: "chat.completion".to_string(),
            created: 1234567890,
            model: "gpt-4".to_string(),
            choices: vec![ChatCompletionChoice {
                index: 0,
                message: ChatMessage {
                    role: ChatMessageRole::Assistant,
                    content: Some(MessageContent::Text("Hello!".to_string())),
                    name: None,
                    tool_calls: None,
                    tool_call_id: None,
                    function_call: None,
                    prompt_id: None,
                    variables: None,
                },
                finish_reason: Some(FinishReason::Stop),
            }],
            usage: Some(Usage {
                prompt_tokens: 10,
                completion_tokens: 5,
                total_tokens: 15,
            }),
            system_fingerprint: None,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("chatcmpl-123"));
        assert!(json.contains("Hello!"));
    }

    #[test]
    fn test_stream_response_chunks() {
        let initial = ChatCompletionStreamResponse::initial("gpt-4", "123");
        assert!(initial.choices[0].delta.role.is_some());
        assert!(initial.choices[0].delta.content.is_none());

        let content = ChatCompletionStreamResponse::content("gpt-4", "123", "Hello");
        assert!(content.choices[0].delta.role.is_none());
        assert_eq!(content.choices[0].delta.content, Some("Hello".to_string()));

        let finish = ChatCompletionStreamResponse::finish("gpt-4", "123", None);
        assert_eq!(
            finish.choices[0].finish_reason,
            Some(FinishReason::Stop)
        );
    }
}
