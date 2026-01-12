//! Usage record entities

use std::collections::HashMap;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use crate::domain::storage::{StorageEntity, StorageKey};

/// Unique identifier for a usage record
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UsageRecordId(String);

impl UsageRecordId {
    /// Create a new usage record ID
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Generate a new unique ID
    pub fn generate() -> Self {
        Self(format!("usage-{}", uuid::Uuid::new_v4()))
    }

    /// Get the inner string value
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for UsageRecordId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for UsageRecordId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl std::fmt::Display for UsageRecordId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl StorageKey for UsageRecordId {
    fn as_str(&self) -> &str {
        &self.0
    }
}

impl StorageEntity for UsageRecord {
    type Key = UsageRecordId;

    fn key(&self) -> &Self::Key {
        &self.id
    }
}

/// Type of usage being recorded
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UsageType {
    /// Chat completion request
    ChatCompletion,
    /// Embedding generation
    Embedding,
    /// Workflow execution
    Workflow,
    /// Knowledge base search
    KnowledgeBaseSearch,
}

impl std::fmt::Display for UsageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ChatCompletion => write!(f, "chat_completion"),
            Self::Embedding => write!(f, "embedding"),
            Self::Workflow => write!(f, "workflow"),
            Self::KnowledgeBaseSearch => write!(f, "knowledge_base_search"),
        }
    }
}

/// A single usage record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageRecord {
    /// Unique ID
    id: UsageRecordId,
    /// Type of usage
    pub usage_type: UsageType,
    /// API key ID that made the request
    pub api_key_id: String,
    /// Model ID used (if applicable)
    pub model_id: Option<String>,
    /// Number of input/prompt tokens
    pub input_tokens: u32,
    /// Number of output/completion tokens
    pub output_tokens: u32,
    /// Total tokens
    pub total_tokens: u32,
    /// Cost in USD (micro-dollars for precision)
    pub cost_micros: i64,
    /// Request latency in milliseconds
    pub latency_ms: u64,
    /// Whether the request was successful
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
    /// Timestamp when the request was made
    pub timestamp: u64,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

impl UsageRecord {
    /// Create a new usage record
    pub fn new(
        id: impl Into<UsageRecordId>,
        usage_type: UsageType,
        api_key_id: impl Into<String>,
    ) -> Self {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            id: id.into(),
            usage_type,
            api_key_id: api_key_id.into(),
            model_id: None,
            input_tokens: 0,
            output_tokens: 0,
            total_tokens: 0,
            cost_micros: 0,
            latency_ms: 0,
            success: true,
            error: None,
            timestamp: now,
            metadata: HashMap::new(),
        }
    }

    /// Set the model ID
    pub fn with_model_id(mut self, model_id: impl Into<String>) -> Self {
        self.model_id = Some(model_id.into());
        self
    }

    /// Set token counts
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

    /// Set the cost in dollars
    pub fn with_cost(mut self, cost_usd: f64) -> Self {
        self.cost_micros = (cost_usd * 1_000_000.0) as i64;
        self
    }

    /// Set the latency
    pub fn with_latency_ms(mut self, latency: u64) -> Self {
        self.latency_ms = latency;
        self
    }

    /// Mark as failed with error
    pub fn with_error(mut self, error: impl Into<String>) -> Self {
        self.success = false;
        self.error = Some(error.into());
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Get cost in USD
    pub fn cost_usd(&self) -> f64 {
        self.cost_micros as f64 / 1_000_000.0
    }

    /// Get the record ID
    pub fn id(&self) -> &UsageRecordId {
        &self.id
    }
}

/// Aggregated usage statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UsageAggregate {
    /// Total number of requests
    pub total_requests: u64,
    /// Number of successful requests
    pub successful_requests: u64,
    /// Number of failed requests
    pub failed_requests: u64,
    /// Total input tokens
    pub total_input_tokens: u64,
    /// Total output tokens
    pub total_output_tokens: u64,
    /// Total tokens
    pub total_tokens: u64,
    /// Total cost in micro-dollars
    pub total_cost_micros: i64,
    /// Average latency in milliseconds
    pub avg_latency_ms: f64,
    /// Usage breakdown by type
    pub by_type: HashMap<UsageType, u64>,
    /// Usage breakdown by model
    pub by_model: HashMap<String, u64>,
}

impl UsageAggregate {
    /// Create empty aggregate
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a usage record to the aggregate
    pub fn add_record(&mut self, record: &UsageRecord) {
        self.total_requests += 1;

        if record.success {
            self.successful_requests += 1;
        } else {
            self.failed_requests += 1;
        }

        self.total_input_tokens += record.input_tokens as u64;
        self.total_output_tokens += record.output_tokens as u64;
        self.total_tokens += record.total_tokens as u64;
        self.total_cost_micros += record.cost_micros;

        // Update running average latency
        let prev_total = self.avg_latency_ms * (self.total_requests - 1) as f64;
        self.avg_latency_ms = (prev_total + record.latency_ms as f64) / self.total_requests as f64;

        // Update breakdowns
        *self.by_type.entry(record.usage_type).or_insert(0) += 1;

        if let Some(ref model_id) = record.model_id {
            *self.by_model.entry(model_id.clone()).or_insert(0) += 1;
        }
    }

