use serde::{Deserialize, Serialize};

use super::Message;

/// Reason why the generation finished
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FinishReason {
    Stop,
    Length,
    ContentFilter,
    ToolCalls,
    Error,
}

/// Token usage statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

impl Usage {
    pub fn new(prompt_tokens: u32, completion_tokens: u32) -> Self {
        Self {
            prompt_tokens,
            completion_tokens,
            total_tokens: prompt_tokens + completion_tokens,
        }
    }
}

/// Response from an LLM provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmResponse {
    pub id: String,
    pub model: String,
    pub message: Message,
    pub finish_reason: Option<FinishReason>,
    pub usage: Option<Usage>,
}

impl LlmResponse {
    pub fn new(id: String, model: String, message: Message) -> Self {
        Self {
            id,
            model,
            message,
            finish_reason: None,
            usage: None,
        }
    }

    pub fn with_finish_reason(mut self, reason: FinishReason) -> Self {
        self.finish_reason = Some(reason);
        self
    }

    pub fn with_usage(mut self, usage: Usage) -> Self {
        self.usage = Some(usage);
        self
    }

    pub fn content(&self) -> Option<&str> {
        self.message.content_text()
    }
}

/// Streaming chunk from an LLM provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChunk {
    pub id: String,
    pub model: String,
    pub delta: Option<String>,
    pub finish_reason: Option<FinishReason>,
    pub usage: Option<Usage>,
}

impl StreamChunk {
    pub fn new(id: String, model: String) -> Self {
        Self {
            id,
            model,
            delta: None,
            finish_reason: None,
            usage: None,
        }
    }

    pub fn with_delta(mut self, delta: impl Into<String>) -> Self {
        self.delta = Some(delta.into());
        self
    }

    pub fn with_finish_reason(mut self, reason: FinishReason) -> Self {
        self.finish_reason = Some(reason);
        self
    }

    pub fn with_usage(mut self, usage: Usage) -> Self {
        self.usage = Some(usage);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_usage_calculation() {
        let usage = Usage::new(10, 20);
        assert_eq!(usage.total_tokens, 30);
    }

    #[test]
    fn test_response_content() {
        let response = LlmResponse::new(
            "id-123".to_string(),
            "gpt-4".to_string(),
            Message::assistant("Hello!"),
        );

        assert_eq!(response.content(), Some("Hello!"));
    }
}
