//! Semantic cache trait and types

use std::fmt::Debug;
use std::time::{Duration, SystemTime};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::domain::DomainError;

/// A cached entry in the semantic cache
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedEntry {
    /// Unique identifier for this entry
    id: String,
    /// The embedding vector for similarity search
    embedding: Vec<f32>,
    /// The original query/input text
    query_text: String,
    /// The cached response value (JSON serialized)
    value: String,
    /// Model ID this entry is associated with
    model_id: Option<String>,
    /// Temperature used for the request
    temperature: Option<f32>,
    /// When this entry was created
    created_at: u64,
    /// When this entry expires
    expires_at: u64,
    /// Number of cache hits
    hit_count: u32,
    /// Additional metadata
    metadata: Option<serde_json::Value>,
}

impl CachedEntry {
    /// Create a new cached entry
    pub fn new(
        id: impl Into<String>,
        embedding: Vec<f32>,
        query_text: impl Into<String>,
        value: impl Into<String>,
        ttl: Duration,
    ) -> Self {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            id: id.into(),
            embedding,
            query_text: query_text.into(),
            value: value.into(),
            model_id: None,
            temperature: None,
            created_at: now,
            expires_at: now + ttl.as_secs(),
            hit_count: 0,
            metadata: None,
        }
    }

    /// Set the model ID
    pub fn with_model_id(mut self, model_id: impl Into<String>) -> Self {
        self.model_id = Some(model_id.into());
        self
    }

    /// Set the temperature
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Set metadata
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Get the entry ID
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Get the embedding vector
    pub fn embedding(&self) -> &[f32] {
        &self.embedding
    }

    /// Get the original query text
    pub fn query_text(&self) -> &str {
        &self.query_text
    }

    /// Get the cached value
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Get the model ID
    pub fn model_id(&self) -> Option<&str> {
        self.model_id.as_deref()
    }

    /// Get the temperature
    pub fn temperature(&self) -> Option<f32> {
        self.temperature
    }

    /// Get creation timestamp
    pub fn created_at(&self) -> u64 {
        self.created_at
    }

    /// Get expiration timestamp
    pub fn expires_at(&self) -> u64 {
        self.expires_at
    }

    /// Get hit count
    pub fn hit_count(&self) -> u32 {
        self.hit_count
    }

    /// Get metadata
    pub fn metadata(&self) -> Option<&serde_json::Value> {
        self.metadata.as_ref()
    }

    /// Check if entry is expired
    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        now >= self.expires_at
    }

    /// Increment hit count
    pub fn increment_hits(&mut self) {
        self.hit_count += 1;
    }

    /// Deserialize the cached value
    pub fn deserialize_value<T: for<'de> Deserialize<'de>>(&self) -> Result<T, DomainError> {
        serde_json::from_str(&self.value).map_err(|e| {
            DomainError::internal(format!("Failed to deserialize cached value: {}", e))
        })
    }
}

/// Result of a semantic cache search
#[derive(Debug, Clone)]
pub struct SemanticSearchResult {
    /// The matching cached entry
    pub entry: CachedEntry,
    /// Similarity score (0.0 to 1.0)
    pub similarity: f32,
}

impl SemanticSearchResult {
    /// Create a new search result
    pub fn new(entry: CachedEntry, similarity: f32) -> Self {
        Self { entry, similarity }
    }
}

/// Statistics for the semantic cache
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SemanticCacheStats {
    /// Total number of entries
    pub total_entries: usize,
    /// Total cache hits
    pub hits: u64,
    /// Total cache misses
    pub misses: u64,
    /// Total entries evicted
    pub evictions: u64,
    /// Average similarity of hits
    pub avg_hit_similarity: f32,
}

impl SemanticCacheStats {
    /// Calculate hit rate
    pub fn hit_rate(&self) -> f32 {
        let total = self.hits + self.misses;

        if total == 0 {
            return 0.0;
        }

        self.hits as f32 / total as f32
    }
}

/// Search parameters for semantic cache lookup
#[derive(Debug, Clone)]
pub struct SemanticSearchParams {
    /// Model ID to filter by (if applicable)
    pub model_id: Option<String>,
    /// Temperature to filter by (if applicable)
    pub temperature: Option<f32>,
    /// Minimum similarity threshold
    pub min_similarity: f32,
    /// Maximum results to return
    pub limit: usize,
}

impl Default for SemanticSearchParams {
    fn default() -> Self {
        Self {
            model_id: None,
            temperature: None,
            min_similarity: 0.95,
            limit: 1,
        }
    }
}

impl SemanticSearchParams {
    /// Create new search params with minimum similarity
    pub fn new(min_similarity: f32) -> Self {
        Self {
            min_similarity,
            ..Default::default()
        }
    }

    /// Set model ID filter
    pub fn with_model_id(mut self, model_id: impl Into<String>) -> Self {
        self.model_id = Some(model_id.into());
        self
    }

    /// Set temperature filter
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Set result limit
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }
}

