//! LLM response caching service

use std::sync::Arc;
use std::time::Duration;

use crate::domain::cache::{Cache, CacheExt, CacheKeyGenerator, CacheKeyParams, DefaultKeyGenerator};
use crate::domain::llm::{LlmRequest, LlmResponse};
use crate::domain::DomainError;

/// Configuration for LLM response caching
#[derive(Debug, Clone)]
pub struct LlmCacheConfig {
    /// Namespace prefix for cache keys
    pub namespace: String,
    /// Default TTL for cached responses
    pub default_ttl: Duration,
    /// Whether to include temperature in cache key (false = cache regardless of temp)
    pub include_temperature_in_key: bool,
    /// Whether to include max_tokens in cache key
    pub include_max_tokens_in_key: bool,
    /// Whether caching is enabled
    pub enabled: bool,
}

impl Default for LlmCacheConfig {
    fn default() -> Self {
        Self {
            namespace: "llm:responses".to_string(),
            default_ttl: Duration::from_secs(3600), // 1 hour
            include_temperature_in_key: false,
            include_max_tokens_in_key: false,
            enabled: true,
        }
    }
}

impl LlmCacheConfig {
    /// Creates a new config with the given namespace
    pub fn with_namespace(mut self, namespace: impl Into<String>) -> Self {
        self.namespace = namespace.into();
        self
    }

    /// Sets the default TTL
    pub fn with_default_ttl(mut self, ttl: Duration) -> Self {
        self.default_ttl = ttl;
        self
    }

    /// Includes temperature in cache key generation
    pub fn with_temperature_in_key(mut self) -> Self {
        self.include_temperature_in_key = true;
        self
    }

    /// Includes max_tokens in cache key generation
    pub fn with_max_tokens_in_key(mut self) -> Self {
        self.include_max_tokens_in_key = true;
        self
    }

    /// Disables caching
    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }
}

/// Cached LLM response with metadata
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CachedLlmResponse {
    /// The cached response
    pub response: LlmResponse,
    /// Model ID that produced this response
    pub model_id: String,
    /// Unix timestamp when cached
    pub cached_at: u64,
    /// Cache hit count (updated on retrieval)
    pub hit_count: u32,
}

impl CachedLlmResponse {
    fn new(response: LlmResponse, model_id: String) -> Self {
        Self {
            response,
            model_id,
            cached_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            hit_count: 0,
        }
    }

    fn increment_hits(mut self) -> Self {
        self.hit_count += 1;
        self
    }
}

/// Service for caching LLM responses
#[derive(Debug)]
pub struct LlmCacheService {
    cache: Arc<dyn Cache>,
    config: LlmCacheConfig,
    key_generator: DefaultKeyGenerator,
}

impl LlmCacheService {
    /// Creates a new LLM cache service
    pub fn new(cache: Arc<dyn Cache>) -> Self {
        Self::with_config(cache, LlmCacheConfig::default())
    }

    /// Creates a new LLM cache service with custom config
    pub fn with_config(cache: Arc<dyn Cache>, config: LlmCacheConfig) -> Self {
        Self {
            cache,
            config,
            key_generator: DefaultKeyGenerator::new().with_short_hash(),
        }
    }

    /// Generates a cache key for the given request and model
    pub fn generate_cache_key(&self, model_id: &str, request: &LlmRequest) -> String {
        let mut params = CacheKeyParams::new(model_id);

        // Add message content to key
        let messages_json =
            serde_json::to_string(&request.messages).unwrap_or_else(|_| "[]".to_string());
        params = params.with_component("messages", messages_json);

        // Add prompt reference if present
        if let Some(prompt_id) = &request.system_prompt_id {
            params = params.with_component("prompt_id", prompt_id.clone());
        }

        // Add prompt variables if present
        if let Some(vars) = &request.prompt_variables {
            let vars_json = serde_json::to_string(vars).unwrap_or_else(|_| "{}".to_string());
            params = params.with_component("prompt_vars", vars_json);
        }

        // Optionally include temperature
        if self.config.include_temperature_in_key {
            if let Some(temp) = request.temperature {
                params = params.with_component("temperature", format!("{:.2}", temp));
            }
        }

        // Optionally include max_tokens
        if self.config.include_max_tokens_in_key {
            if let Some(tokens) = request.max_tokens {
                params = params.with_component("max_tokens", tokens.to_string());
            }
        }

        self.key_generator
            .generate_with_namespace(&self.config.namespace, &params)
    }