    /// Get total cost in USD
    pub fn total_cost_usd(&self) -> f64 {
        self.total_cost_micros as f64 / 1_000_000.0
    }

    /// Get success rate
    pub fn success_rate(&self) -> f64 {
        if self.total_requests == 0 {
            return 0.0;
        }

        self.successful_requests as f64 / self.total_requests as f64
    }
}

/// Usage summary for a time period
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageSummary {
    /// Start of the period (unix timestamp)
    pub period_start: u64,
    /// End of the period (unix timestamp)
    pub period_end: u64,
    /// Aggregated usage
    pub aggregate: UsageAggregate,
    /// Daily breakdown (timestamp -> aggregate)
    pub daily: Vec<DailyUsage>,
}

/// Daily usage breakdown
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyUsage {
    /// Date (unix timestamp at start of day)
    pub date: u64,
    /// Request count
    pub requests: u64,
    /// Total tokens
    pub tokens: u64,
    /// Cost in micro-dollars
    pub cost_micros: i64,
}

impl DailyUsage {
    /// Get cost in USD
    pub fn cost_usd(&self) -> f64 {
        self.cost_micros as f64 / 1_000_000.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_usage_record_creation() {
        let record = UsageRecord::new("rec-123", UsageType::ChatCompletion, "api-key-1")
            .with_model_id("gpt-4")
            .with_tokens(100, 50)
            .with_cost(0.005)
            .with_latency_ms(250);

        assert_eq!(record.id().as_str(), "rec-123");
        assert_eq!(record.usage_type, UsageType::ChatCompletion);
        assert_eq!(record.model_id, Some("gpt-4".to_string()));
        assert_eq!(record.input_tokens, 100);
        assert_eq!(record.output_tokens, 50);
        assert_eq!(record.total_tokens, 150);
        assert_eq!(record.cost_micros, 5000);
        assert!((record.cost_usd() - 0.005).abs() < 0.0001);
        assert_eq!(record.latency_ms, 250);
        assert!(record.success);
    }

    #[test]
    fn test_usage_record_with_error() {
        let record = UsageRecord::new("rec-123", UsageType::ChatCompletion, "api-key-1")
            .with_error("Rate limit exceeded");

        assert!(!record.success);
        assert_eq!(record.error, Some("Rate limit exceeded".to_string()));
    }

    #[test]
    fn test_usage_aggregate() {
        let mut aggregate = UsageAggregate::new();

        let record1 = UsageRecord::new("rec-1", UsageType::ChatCompletion, "api-key-1")
            .with_model_id("gpt-4")
            .with_tokens(100, 50)
            .with_cost(0.01)
            .with_latency_ms(200);

        let record2 = UsageRecord::new("rec-2", UsageType::ChatCompletion, "api-key-1")
            .with_model_id("gpt-4")
            .with_tokens(200, 100)
            .with_cost(0.02)
            .with_latency_ms(300);

        aggregate.add_record(&record1);
        aggregate.add_record(&record2);

        assert_eq!(aggregate.total_requests, 2);
        assert_eq!(aggregate.successful_requests, 2);
        assert_eq!(aggregate.total_input_tokens, 300);
        assert_eq!(aggregate.total_output_tokens, 150);
        assert_eq!(aggregate.total_tokens, 450);
        assert!((aggregate.total_cost_usd() - 0.03).abs() < 0.0001);
        assert!((aggregate.avg_latency_ms - 250.0).abs() < 0.1);
        assert!((aggregate.success_rate() - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_usage_aggregate_with_failures() {
        let mut aggregate = UsageAggregate::new();

        let success = UsageRecord::new("rec-1", UsageType::ChatCompletion, "api-key-1");
        let failure = UsageRecord::new("rec-2", UsageType::ChatCompletion, "api-key-1")
            .with_error("Error");

        aggregate.add_record(&success);
        aggregate.add_record(&failure);

        assert_eq!(aggregate.total_requests, 2);
        assert_eq!(aggregate.successful_requests, 1);
        assert_eq!(aggregate.failed_requests, 1);
        assert!((aggregate.success_rate() - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_usage_type_display() {
        assert_eq!(UsageType::ChatCompletion.to_string(), "chat_completion");
        assert_eq!(UsageType::Embedding.to_string(), "embedding");
        assert_eq!(UsageType::Workflow.to_string(), "workflow");
    }
}