/// Trait for semantic (vector-based) caching
#[async_trait]
pub trait SemanticCache: Send + Sync + Debug {
    /// Search for similar entries based on embedding
    async fn search(
        &self,
        embedding: &[f32],
        params: &SemanticSearchParams,
    ) -> Result<Vec<SemanticSearchResult>, DomainError>;

    /// Find the most similar entry
    async fn find_similar(
        &self,
        embedding: &[f32],
        params: &SemanticSearchParams,
    ) -> Result<Option<SemanticSearchResult>, DomainError> {
        let results = self.search(embedding, params).await?;
        Ok(results.into_iter().next())
    }

    /// Store a new entry
    async fn store(&self, entry: CachedEntry) -> Result<(), DomainError>;

    /// Get an entry by ID
    async fn get(&self, id: &str) -> Result<Option<CachedEntry>, DomainError>;

    /// Delete an entry by ID
    async fn delete(&self, id: &str) -> Result<bool, DomainError>;

    /// Delete entries by model ID
    async fn delete_by_model(&self, model_id: &str) -> Result<usize, DomainError>;

    /// Clear all entries
    async fn clear(&self) -> Result<(), DomainError>;

    /// Get cache statistics
    async fn stats(&self) -> Result<SemanticCacheStats, DomainError>;

    /// Get the number of entries
    async fn size(&self) -> Result<usize, DomainError>;

    /// Record a cache hit
    async fn record_hit(&self, id: &str) -> Result<(), DomainError>;

    /// Record a cache miss
    async fn record_miss(&self) -> Result<(), DomainError>;

    /// Clean up expired entries
    async fn cleanup_expired(&self) -> Result<usize, DomainError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cached_entry_creation() {
        let embedding = vec![0.1, 0.2, 0.3];
        let entry = CachedEntry::new(
            "test-id",
            embedding.clone(),
            "hello world",
            r#"{"response": "test"}"#,
            Duration::from_secs(3600),
        );

        assert_eq!(entry.id(), "test-id");
        assert_eq!(entry.embedding(), &embedding);
        assert_eq!(entry.query_text(), "hello world");
        assert_eq!(entry.value(), r#"{"response": "test"}"#);
        assert_eq!(entry.hit_count(), 0);
        assert!(!entry.is_expired());
    }

    #[test]
    fn test_cached_entry_with_metadata() {
        let entry = CachedEntry::new(
            "test-id",
            vec![0.1],
            "query",
            "value",
            Duration::from_secs(3600),
        )
        .with_model_id("gpt-4")
        .with_temperature(0.7)
        .with_metadata(serde_json::json!({"key": "value"}));

        assert_eq!(entry.model_id(), Some("gpt-4"));
        assert_eq!(entry.temperature(), Some(0.7));
        assert!(entry.metadata().is_some());
    }

    #[test]
    fn test_cached_entry_expired() {
        let mut entry = CachedEntry::new(
            "test-id",
            vec![0.1],
            "query",
            "value",
            Duration::from_secs(0),
        );

        // Force expiration by setting expires_at to past
        entry.expires_at = 0;

        assert!(entry.is_expired());
    }

    #[test]
    fn test_cached_entry_increment_hits() {
        let mut entry = CachedEntry::new(
            "test-id",
            vec![0.1],
            "query",
            "value",
            Duration::from_secs(3600),
        );

        assert_eq!(entry.hit_count(), 0);
        entry.increment_hits();
        assert_eq!(entry.hit_count(), 1);
        entry.increment_hits();
        assert_eq!(entry.hit_count(), 2);
    }

    #[test]
    fn test_semantic_search_params() {
        let params = SemanticSearchParams::new(0.9)
            .with_model_id("gpt-4")
            .with_temperature(0.5)
            .with_limit(5);

        assert_eq!(params.model_id, Some("gpt-4".to_string()));
        assert_eq!(params.temperature, Some(0.5));
        assert!((params.min_similarity - 0.9).abs() < 0.01);
        assert_eq!(params.limit, 5);
    }

    #[test]
    fn test_semantic_cache_stats() {
        let stats = SemanticCacheStats {
            total_entries: 100,
            hits: 80,
            misses: 20,
            evictions: 5,
            avg_hit_similarity: 0.98,
        };

        assert!((stats.hit_rate() - 0.8).abs() < 0.01);
    }

    #[test]
    fn test_semantic_cache_stats_no_requests() {
        let stats = SemanticCacheStats::default();

        assert_eq!(stats.hit_rate(), 0.0);
    }

    #[test]
    fn test_deserialize_value() {
        #[derive(Debug, Deserialize, PartialEq)]
        struct TestValue {
            message: String,
        }

        let entry = CachedEntry::new(
            "test-id",
            vec![0.1],
            "query",
            r#"{"message": "hello"}"#,
            Duration::from_secs(3600),
        );

        let value: TestValue = entry.deserialize_value().unwrap();

        assert_eq!(value.message, "hello");
    }
}
