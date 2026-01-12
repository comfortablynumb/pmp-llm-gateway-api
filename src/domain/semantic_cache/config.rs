//! Semantic cache configuration

use std::time::Duration;

use serde::{Deserialize, Serialize};

/// Configuration for semantic caching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticCacheConfig {
    /// Whether semantic caching is enabled
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    /// Similarity threshold for cache hits (0.0 to 1.0)
    /// Higher values require more similar queries
    #[serde(default = "default_similarity_threshold")]
    pub similarity_threshold: f32,

    /// Maximum number of entries to store
    #[serde(default = "default_max_entries")]
    pub max_entries: usize,

    /// Time-to-live for cached entries in seconds
    #[serde(default = "default_ttl_secs")]
    pub ttl_secs: u64,

    /// Embedding model to use
    #[serde(default = "default_embedding_model")]
    pub embedding_model: String,

    /// Namespace prefix for cache entries
    #[serde(default = "default_namespace")]
    pub namespace: String,

    /// Whether to cache streaming responses
    #[serde(default)]
    pub cache_streaming: bool,

    /// Include model ID in cache key (cache per model)
    #[serde(default = "default_true")]
    pub include_model_in_key: bool,

    /// Include temperature in cache key
    #[serde(default)]
    pub include_temperature_in_key: bool,
}

fn default_enabled() -> bool {
    true
}

fn default_similarity_threshold() -> f32 {
    0.95
}

fn default_max_entries() -> usize {
    10000
}

fn default_ttl_secs() -> u64 {
    3600
}

fn default_embedding_model() -> String {
    "text-embedding-3-small".to_string()
}

fn default_namespace() -> String {
    "semantic:llm".to_string()
}

fn default_true() -> bool {
    true
}

impl Default for SemanticCacheConfig {
    fn default() -> Self {
        Self {
            enabled: default_enabled(),
            similarity_threshold: default_similarity_threshold(),
            max_entries: default_max_entries(),
            ttl_secs: default_ttl_secs(),
            embedding_model: default_embedding_model(),
            namespace: default_namespace(),
            cache_streaming: false,
            include_model_in_key: default_true(),
            include_temperature_in_key: false,
        }
    }
}

impl SemanticCacheConfig {
    /// Create a new config with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Get TTL as Duration
    pub fn ttl(&self) -> Duration {
        Duration::from_secs(self.ttl_secs)
    }

    /// Set whether caching is enabled
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Set the similarity threshold
    pub fn with_similarity_threshold(mut self, threshold: f32) -> Self {
        self.similarity_threshold = threshold.clamp(0.0, 1.0);
        self
    }

    /// Set the maximum number of entries
    pub fn with_max_entries(mut self, max: usize) -> Self {
        self.max_entries = max;
        self
    }

    /// Set the TTL
    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.ttl_secs = ttl.as_secs();
        self
    }

    /// Set the embedding model
    pub fn with_embedding_model(mut self, model: impl Into<String>) -> Self {
        self.embedding_model = model.into();
        self
    }

    /// Set the namespace
    pub fn with_namespace(mut self, namespace: impl Into<String>) -> Self {
        self.namespace = namespace.into();
        self
    }

    /// Set whether to cache streaming responses
    pub fn with_cache_streaming(mut self, cache: bool) -> Self {
        self.cache_streaming = cache;
        self
    }

    /// Set whether to include model in cache key
    pub fn with_include_model(mut self, include: bool) -> Self {
        self.include_model_in_key = include;
        self
    }

    /// Set whether to include temperature in cache key
    pub fn with_include_temperature(mut self, include: bool) -> Self {
        self.include_temperature_in_key = include;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = SemanticCacheConfig::default();

        assert!(config.enabled);
        assert!((config.similarity_threshold - 0.95).abs() < 0.01);
        assert_eq!(config.max_entries, 10000);
        assert_eq!(config.ttl(), Duration::from_secs(3600));
        assert_eq!(config.embedding_model, "text-embedding-3-small");
        assert_eq!(config.namespace, "semantic:llm");
        assert!(!config.cache_streaming);
        assert!(config.include_model_in_key);
        assert!(!config.include_temperature_in_key);
    }

    #[test]
    fn test_config_builder() {
        let config = SemanticCacheConfig::new()
            .with_enabled(false)
            .with_similarity_threshold(0.9)
            .with_max_entries(5000)
            .with_ttl(Duration::from_secs(1800))
            .with_embedding_model("custom-model")
            .with_namespace("custom:ns")
            .with_cache_streaming(true)
            .with_include_model(false)
            .with_include_temperature(true);

        assert!(!config.enabled);
        assert!((config.similarity_threshold - 0.9).abs() < 0.01);
        assert_eq!(config.max_entries, 5000);
        assert_eq!(config.ttl(), Duration::from_secs(1800));
        assert_eq!(config.embedding_model, "custom-model");
        assert_eq!(config.namespace, "custom:ns");
        assert!(config.cache_streaming);
        assert!(!config.include_model_in_key);
        assert!(config.include_temperature_in_key);
    }

    #[test]
    fn test_similarity_threshold_clamped() {
        let config = SemanticCacheConfig::new().with_similarity_threshold(1.5);
        assert!((config.similarity_threshold - 1.0).abs() < 0.01);

        let config = SemanticCacheConfig::new().with_similarity_threshold(-0.5);
        assert!(config.similarity_threshold.abs() < 0.01);
    }
}
