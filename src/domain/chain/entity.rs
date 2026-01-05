//! Model chain entity and related types

use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::{validate_model_id, ModelId, ModelValidationError};

/// Chain identifier - uses same validation as ModelId
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ChainId(String);

impl ChainId {
    /// Create a new ChainId after validation
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

impl TryFrom<String> for ChainId {
    type Error = ModelValidationError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<ChainId> for String {
    fn from(id: ChainId) -> Self {
        id.0
    }
}

impl std::fmt::Display for ChainId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Retry configuration for a chain step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_retries: u32,
    /// Initial delay before first retry
    pub initial_delay_ms: u64,
    /// Maximum delay between retries
    pub max_delay_ms: u64,
    /// Multiplier for exponential backoff
    pub backoff_multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay_ms: 100,
            max_delay_ms: 5000,
            backoff_multiplier: 2.0,
        }
    }
}

impl RetryConfig {
    pub fn new(max_retries: u32) -> Self {
        Self {
            max_retries,
            ..Default::default()
        }
    }

    pub fn with_initial_delay(mut self, ms: u64) -> Self {
        self.initial_delay_ms = ms;
        self
    }

    pub fn with_max_delay(mut self, ms: u64) -> Self {
        self.max_delay_ms = ms;
        self
    }

    pub fn with_backoff_multiplier(mut self, multiplier: f64) -> Self {
        self.backoff_multiplier = multiplier;
        self
    }

    /// Calculate delay for a given attempt number (0-indexed)
    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        if attempt == 0 {
            return Duration::from_millis(self.initial_delay_ms);
        }

        let delay = self.initial_delay_ms as f64
            * self.backoff_multiplier.powi(attempt as i32);
        let delay_ms = delay.min(self.max_delay_ms as f64) as u64;

        Duration::from_millis(delay_ms)
    }
}

/// Behavior when a step fails after all retries
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FallbackBehavior {
    /// Continue to the next step in the chain
    Continue,
    /// Stop the chain and return the error
    Stop,
    /// Skip remaining steps and return partial result
    Skip,
}

impl Default for FallbackBehavior {
    fn default() -> Self {
        Self::Continue
    }
}

/// A single step in a model chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainStep {
    /// Reference to the model ID to use
    model_id: ModelId,
    /// Optional step name/description
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    /// Retry configuration
    retry_config: RetryConfig,
    /// Maximum latency threshold in milliseconds (0 = no limit)
    max_latency_ms: u64,
    /// Behavior when this step fails
    fallback_behavior: FallbackBehavior,
    /// Priority weight (higher = preferred)
    priority: u32,
}

impl ChainStep {
    /// Create a new chain step with a model reference
    pub fn new(model_id: ModelId) -> Self {
        Self {
            model_id,
            name: None,
            retry_config: RetryConfig::default(),
            max_latency_ms: 0,
            fallback_behavior: FallbackBehavior::default(),
            priority: 0,
        }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn with_retry_config(mut self, config: RetryConfig) -> Self {
        self.retry_config = config;
        self
    }

    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.retry_config.max_retries = max_retries;
        self
    }

    pub fn with_max_latency_ms(mut self, ms: u64) -> Self {
        self.max_latency_ms = ms;
        self
    }

    pub fn with_fallback_behavior(mut self, behavior: FallbackBehavior) -> Self {
        self.fallback_behavior = behavior;
        self
    }

    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }

    // Getters

    pub fn model_id(&self) -> &ModelId {
        &self.model_id
    }

    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    pub fn retry_config(&self) -> &RetryConfig {
        &self.retry_config
    }

    pub fn max_latency_ms(&self) -> u64 {
        self.max_latency_ms
    }

    pub fn max_latency(&self) -> Option<Duration> {
        if self.max_latency_ms > 0 {
            Some(Duration::from_millis(self.max_latency_ms))
        } else {
            None
        }
    }

    pub fn fallback_behavior(&self) -> FallbackBehavior {
        self.fallback_behavior
    }

    pub fn priority(&self) -> u32 {
        self.priority
    }
}

/// Model chain entity representing an ordered sequence of model steps
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelChain {
    /// Unique identifier for this chain
    id: ChainId,
    /// Display name for the chain
    name: String,
    /// Description of the chain's purpose
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    /// Ordered list of chain steps
    steps: Vec<ChainStep>,
    /// Whether the chain is enabled
    enabled: bool,
    /// Creation timestamp
    created_at: DateTime<Utc>,
    /// Last update timestamp
    updated_at: DateTime<Utc>,
}

