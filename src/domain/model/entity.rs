//! Model entity and related types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::validation::{validate_model_id, ModelValidationError};
use crate::domain::CredentialType;

/// Model identifier - alphanumeric + hyphens, max 50 characters
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ModelId(String);

impl ModelId {
    /// Create a new ModelId after validation
    pub fn new(id: impl Into<String>) -> Result<Self, ModelValidationError> {
        let id = id.into();
        validate_model_id(&id)?;
        Ok(Self(id))
    }

    /// Get the inner string value
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for ModelId {
    type Error = ModelValidationError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<ModelId> for String {
    fn from(id: ModelId) -> Self {
        id.0
    }
}

impl std::fmt::Display for ModelId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Model configuration parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    /// Temperature for response randomness (0.0 - 2.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,

    /// Maximum tokens in response
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,

    /// Top-p (nucleus) sampling parameter (0.0 - 1.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,

    /// Stop sequences
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,

    /// Presence penalty (-2.0 - 2.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f32>,

    /// Frequency penalty (-2.0 - 2.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f32>,

    /// System prompt template
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            temperature: None,
            max_tokens: None,
            top_p: None,
            stop: None,
            presence_penalty: None,
            frequency_penalty: None,
            system_prompt: None,
        }
    }
}

impl ModelConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_temperature(mut self, temp: f32) -> Self {
        self.temperature = Some(temp);
        self
    }

    pub fn with_max_tokens(mut self, max: u32) -> Self {
        self.max_tokens = Some(max);
        self
    }

    pub fn with_top_p(mut self, top_p: f32) -> Self {
        self.top_p = Some(top_p);
        self
    }

    pub fn with_stop(mut self, stop: Vec<String>) -> Self {
        self.stop = Some(stop);
        self
    }

    pub fn with_presence_penalty(mut self, penalty: f32) -> Self {
        self.presence_penalty = Some(penalty);
        self
    }

    pub fn with_frequency_penalty(mut self, penalty: f32) -> Self {
        self.frequency_penalty = Some(penalty);
        self
    }

    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }
}

/// Model entity representing a configured LLM model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Model {
    /// Unique identifier for this model configuration
    id: ModelId,

    /// Display name for the model
    name: String,

    /// Description of the model's purpose
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,

    /// Provider type (OpenAI, Anthropic, etc.)
    provider: CredentialType,

    /// Provider-specific model name (e.g., "gpt-4o", "claude-3-5-sonnet-20241022")
    provider_model: String,

    /// Model configuration parameters
    config: ModelConfig,

    /// Configuration version (for tracking changes)
    version: u32,

    /// Whether the model is enabled
    enabled: bool,

    /// Creation timestamp
    created_at: DateTime<Utc>,

    /// Last update timestamp
    updated_at: DateTime<Utc>,
}

impl Model {
    /// Create a new Model with required fields
    pub fn new(
        id: ModelId,
        name: impl Into<String>,
        provider: CredentialType,
        provider_model: impl Into<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id,
            name: name.into(),
            description: None,
            provider,
            provider_model: provider_model.into(),
            config: ModelConfig::default(),
            version: 1,
            enabled: true,
            created_at: now,
            updated_at: now,
        }
    }

    /// Builder-style method to set description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Builder-style method to set config
    pub fn with_config(mut self, config: ModelConfig) -> Self {
        self.config = config;
        self
    }

    /// Builder-style method to set enabled state
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    // Getters

    pub fn id(&self) -> &ModelId {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    pub fn provider(&self) -> &CredentialType {
        &self.provider
    }

    pub fn provider_model(&self) -> &str {
        &self.provider_model
    }

    pub fn config(&self) -> &ModelConfig {
        &self.config
    }

    pub fn version(&self) -> u32 {
        self.version
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    // Mutators (for service layer updates)

    /// Update the model name
    pub fn set_name(&mut self, name: impl Into<String>) {
        self.name = name.into();
        self.touch();
    }

    /// Update the description
    pub fn set_description(&mut self, description: Option<String>) {
        self.description = description;
        self.touch();
    }

    /// Update the provider model name
    pub fn set_provider_model(&mut self, provider_model: impl Into<String>) {
        self.provider_model = provider_model.into();
        self.touch();
    }

    /// Update the configuration (increments version)
    pub fn set_config(&mut self, config: ModelConfig) {
        self.config = config;
        self.version += 1;
        self.touch();
    }

    /// Enable or disable the model
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        self.touch();
    }

    /// Update the updated_at timestamp
    fn touch(&mut self) {
        self.updated_at = Utc::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_id_valid() {
        let id = ModelId::new("my-model-1").unwrap();
        assert_eq!(id.as_str(), "my-model-1");
    }

    #[test]
    fn test_model_id_invalid_chars() {
        let result = ModelId::new("my_model!");
        assert!(result.is_err());
    }

    #[test]
    fn test_model_id_too_long() {
        let long_id = "a".repeat(51);
        let result = ModelId::new(long_id);
        assert!(result.is_err());
    }

    #[test]
    fn test_model_id_empty() {
        let result = ModelId::new("");
        assert!(result.is_err());
    }

    #[test]
    fn test_model_creation() {
        let id = ModelId::new("gpt4-production").unwrap();
        let model = Model::new(
            id.clone(),
            "GPT-4 Production",
            CredentialType::OpenAi,
            "gpt-4o",
        )
        .with_description("Production GPT-4 model")
        .with_config(ModelConfig::new().with_temperature(0.7).with_max_tokens(4096));

        assert_eq!(model.id().as_str(), "gpt4-production");
        assert_eq!(model.name(), "GPT-4 Production");
        assert_eq!(model.description(), Some("Production GPT-4 model"));
        assert_eq!(model.provider(), &CredentialType::OpenAi);
        assert_eq!(model.provider_model(), "gpt-4o");
        assert_eq!(model.config().temperature, Some(0.7));
        assert_eq!(model.config().max_tokens, Some(4096));
        assert_eq!(model.version(), 1);
        assert!(model.is_enabled());
    }

    #[test]
    fn test_model_config_update_increments_version() {
        let id = ModelId::new("test-model").unwrap();
        let mut model = Model::new(id, "Test", CredentialType::OpenAi, "gpt-4");

        assert_eq!(model.version(), 1);

        model.set_config(ModelConfig::new().with_temperature(0.5));
        assert_eq!(model.version(), 2);

        model.set_config(ModelConfig::new().with_temperature(0.8));
        assert_eq!(model.version(), 3);
    }

    #[test]
    fn test_model_config_builder() {
        let config = ModelConfig::new()
            .with_temperature(0.7)
            .with_max_tokens(2048)
            .with_top_p(0.9)
            .with_presence_penalty(0.1)
            .with_frequency_penalty(0.2)
            .with_stop(vec!["END".to_string()])
            .with_system_prompt("You are helpful");

        assert_eq!(config.temperature, Some(0.7));
        assert_eq!(config.max_tokens, Some(2048));
        assert_eq!(config.top_p, Some(0.9));
        assert_eq!(config.presence_penalty, Some(0.1));
        assert_eq!(config.frequency_penalty, Some(0.2));
        assert_eq!(config.stop, Some(vec!["END".to_string()]));
        assert_eq!(config.system_prompt, Some("You are helpful".to_string()));
    }
}
