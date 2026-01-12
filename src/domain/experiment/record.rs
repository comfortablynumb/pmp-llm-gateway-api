//! Experiment record types for tracking individual requests

use serde::{Deserialize, Serialize};
use std::time::SystemTime;

use crate::domain::storage::{StorageEntity, StorageKey};

/// Unique identifier for an experiment record
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ExperimentRecordId(String);

impl ExperimentRecordId {
    /// Create a new experiment record ID
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Generate a new unique ID
    pub fn generate() -> Self {
        Self(format!("exprec-{}", uuid::Uuid::new_v4()))
    }

    /// Get the inner string value
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for ExperimentRecordId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for ExperimentRecordId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl std::fmt::Display for ExperimentRecordId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl StorageKey for ExperimentRecordId {
    fn as_str(&self) -> &str {
        &self.0
    }
}

impl StorageEntity for ExperimentRecord {
    type Key = ExperimentRecordId;

    fn key(&self) -> &Self::Key {
        &self.id
    }
}

/// A record of a single request in an experiment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentRecord {
    /// Unique identifier for this record
    id: ExperimentRecordId,
    /// ID of the experiment this record belongs to
    pub experiment_id: String,
    /// ID of the variant that was used
    pub variant_id: String,
    /// ID of the API key that made the request
    pub api_key_id: String,
    /// ID of the model that was used
    pub model_id: String,
    /// Number of input tokens
    pub input_tokens: u32,
    /// Number of output tokens
    pub output_tokens: u32,
    /// Total tokens (input + output)
    pub total_tokens: u32,
    /// Cost in micro-dollars
    pub cost_micros: i64,
    /// Latency in milliseconds
    pub latency_ms: u64,
    /// Whether the request was successful
    pub success: bool,
    /// Error message if the request failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Unix timestamp when the request was made
    pub timestamp: u64,
}

impl ExperimentRecord {
    /// Create a new experiment record
    pub fn new(
        id: impl Into<ExperimentRecordId>,
        experiment_id: impl Into<String>,
        variant_id: impl Into<String>,
        api_key_id: impl Into<String>,
    ) -> Self {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            id: id.into(),
            experiment_id: experiment_id.into(),
            variant_id: variant_id.into(),
            api_key_id: api_key_id.into(),
            model_id: String::new(),
            input_tokens: 0,
            output_tokens: 0,
            total_tokens: 0,
            cost_micros: 0,
            latency_ms: 0,
            success: true,
            error: None,
            timestamp: now,
        }
    }

    /// Set the model ID
    pub fn with_model_id(mut self, model_id: impl Into<String>) -> Self {
        self.model_id = model_id.into();
        self
    }

    /// Set the token counts
    pub fn with_tokens(mut self, input: u32, output: u32) -> Self {
        self.input_tokens = input;
        self.output_tokens = output;
        self.total_tokens = input + output;
        self
    }

    /// Set the cost in micro-dollars
    pub fn with_cost_micros(mut self, cost: i64) -> Self {
        self.cost_micros = cost;
        self
    }

    /// Set the cost in USD
    pub fn with_cost_usd(mut self, cost: f64) -> Self {
        self.cost_micros = (cost * 1_000_000.0) as i64;
        self
    }

    /// Set the latency in milliseconds
    pub fn with_latency_ms(mut self, latency: u64) -> Self {
        self.latency_ms = latency;
        self
    }

    /// Mark as failed with an error message
    pub fn with_error(mut self, error: impl Into<String>) -> Self {
        self.success = false;
        self.error = Some(error.into());
        self
    }

    /// Set the timestamp
    pub fn with_timestamp(mut self, timestamp: u64) -> Self {
        self.timestamp = timestamp;
        self
    }

    /// Get the cost in USD
    pub fn cost_usd(&self) -> f64 {
        self.cost_micros as f64 / 1_000_000.0
    }

    /// Get the record ID
    pub fn id(&self) -> &ExperimentRecordId {
        &self.id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_creation() {
        let record = ExperimentRecord::new("rec-1", "exp-1", "control", "api-key-1");

        assert_eq!(record.id().as_str(), "rec-1");
        assert_eq!(record.experiment_id, "exp-1");
        assert_eq!(record.variant_id, "control");
        assert_eq!(record.api_key_id, "api-key-1");
        assert!(record.success);
        assert!(record.error.is_none());
    }

    #[test]
    fn test_record_with_tokens() {
        let record = ExperimentRecord::new("rec-1", "exp-1", "control", "api-key-1")
            .with_tokens(100, 50);

        assert_eq!(record.input_tokens, 100);
        assert_eq!(record.output_tokens, 50);
        assert_eq!(record.total_tokens, 150);
    }

    #[test]
    fn test_record_with_cost() {
        let record = ExperimentRecord::new("rec-1", "exp-1", "control", "api-key-1")
            .with_cost_usd(0.001);

        assert_eq!(record.cost_micros, 1000);
        assert!((record.cost_usd() - 0.001).abs() < 0.000001);
    }

    #[test]
    fn test_record_with_error() {
        let record = ExperimentRecord::new("rec-1", "exp-1", "control", "api-key-1")
            .with_error("Connection timeout");

        assert!(!record.success);
        assert_eq!(record.error, Some("Connection timeout".to_string()));
    }

    #[test]
    fn test_record_builder_chain() {
        let record = ExperimentRecord::new("rec-1", "exp-1", "control", "api-key-1")
            .with_model_id("gpt-4")
            .with_tokens(100, 50)
            .with_cost_micros(1500)
            .with_latency_ms(250);

        assert_eq!(record.model_id, "gpt-4");
        assert_eq!(record.total_tokens, 150);
        assert_eq!(record.cost_micros, 1500);
        assert_eq!(record.latency_ms, 250);
        assert!(record.success);
    }
}