impl ModelChain {
    /// Create a new ModelChain with required fields
    pub fn new(id: ChainId, name: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id,
            name: name.into(),
            description: None,
            steps: Vec::new(),
            enabled: true,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_step(mut self, step: ChainStep) -> Self {
        self.steps.push(step);
        self
    }

    pub fn with_steps(mut self, steps: Vec<ChainStep>) -> Self {
        self.steps = steps;
        self
    }

    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    // Getters

    pub fn id(&self) -> &ChainId {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    pub fn steps(&self) -> &[ChainStep] {
        &self.steps
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

    pub fn step_count(&self) -> usize {
        self.steps.len()
    }

    /// Check if the chain is empty (has no steps)
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    // Mutators

    pub fn set_name(&mut self, name: impl Into<String>) {
        self.name = name.into();
        self.touch();
    }

    pub fn set_description(&mut self, description: Option<String>) {
        self.description = description;
        self.touch();
    }

    pub fn set_steps(&mut self, steps: Vec<ChainStep>) {
        self.steps = steps;
        self.touch();
    }

    pub fn add_step(&mut self, step: ChainStep) {
        self.steps.push(step);
        self.touch();
    }

    pub fn remove_step(&mut self, index: usize) -> Option<ChainStep> {
        if index < self.steps.len() {
            let step = self.steps.remove(index);
            self.touch();
            Some(step)
        } else {
            None
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        self.touch();
    }

    fn touch(&mut self) {
        self.updated_at = Utc::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_model_id(id: &str) -> ModelId {
        ModelId::new(id).unwrap()
    }

    #[test]
    fn test_chain_id_valid() {
        let id = ChainId::new("my-chain-1").unwrap();
        assert_eq!(id.as_str(), "my-chain-1");
    }

    #[test]
    fn test_chain_id_invalid() {
        let result = ChainId::new("invalid_chain!");
        assert!(result.is_err());
    }

    #[test]
    fn test_retry_config_default() {
        let config = RetryConfig::default();
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.initial_delay_ms, 100);
        assert_eq!(config.backoff_multiplier, 2.0);
    }

    #[test]
    fn test_retry_config_delay_calculation() {
        let config = RetryConfig::new(5)
            .with_initial_delay(100)
            .with_backoff_multiplier(2.0)
            .with_max_delay(1000);

        assert_eq!(config.delay_for_attempt(0), Duration::from_millis(100));
        assert_eq!(config.delay_for_attempt(1), Duration::from_millis(200));
        assert_eq!(config.delay_for_attempt(2), Duration::from_millis(400));
        assert_eq!(config.delay_for_attempt(3), Duration::from_millis(800));
        // Should cap at max_delay
        assert_eq!(config.delay_for_attempt(4), Duration::from_millis(1000));
    }

    #[test]
    fn test_chain_step_creation() {
        let model_id = create_model_id("gpt-4-prod");
        let step = ChainStep::new(model_id.clone())
            .with_name("Primary Model")
            .with_max_retries(5)
            .with_max_latency_ms(5000)
            .with_fallback_behavior(FallbackBehavior::Continue)
            .with_priority(10);

        assert_eq!(step.model_id(), &model_id);
        assert_eq!(step.name(), Some("Primary Model"));
        assert_eq!(step.retry_config().max_retries, 5);
        assert_eq!(step.max_latency_ms(), 5000);
        assert_eq!(step.max_latency(), Some(Duration::from_millis(5000)));
        assert_eq!(step.fallback_behavior(), FallbackBehavior::Continue);
        assert_eq!(step.priority(), 10);
    }

    #[test]
    fn test_chain_step_no_latency_limit() {
        let step = ChainStep::new(create_model_id("test"));
        assert_eq!(step.max_latency_ms(), 0);
        assert_eq!(step.max_latency(), None);
    }

    #[test]
    fn test_model_chain_creation() {
        let chain_id = ChainId::new("prod-chain").unwrap();
        let chain = ModelChain::new(chain_id.clone(), "Production Chain")
            .with_description("Primary production model chain")
            .with_step(ChainStep::new(create_model_id("gpt-4")))
            .with_step(ChainStep::new(create_model_id("claude-3")));

        assert_eq!(chain.id().as_str(), "prod-chain");
        assert_eq!(chain.name(), "Production Chain");
        assert_eq!(chain.description(), Some("Primary production model chain"));
        assert_eq!(chain.step_count(), 2);
        assert!(!chain.is_empty());
        assert!(chain.is_enabled());
    }

    #[test]
    fn test_model_chain_modifications() {
        let chain_id = ChainId::new("test-chain").unwrap();
        let mut chain = ModelChain::new(chain_id, "Test");

        assert!(chain.is_empty());

        chain.add_step(ChainStep::new(create_model_id("model-1")));
        assert_eq!(chain.step_count(), 1);

        chain.add_step(ChainStep::new(create_model_id("model-2")));
        assert_eq!(chain.step_count(), 2);

        let removed = chain.remove_step(0);
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().model_id().as_str(), "model-1");
        assert_eq!(chain.step_count(), 1);
    }

    #[test]
    fn test_fallback_behavior_default() {
        assert_eq!(FallbackBehavior::default(), FallbackBehavior::Continue);
    }
}
