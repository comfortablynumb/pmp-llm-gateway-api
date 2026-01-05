use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::Message;

/// Parameters for LLM generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmRequest {
    pub messages: Vec<Message>,
    /// Optional prompt ID to use as system message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt_id: Option<String>,
    /// Variables for prompt template rendering
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_variables: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f32>,
    #[serde(default)]
    pub stream: bool,
}

impl LlmRequest {
    pub fn new(messages: Vec<Message>) -> Self {
        Self {
            messages,
            system_prompt_id: None,
            prompt_variables: None,
            temperature: None,
            max_tokens: None,
            top_p: None,
            stop: None,
            presence_penalty: None,
            frequency_penalty: None,
            stream: false,
        }
    }

    pub fn builder() -> LlmRequestBuilder {
        LlmRequestBuilder::new()
    }

    /// Check if this request uses a prompt reference
    pub fn has_prompt_reference(&self) -> bool {
        self.system_prompt_id.is_some()
    }
}

/// Builder for LlmRequest
#[derive(Debug, Default)]
pub struct LlmRequestBuilder {
    messages: Vec<Message>,
    system_prompt_id: Option<String>,
    prompt_variables: Option<HashMap<String, String>>,
    temperature: Option<f32>,
    max_tokens: Option<u32>,
    top_p: Option<f32>,
    stop: Option<Vec<String>>,
    presence_penalty: Option<f32>,
    frequency_penalty: Option<f32>,
    stream: bool,
}

impl LlmRequestBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn message(mut self, message: Message) -> Self {
        self.messages.push(message);
        self
    }

    pub fn messages(mut self, messages: Vec<Message>) -> Self {
        self.messages = messages;
        self
    }

    pub fn system(self, content: impl Into<String>) -> Self {
        self.message(Message::system(content))
    }

    pub fn user(self, content: impl Into<String>) -> Self {
        self.message(Message::user(content))
    }

    pub fn assistant(self, content: impl Into<String>) -> Self {
        self.message(Message::assistant(content))
    }

    /// Set a prompt ID to use as the system message
    pub fn system_prompt(mut self, prompt_id: impl Into<String>) -> Self {
        self.system_prompt_id = Some(prompt_id.into());
        self
    }

    /// Set variables for prompt template rendering
    pub fn prompt_variables(mut self, variables: HashMap<String, String>) -> Self {
        self.prompt_variables = Some(variables);
        self
    }

    /// Add a single prompt variable
    pub fn prompt_variable(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.prompt_variables
            .get_or_insert_with(HashMap::new)
            .insert(name.into(), value.into());
        self
    }

    pub fn temperature(mut self, temp: f32) -> Self {
        self.temperature = Some(temp);
        self
    }

    pub fn max_tokens(mut self, tokens: u32) -> Self {
        self.max_tokens = Some(tokens);
        self
    }

    pub fn top_p(mut self, top_p: f32) -> Self {
        self.top_p = Some(top_p);
        self
    }

    pub fn stop(mut self, stop: Vec<String>) -> Self {
        self.stop = Some(stop);
        self
    }

    pub fn presence_penalty(mut self, penalty: f32) -> Self {
        self.presence_penalty = Some(penalty);
        self
    }

    pub fn frequency_penalty(mut self, penalty: f32) -> Self {
        self.frequency_penalty = Some(penalty);
        self
    }

    pub fn stream(mut self, stream: bool) -> Self {
        self.stream = stream;
        self
    }

    pub fn build(self) -> LlmRequest {
        LlmRequest {
            messages: self.messages,
            system_prompt_id: self.system_prompt_id,
            prompt_variables: self.prompt_variables,
            temperature: self.temperature,
            max_tokens: self.max_tokens,
            top_p: self.top_p,
            stop: self.stop,
            presence_penalty: self.presence_penalty,
            frequency_penalty: self.frequency_penalty,
            stream: self.stream,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_builder() {
        let request = LlmRequest::builder()
            .system("You are a helpful assistant")
            .user("Hello!")
            .temperature(0.7)
            .max_tokens(100)
            .build();

        assert_eq!(request.messages.len(), 2);
        assert_eq!(request.temperature, Some(0.7));
        assert_eq!(request.max_tokens, Some(100));
        assert!(!request.stream);
        assert!(!request.has_prompt_reference());
    }

    #[test]
    fn test_request_with_prompt_reference() {
        let request = LlmRequest::builder()
            .system_prompt("my-system-prompt")
            .prompt_variable("role", "assistant")
            .prompt_variable("task", "help users")
            .user("Hello!")
            .build();

        assert!(request.has_prompt_reference());
        assert_eq!(request.system_prompt_id, Some("my-system-prompt".to_string()));

        let vars = request.prompt_variables.unwrap();
        assert_eq!(vars.get("role"), Some(&"assistant".to_string()));
        assert_eq!(vars.get("task"), Some(&"help users".to_string()));
    }

    #[test]
    fn test_request_with_prompt_variables_map() {
        let mut variables = HashMap::new();
        variables.insert("name".to_string(), "Claude".to_string());
        variables.insert("mode".to_string(), "friendly".to_string());

        let request = LlmRequest::builder()
            .system_prompt("greeting-prompt")
            .prompt_variables(variables)
            .user("Hi!")
            .build();

        assert!(request.has_prompt_reference());

        let vars = request.prompt_variables.unwrap();
        assert_eq!(vars.len(), 2);
        assert_eq!(vars.get("name"), Some(&"Claude".to_string()));
    }
}
