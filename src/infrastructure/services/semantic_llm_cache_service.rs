//! Semantic LLM response caching service
//!
//! Provides semantic caching for LLM responses, allowing cache hits
//! for semantically similar queries rather than requiring exact matches.

use std::sync::Arc;

use tracing::{debug, warn};
use uuid::Uuid;

use crate::domain::embedding::{EmbeddingProvider, EmbeddingRequest};
use crate::domain::llm::{LlmRequest, LlmResponse};
use crate::domain::semantic_cache::{
    CachedEntry, SemanticCache, SemanticCacheConfig, SemanticCacheStats, SemanticSearchParams,
};
use crate::domain::DomainError;

/// Semantic LLM cache service that uses embeddings for similarity matching
#[derive(Debug)]
pub struct SemanticLlmCacheService {
    cache: Arc<dyn SemanticCache>,
    embedding_provider: Arc<dyn EmbeddingProvider>,
    config: SemanticCacheConfig,
}

impl SemanticLlmCacheService {
    /// Create a new semantic LLM cache service
    pub fn new(
        cache: Arc<dyn SemanticCache>,
        embedding_provider: Arc<dyn EmbeddingProvider>,
    ) -> Self {
        Self::with_config(cache, embedding_provider, SemanticCacheConfig::default())
    }

    /// Create a new semantic LLM cache service with custom config
    pub fn with_config(
        cache: Arc<dyn SemanticCache>,
        embedding_provider: Arc<dyn EmbeddingProvider>,
        config: SemanticCacheConfig,
    ) -> Self {
        Self {
            cache,
            embedding_provider,
            config,
        }
    }

    /// Check if semantic caching is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Get the configuration
    pub fn config(&self) -> &SemanticCacheConfig {
        &self.config
    }

    /// Extract the query text from an LLM request for embedding
    fn extract_query_text(&self, request: &LlmRequest) -> String {
        // Concatenate all user messages as the query
        request
            .messages
            .iter()
            .filter(|m| m.role == crate::domain::llm::MessageRole::User)
            .filter_map(|m| m.content_text())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Generate an embedding for the given text
    async fn generate_embedding(&self, text: &str) -> Result<Vec<f32>, DomainError> {
        let request = EmbeddingRequest::single(&self.config.embedding_model, text);
        let response = self.embedding_provider.embed(request).await?;

        response
            .first()
            .map(|e| e.vector().to_vec())
            .ok_or_else(|| DomainError::internal("No embedding returned"))
    }

    /// Try to get a cached response for semantically similar queries
    pub async fn get(
        &self,
        model_id: &str,
        request: &LlmRequest,
    ) -> Result<Option<CachedLlmResponse>, DomainError> {
        if !self.config.enabled {
            return Ok(None);
        }

        // Don't cache streaming requests unless configured
        if request.stream && !self.config.cache_streaming {
            return Ok(None);
        }

        let query_text = self.extract_query_text(request);

        if query_text.is_empty() {
            return Ok(None);
        }

        // Generate embedding for the query
        let embedding = match self.generate_embedding(&query_text).await {
            Ok(emb) => emb,
            Err(e) => {
                warn!("Failed to generate embedding for cache lookup: {}", e);
                self.cache.record_miss().await?;
                return Ok(None);
            }
        };

        // Build search params
        let mut params = SemanticSearchParams::new(self.config.similarity_threshold);

        if self.config.include_model_in_key {
            params = params.with_model_id(model_id);
        }

        if self.config.include_temperature_in_key {
            if let Some(temp) = request.temperature {
                params = params.with_temperature(temp);
            }
        }

        // Search for similar cached entries
        let result = self.cache.find_similar(&embedding, &params).await?;

        match result {
            Some(search_result) => {
                debug!(
                    "Semantic cache hit with similarity {:.4} for entry {}",
                    search_result.similarity,
                    search_result.entry.id()
                );

                self.cache.record_hit(search_result.entry.id()).await?;

                let response: LlmResponse = search_result.entry.deserialize_value()?;

                Ok(Some(CachedLlmResponse {
                    response,
                    model_id: search_result
                        .entry
                        .model_id()
                        .unwrap_or(model_id)
                        .to_string(),
                    similarity: search_result.similarity,
                    cached_at: search_result.entry.created_at(),
                    hit_count: search_result.entry.hit_count() + 1,
                }))
            }
            None => {
                debug!("Semantic cache miss for query: {}...", &query_text[..query_text.len().min(50)]);
                self.cache.record_miss().await?;
                Ok(None)
            }
        }
    }

    /// Cache a response for the given request
    pub async fn set(
        &self,
        model_id: &str,
        request: &LlmRequest,
        response: LlmResponse,
    ) -> Result<(), DomainError> {
        if !self.config.enabled {
            return Ok(());
        }

        // Don't cache streaming requests unless configured
        if request.stream && !self.config.cache_streaming {
            return Ok(());
        }

        let query_text = self.extract_query_text(request);

        if query_text.is_empty() {
            return Ok(());
        }

        // Generate embedding for the query
        let embedding = match self.generate_embedding(&query_text).await {
            Ok(emb) => emb,
            Err(e) => {
                warn!("Failed to generate embedding for caching: {}", e);
                return Ok(());
            }
        };

        // Serialize the response
        let value = serde_json::to_string(&response).map_err(|e| {
            DomainError::internal(format!("Failed to serialize response for cache: {}", e))
        })?;

        // Create cache entry
        let entry_id = format!("sem:{}", Uuid::new_v4());
        let mut entry = CachedEntry::new(entry_id, embedding, query_text, value, self.config.ttl());

        if self.config.include_model_in_key {
            entry = entry.with_model_id(model_id);
        }

        if self.config.include_temperature_in_key {
            if let Some(temp) = request.temperature {
                entry = entry.with_temperature(temp);
            }
        }

        // Store in cache
        self.cache.store(entry).await?;

        debug!("Cached LLM response for model {}", model_id);

        Ok(())
    }

    /// Invalidate all cached responses for a model
    pub async fn invalidate_model(&self, model_id: &str) -> Result<usize, DomainError> {
        self.cache.delete_by_model(model_id).await
    }

    /// Invalidate all cached responses
    pub async fn invalidate_all(&self) -> Result<(), DomainError> {
        self.cache.clear().await
    }

    /// Get cache statistics
    pub async fn stats(&self) -> Result<SemanticCacheStats, DomainError> {
        self.cache.stats().await
    }

    /// Clean up expired entries
    pub async fn cleanup(&self) -> Result<usize, DomainError> {
        self.cache.cleanup_expired().await
    }
}

/// Cached LLM response with semantic cache metadata
#[derive(Debug, Clone)]
pub struct CachedLlmResponse {
    /// The cached response
    pub response: LlmResponse,
    /// Model ID that produced this response
    pub model_id: String,
    /// Similarity score of the cache hit
    pub similarity: f32,
    /// Unix timestamp when cached
    pub cached_at: u64,
    /// Cache hit count
    pub hit_count: u32,
}

/// Trait for semantic LLM cache service operations
#[async_trait::async_trait]
pub trait SemanticLlmCacheServiceTrait: Send + Sync + std::fmt::Debug {
    /// Check if semantic caching is enabled
    fn is_enabled(&self) -> bool;