    /// Tries to get a cached response for the given request
    pub async fn get(
        &self,
        model_id: &str,
        request: &LlmRequest,
    ) -> Result<Option<CachedLlmResponse>, DomainError> {
        if !self.config.enabled || request.stream {
            return Ok(None);
        }

        let key = self.generate_cache_key(model_id, request);
        let result: Option<CachedLlmResponse> = self.cache.get(&key).await?;

        // Update hit count if found
        if let Some(cached) = result {
            let updated = cached.increment_hits();

            // Fire-and-forget update (don't block on this)
            let cache = self.cache.clone();
            let key_clone = key.clone();
            let updated_clone = updated.clone();
            let ttl = self.config.default_ttl;

            tokio::spawn(async move {
                let _ = cache.set(&key_clone, &updated_clone, ttl).await;
            });

            return Ok(Some(updated));
        }

        Ok(None)
    }

    /// Caches a response for the given request
    pub async fn set(
        &self,
        model_id: &str,
        request: &LlmRequest,
        response: LlmResponse,
    ) -> Result<(), DomainError> {
        if !self.config.enabled || request.stream {
            return Ok(());
        }

        let key = self.generate_cache_key(model_id, request);
        let cached = CachedLlmResponse::new(response, model_id.to_string());

        self.cache.set(&key, &cached, self.config.default_ttl).await
    }

    /// Caches a response with custom TTL
    pub async fn set_with_ttl(
        &self,
        model_id: &str,
        request: &LlmRequest,
        response: LlmResponse,
        ttl: Duration,
    ) -> Result<(), DomainError> {
        if !self.config.enabled || request.stream {
            return Ok(());
        }

        let key = self.generate_cache_key(model_id, request);
        let cached = CachedLlmResponse::new(response, model_id.to_string());

        self.cache.set(&key, &cached, ttl).await
    }

    /// Invalidates a specific cached response
    pub async fn invalidate(
        &self,
        model_id: &str,
        request: &LlmRequest,
    ) -> Result<bool, DomainError> {
        let key = self.generate_cache_key(model_id, request);
        self.cache.delete(&key).await
    }

    /// Invalidates all cached responses for a model
    pub async fn invalidate_model(&self, model_id: &str) -> Result<usize, DomainError> {
        let pattern = format!("{}:{}*", self.config.namespace, model_id);
        self.cache.delete_pattern(&pattern).await
    }

    /// Invalidates all cached responses
    pub async fn invalidate_all(&self) -> Result<usize, DomainError> {
        let pattern = format!("{}:*", self.config.namespace);
        self.cache.delete_pattern(&pattern).await
    }

    /// Returns cache statistics
    pub async fn stats(&self) -> Result<CacheStats, DomainError> {
        let size = self.cache.size().await?;
        Ok(CacheStats {
            entries: size,
            enabled: self.config.enabled,
        })
    }

    /// Checks if caching is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    /// Number of entries in cache
    pub entries: usize,
    /// Whether caching is enabled
    pub enabled: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::cache::MockCache;
    use crate::domain::llm::Message;

    fn create_test_request() -> LlmRequest {
        LlmRequest::builder()
            .system("You are a helpful assistant")
            .user("Hello!")
            .build()
    }

    fn create_test_response() -> LlmResponse {
        LlmResponse::new(
            "resp-123".to_string(),
            "gpt-4".to_string(),
            Message::assistant("Hello! How can I help you?"),
        )
    }

    #[tokio::test]
    async fn test_cache_set_and_get() {
        let cache = Arc::new(MockCache::new());
        let service = LlmCacheService::new(cache);

        let request = create_test_request();
        let response = create_test_response();

        // Cache the response
        service
            .set("gpt-4", &request, response.clone())
            .await
            .unwrap();

        // Retrieve from cache
        let cached = service.get("gpt-4", &request).await.unwrap();
        assert!(cached.is_some());

        let cached = cached.unwrap();
        assert_eq!(cached.response.id, "resp-123");
        assert_eq!(cached.model_id, "gpt-4");
    }

    #[tokio::test]
    async fn test_cache_miss() {
        let cache = Arc::new(MockCache::new());
        let service = LlmCacheService::new(cache);

        let request = create_test_request();

        let cached = service.get("gpt-4", &request).await.unwrap();
        assert!(cached.is_none());
    }

    #[tokio::test]
    async fn test_cache_disabled() {
        let cache = Arc::new(MockCache::new());
        let config = LlmCacheConfig::default().disabled();
        let service = LlmCacheService::with_config(cache, config);

        let request = create_test_request();
        let response = create_test_response();

        // Try to cache (should be no-op)
        service
            .set("gpt-4", &request, response)
            .await
            .unwrap();

        // Should return None even if we tried to cache
        let cached = service.get("gpt-4", &request).await.unwrap();
        assert!(cached.is_none());
    }

    #[tokio::test]
    async fn test_streaming_requests_not_cached() {
        let cache = Arc::new(MockCache::new());
        let service = LlmCacheService::new(cache);

        let request = LlmRequest::builder()
            .user("Hello!")
            .stream(true)
            .build();
        let response = create_test_response();

        // Try to cache (should be no-op for streaming)
        service
            .set("gpt-4", &request, response)
            .await
            .unwrap();

        let cached = service.get("gpt-4", &request).await.unwrap();
        assert!(cached.is_none());
    }

    #[tokio::test]
    async fn test_cache_key_differs_by_model() {
        let cache = Arc::new(MockCache::new());
        let service = LlmCacheService::new(cache);

        let request = create_test_request();
        let response = create_test_response();

        // Cache for gpt-4
        service
            .set("gpt-4", &request, response.clone())
            .await
            .unwrap();

        // Cache miss for gpt-3.5-turbo
        let cached = service.get("gpt-3.5-turbo", &request).await.unwrap();
        assert!(cached.is_none());

        // Cache hit for gpt-4
        let cached = service.get("gpt-4", &request).await.unwrap();
        assert!(cached.is_some());
    }

    #[tokio::test]
    async fn test_cache_key_differs_by_messages() {
        let cache = Arc::new(MockCache::new());
        let service = LlmCacheService::new(cache);

        let request1 = LlmRequest::builder().user("Hello!").build();
        let request2 = LlmRequest::builder().user("Goodbye!").build();
        let response = create_test_response();

        // Cache request1
        service
            .set("gpt-4", &request1, response.clone())
            .await
            .unwrap();

        // Cache miss for request2
        let cached = service.get("gpt-4", &request2).await.unwrap();
        assert!(cached.is_none());

        // Cache hit for request1
        let cached = service.get("gpt-4", &request1).await.unwrap();
        assert!(cached.is_some());
    }

    #[tokio::test]
    async fn test_invalidate() {
        let cache = Arc::new(MockCache::new());
        let service = LlmCacheService::new(cache);

        let request = create_test_request();
        let response = create_test_response();

        // Cache the response
        service
            .set("gpt-4", &request, response)
            .await
            .unwrap();

        // Verify it's cached
        let cached = service.get("gpt-4", &request).await.unwrap();
        assert!(cached.is_some());

        // Invalidate
        let deleted = service.invalidate("gpt-4", &request).await.unwrap();
        assert!(deleted);

        // Verify it's gone
        let cached = service.get("gpt-4", &request).await.unwrap();
        assert!(cached.is_none());
    }

    #[tokio::test]
    async fn test_cache_key_with_temperature() {
        let cache = Arc::new(MockCache::new());
        let config = LlmCacheConfig::default().with_temperature_in_key();
        let service = LlmCacheService::with_config(cache, config);

        let request1 = LlmRequest::builder()
            .user("Hello!")
            .temperature(0.7)
            .build();
        let request2 = LlmRequest::builder()
            .user("Hello!")
            .temperature(0.9)
            .build();

        let key1 = service.generate_cache_key("gpt-4", &request1);
        let key2 = service.generate_cache_key("gpt-4", &request2);

        // Keys should be different when temperature is included
        assert_ne!(key1, key2);
    }

    #[tokio::test]
    async fn test_stats() {
        let cache = Arc::new(MockCache::new());
        let service = LlmCacheService::new(cache);

        let stats = service.stats().await.unwrap();
        assert_eq!(stats.entries, 0);
        assert!(stats.enabled);
    }
}