    /// Try to get a cached response for semantically similar queries
    async fn get(
        &self,
        model_id: &str,
        request: &LlmRequest,
    ) -> Result<Option<CachedLlmResponse>, DomainError>;

    /// Cache a response for the given request
    async fn set(
        &self,
        model_id: &str,
        request: &LlmRequest,
        response: LlmResponse,
    ) -> Result<(), DomainError>;

    /// Invalidate all cached responses for a model
    async fn invalidate_model(&self, model_id: &str) -> Result<usize, DomainError>;

    /// Invalidate all cached responses
    async fn invalidate_all(&self) -> Result<(), DomainError>;

    /// Get cache statistics
    async fn stats(&self) -> Result<SemanticCacheStats, DomainError>;
}

#[async_trait::async_trait]
impl SemanticLlmCacheServiceTrait for SemanticLlmCacheService {
    fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    async fn get(
        &self,
        model_id: &str,
        request: &LlmRequest,
    ) -> Result<Option<CachedLlmResponse>, DomainError> {
        SemanticLlmCacheService::get(self, model_id, request).await
    }

    async fn set(
        &self,
        model_id: &str,
        request: &LlmRequest,
        response: LlmResponse,
    ) -> Result<(), DomainError> {
        SemanticLlmCacheService::set(self, model_id, request, response).await
    }

    async fn invalidate_model(&self, model_id: &str) -> Result<usize, DomainError> {
        SemanticLlmCacheService::invalidate_model(self, model_id).await
    }

    async fn invalidate_all(&self) -> Result<(), DomainError> {
        SemanticLlmCacheService::invalidate_all(self).await
    }

    async fn stats(&self) -> Result<SemanticCacheStats, DomainError> {
        SemanticLlmCacheService::stats(self).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::embedding::MockEmbeddingProvider;
    use crate::domain::llm::Message;
    use crate::infrastructure::semantic_cache::InMemorySemanticCache;
    use std::time::Duration;

    fn create_test_request(user_message: &str) -> LlmRequest {
        LlmRequest::builder()
            .system("You are a helpful assistant")
            .user(user_message)
            .build()
    }

    fn create_test_response() -> LlmResponse {
        LlmResponse::new(
            "resp-123".to_string(),
            "gpt-4".to_string(),
            Message::assistant("Hello! How can I help you?"),
        )
    }

    fn create_service() -> SemanticLlmCacheService {
        let cache = Arc::new(InMemorySemanticCache::new(100));
        let embedding_provider = Arc::new(MockEmbeddingProvider::new("mock", 128));
        let config = SemanticCacheConfig::new()
            .with_similarity_threshold(0.9)
            .with_ttl(Duration::from_secs(3600));

        SemanticLlmCacheService::with_config(cache, embedding_provider, config)
    }

    #[tokio::test]
    async fn test_cache_set_and_get() {
        let service = create_service();
        let request = create_test_request("Hello world!");
        let response = create_test_response();

        // Cache the response
        service.set("gpt-4", &request, response.clone()).await.unwrap();

        // Retrieve with same query
        let cached = service.get("gpt-4", &request).await.unwrap();
        assert!(cached.is_some());

        let cached = cached.unwrap();
        assert_eq!(cached.response.id, "resp-123");
        assert!(cached.similarity > 0.99); // Should be very high for identical query
    }

    #[tokio::test]
    async fn test_cache_miss_different_query() {
        let service = create_service();
        let request1 = create_test_request("Hello world!");
        let request2 = create_test_request("Goodbye universe!");
        let response = create_test_response();

        // Cache response for request1
        service.set("gpt-4", &request1, response).await.unwrap();

        // Try to get with different query (should miss due to low similarity)
        let cached = service.get("gpt-4", &request2).await.unwrap();
        assert!(cached.is_none());
    }

    #[tokio::test]
    async fn test_cache_disabled() {
        let cache = Arc::new(InMemorySemanticCache::new(100));
        let embedding_provider = Arc::new(MockEmbeddingProvider::new("mock", 128));
        let config = SemanticCacheConfig::new().with_enabled(false);
        let service = SemanticLlmCacheService::with_config(cache, embedding_provider, config);

        let request = create_test_request("Hello!");
        let response = create_test_response();

        // Try to cache (should be no-op)
        service.set("gpt-4", &request, response).await.unwrap();

        // Should return None
        let cached = service.get("gpt-4", &request).await.unwrap();
        assert!(cached.is_none());
    }

    #[tokio::test]
    async fn test_streaming_not_cached_by_default() {
        let service = create_service();
        let request = LlmRequest::builder()
            .user("Hello!")
            .stream(true)
            .build();
        let response = create_test_response();

        // Try to cache streaming request
        service.set("gpt-4", &request, response).await.unwrap();

        // Should return None
        let cached = service.get("gpt-4", &request).await.unwrap();
        assert!(cached.is_none());
    }

    #[tokio::test]
    async fn test_model_filtering() {
        let cache = Arc::new(InMemorySemanticCache::new(100));
        let embedding_provider = Arc::new(MockEmbeddingProvider::new("mock", 128));
        let config = SemanticCacheConfig::new()
            .with_similarity_threshold(0.9)
            .with_include_model(true);
        let service = SemanticLlmCacheService::with_config(cache, embedding_provider, config);

        let request = create_test_request("Hello!");
        let response = create_test_response();

        // Cache for gpt-4
        service.set("gpt-4", &request, response.clone()).await.unwrap();

        // Hit for gpt-4
        let cached = service.get("gpt-4", &request).await.unwrap();
        assert!(cached.is_some());

        // Miss for gpt-3.5 (different model)
        let cached = service.get("gpt-3.5", &request).await.unwrap();
        assert!(cached.is_none());
    }

    #[tokio::test]
    async fn test_invalidate_model() {
        let service = create_service();
        let request1 = create_test_request("Query 1");
        let request2 = create_test_request("Query 2");
        let response = create_test_response();

        // Cache responses
        service.set("gpt-4", &request1, response.clone()).await.unwrap();
        service.set("gpt-4", &request2, response.clone()).await.unwrap();

        // Invalidate model
        let deleted = service.invalidate_model("gpt-4").await.unwrap();
        assert_eq!(deleted, 2);

        // Both should be gone
        assert!(service.get("gpt-4", &request1).await.unwrap().is_none());
        assert!(service.get("gpt-4", &request2).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_stats() {
        let service = create_service();
        let request = create_test_request("Hello!");
        let response = create_test_response();

        // Cache and hit
        service.set("gpt-4", &request, response).await.unwrap();
        let _ = service.get("gpt-4", &request).await.unwrap();

        // Get stats
        let stats = service.stats().await.unwrap();
        assert_eq!(stats.total_entries, 1);
        assert_eq!(stats.hits, 1);
    }

    #[tokio::test]
    async fn test_empty_query_not_cached() {
        let service = create_service();
        let request = LlmRequest::builder()
            .system("System only")
            .build();
        let response = create_test_response();

        // Empty user query should not be cached
        service.set("gpt-4", &request, response).await.unwrap();
        let cached = service.get("gpt-4", &request).await.unwrap();
        assert!(cached.is_none());
    }
}
