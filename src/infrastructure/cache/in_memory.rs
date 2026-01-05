//! In-memory cache implementation using moka

use std::time::Duration;

use async_trait::async_trait;
use moka::future::Cache as MokaCache;

use crate::domain::cache::{Cache, CacheExt};
use crate::domain::DomainError;

/// Configuration for in-memory cache
#[derive(Debug, Clone)]
pub struct InMemoryCacheConfig {
    /// Maximum number of entries
    pub max_capacity: u64,
    /// Default TTL for entries without explicit TTL
    pub default_ttl: Duration,
    /// Time to idle - entries not accessed for this duration are evicted
    pub time_to_idle: Option<Duration>,
}

impl Default for InMemoryCacheConfig {
    fn default() -> Self {
        Self {
            max_capacity: 10_000,
            default_ttl: Duration::from_secs(3600), // 1 hour
            time_to_idle: None,
        }
    }
}

impl InMemoryCacheConfig {
    /// Creates a new configuration with specified max capacity
    pub fn with_max_capacity(mut self, capacity: u64) -> Self {
        self.max_capacity = capacity;
        self
    }

    /// Sets the default TTL
    pub fn with_default_ttl(mut self, ttl: Duration) -> Self {
        self.default_ttl = ttl;
        self
    }

    /// Sets the time-to-idle duration
    pub fn with_time_to_idle(mut self, tti: Duration) -> Self {
        self.time_to_idle = Some(tti);
        self
    }
}

/// Cache entry stored in moka
#[derive(Debug, Clone)]
struct CacheEntry {
    /// Serialized JSON value
    data: String,
    /// Expiration timestamp (millis since epoch)
    expires_at: u64,
}

/// Thread-safe in-memory cache implementation using moka
///
/// Features:
/// - TTL support per entry
/// - LRU-like eviction when capacity is reached
/// - Concurrent access with good performance
/// - Optional time-to-idle eviction
#[derive(Debug)]
pub struct InMemoryCache {
    cache: MokaCache<String, CacheEntry>,
    config: InMemoryCacheConfig,
}

impl InMemoryCache {
    /// Creates a new in-memory cache with default configuration
    pub fn new() -> Self {
        Self::with_config(InMemoryCacheConfig::default())
    }

    /// Creates a new in-memory cache with the given configuration
    pub fn with_config(config: InMemoryCacheConfig) -> Self {
        let mut builder = MokaCache::builder()
            .max_capacity(config.max_capacity)
            .time_to_live(config.default_ttl);

        if let Some(tti) = config.time_to_idle {
            builder = builder.time_to_idle(tti);
        }

        Self {
            cache: builder.build(),
            config,
        }
    }

    fn current_time_millis() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }

    fn is_expired(entry: &CacheEntry) -> bool {
        Self::current_time_millis() > entry.expires_at
    }
}

impl Default for InMemoryCache {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Cache for InMemoryCache {
    async fn get_raw(&self, key: &str) -> Result<Option<String>, DomainError> {
        match self.cache.get(key).await {
            Some(entry) => {
                if Self::is_expired(&entry) {
                    self.cache.remove(key).await;
                    return Ok(None);
                }

                Ok(Some(entry.data.clone()))
            }
            None => Ok(None),
        }
    }

    async fn set_raw(&self, key: &str, value: &str, ttl: Duration) -> Result<(), DomainError> {
        let expires_at = Self::current_time_millis() + ttl.as_millis() as u64;
        let entry = CacheEntry {
            data: value.to_string(),
            expires_at,
        };

        self.cache.insert(key.to_string(), entry).await;
        Ok(())
    }

    async fn set_nx_raw(&self, key: &str, value: &str, ttl: Duration) -> Result<bool, DomainError> {
        // Check if key exists and is not expired
        if let Some(entry) = self.cache.get(key).await {
            if !Self::is_expired(&entry) {
                return Ok(false);
            }
        }

        self.set_raw(key, value, ttl).await?;
        Ok(true)
    }

    async fn delete(&self, key: &str) -> Result<bool, DomainError> {
        let existed = self.cache.get(key).await.is_some();
        self.cache.remove(key).await;
        Ok(existed)
    }

    async fn delete_pattern(&self, pattern: &str) -> Result<usize, DomainError> {
        let pattern_regex = pattern.replace('*', ".*");
        let regex = regex::Regex::new(&pattern_regex)
            .map_err(|e| DomainError::cache(format!("Invalid pattern: {}", e)))?;

        let mut deleted = 0;

        // Sync pending tasks first
        self.cache.run_pending_tasks().await;

        // Use blocking task to iterate over cache entries
        let cache_clone = self.cache.clone();
        let keys_to_delete: Vec<String> = tokio::task::spawn_blocking(move || {
            cache_clone
                .iter()
                .filter_map(|(k, _)| {
                    let key_str: &str = k.as_ref();

                    if regex.is_match(key_str) {
                        Some(key_str.to_string())
                    } else {
                        None
                    }
                })
                .collect()
        })
        .await
        .map_err(|e| DomainError::cache(format!("Failed to iterate cache: {}", e)))?;

        for key in keys_to_delete {
            self.cache.remove(&key).await;
            deleted += 1;
        }

        Ok(deleted)
    }

    async fn exists(&self, key: &str) -> Result<bool, DomainError> {
        match self.cache.get(key).await {
            Some(entry) => {
                if Self::is_expired(&entry) {
                    self.cache.remove(key).await;
                    Ok(false)
                } else {
                    Ok(true)
                }
            }
            None => Ok(false),
        }
    }

    async fn expire(&self, key: &str, ttl: Duration) -> Result<bool, DomainError> {
        match self.cache.get(key).await {
            Some(entry) => {
                if Self::is_expired(&entry) {
                    self.cache.remove(key).await;
                    return Ok(false);
                }

                let new_expires_at = Self::current_time_millis() + ttl.as_millis() as u64;
                let new_entry = CacheEntry {
                    data: entry.data.clone(),
                    expires_at: new_expires_at,
                };

                self.cache.insert(key.to_string(), new_entry).await;
                Ok(true)
            }
            None => Ok(false),
        }
    }

    async fn ttl(&self, key: &str) -> Result<Option<Duration>, DomainError> {
        match self.cache.get(key).await {
            Some(entry) => {
                let now = Self::current_time_millis();

                if entry.expires_at <= now {
                    self.cache.remove(key).await;
                    Ok(None)
                } else {
                    let remaining = entry.expires_at - now;
                    Ok(Some(Duration::from_millis(remaining)))
                }
            }
            None => Ok(None),
        }
    }

    async fn clear(&self) -> Result<(), DomainError> {
        self.cache.invalidate_all();
        self.cache.run_pending_tasks().await;
        Ok(())
    }

    async fn size(&self) -> Result<usize, DomainError> {
        self.cache.run_pending_tasks().await;
        Ok(self.cache.entry_count() as usize)
    }

    async fn increment(&self, key: &str, delta: i64) -> Result<i64, DomainError> {
        let current: i64 = self.get(key).await?.unwrap_or(0);
        let new_value = current + delta;

        // Use default TTL for counters
        self.set(key, &new_value, self.config.default_ttl).await?;
        Ok(new_value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_set_and_get() {
        let cache = InMemoryCache::new();

        cache
            .set("key1", &"value1", Duration::from_secs(60))
            .await
            .unwrap();

        let result: Option<String> = cache.get("key1").await.unwrap();
        assert_eq!(result, Some("value1".to_string()));
    }

    #[tokio::test]
    async fn test_get_missing() {
        let cache = InMemoryCache::new();

        let result: Option<String> = cache.get("missing").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_delete() {
        let cache = InMemoryCache::new();

        cache
            .set("key1", &"value1", Duration::from_secs(60))
            .await
            .unwrap();

        let deleted = cache.delete("key1").await.unwrap();
        assert!(deleted);

        let result: Option<String> = cache.get("key1").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_delete_missing() {
        let cache = InMemoryCache::new();

        let deleted = cache.delete("missing").await.unwrap();
        assert!(!deleted);
    }

    #[tokio::test]
    async fn test_exists() {
        let cache = InMemoryCache::new();

        cache
            .set("key1", &"value1", Duration::from_secs(60))
            .await
            .unwrap();

        assert!(cache.exists("key1").await.unwrap());
        assert!(!cache.exists("missing").await.unwrap());
    }

    #[tokio::test]
    async fn test_ttl_expiration() {
        let cache = InMemoryCache::new();

        // Set with very short TTL
        cache
            .set("key1", &"value1", Duration::from_millis(50))
            .await
            .unwrap();

        // Should exist immediately
        assert!(cache.exists("key1").await.unwrap());

        // Wait for expiration
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Should be expired
        let result: Option<String> = cache.get("key1").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_ttl_remaining() {
        let cache = InMemoryCache::new();

        cache
            .set("key1", &"value1", Duration::from_secs(60))
            .await
            .unwrap();

        let ttl = cache.ttl("key1").await.unwrap();
        assert!(ttl.is_some());

        let remaining = ttl.unwrap();
        assert!(remaining.as_secs() > 50 && remaining.as_secs() <= 60);
    }

    #[tokio::test]
    async fn test_expire() {
        let cache = InMemoryCache::new();

        cache
            .set("key1", &"value1", Duration::from_secs(60))
            .await
            .unwrap();

        // Update TTL to 2 seconds
        let updated = cache.expire("key1", Duration::from_secs(2)).await.unwrap();
        assert!(updated);

        let ttl = cache.ttl("key1").await.unwrap();
        assert!(ttl.is_some());
        assert!(ttl.unwrap().as_secs() <= 2);
    }

    #[tokio::test]
    async fn test_clear() {
        let cache = InMemoryCache::new();

        cache
            .set("key1", &"value1", Duration::from_secs(60))
            .await
            .unwrap();
        cache
            .set("key2", &"value2", Duration::from_secs(60))
            .await
            .unwrap();

        cache.clear().await.unwrap();

        let size = cache.size().await.unwrap();
        assert_eq!(size, 0);
    }

    #[tokio::test]
    async fn test_increment() {
        let cache = InMemoryCache::new();

        let val = cache.increment("counter", 5).await.unwrap();
        assert_eq!(val, 5);

        let val = cache.increment("counter", 3).await.unwrap();
        assert_eq!(val, 8);

        let val = cache.increment("counter", -2).await.unwrap();
        assert_eq!(val, 6);
    }

    #[tokio::test]
    async fn test_set_nx() {
        let cache = InMemoryCache::new();

        // First set should succeed
        let result = cache
            .set_nx("key1", &"value1", Duration::from_secs(60))
            .await
            .unwrap();
        assert!(result);

        // Second set should fail (key exists)
        let result = cache
            .set_nx("key1", &"value2", Duration::from_secs(60))
            .await
            .unwrap();
        assert!(!result);

        // Original value should remain
        let value: Option<String> = cache.get("key1").await.unwrap();
        assert_eq!(value, Some("value1".to_string()));
    }

    #[tokio::test]
    async fn test_complex_types() {
        let cache = InMemoryCache::new();

        #[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
        struct TestData {
            name: String,
            values: Vec<i32>,
        }

        let data = TestData {
            name: "test".to_string(),
            values: vec![1, 2, 3],
        };

        cache
            .set("complex", &data, Duration::from_secs(60))
            .await
            .unwrap();

        let result: Option<TestData> = cache.get("complex").await.unwrap();
        assert_eq!(result, Some(data));
    }

    #[tokio::test]
    async fn test_delete_pattern() {
        let cache = InMemoryCache::new();

        cache
            .set("user:1:profile", &"data1", Duration::from_secs(60))
            .await
            .unwrap();
        cache
            .set("user:2:profile", &"data2", Duration::from_secs(60))
            .await
            .unwrap();
        cache
            .set("other:key", &"data3", Duration::from_secs(60))
            .await
            .unwrap();

        // Wait for cache to sync
        tokio::time::sleep(Duration::from_millis(50)).await;

        let deleted = cache.delete_pattern("user:.*:profile").await.unwrap();
        assert_eq!(deleted, 2);

        // Wait for deletions to complete
        tokio::time::sleep(Duration::from_millis(50)).await;

        let size = cache.size().await.unwrap();
        assert_eq!(size, 1);
    }

    #[tokio::test]
    async fn test_config() {
        let config = InMemoryCacheConfig::default()
            .with_max_capacity(100)
            .with_default_ttl(Duration::from_secs(300))
            .with_time_to_idle(Duration::from_secs(60));

        let cache = InMemoryCache::with_config(config.clone());

        assert_eq!(cache.config.max_capacity, 100);
        assert_eq!(cache.config.default_ttl, Duration::from_secs(300));
        assert_eq!(cache.config.time_to_idle, Some(Duration::from_secs(60)));
    }

    #[tokio::test]
    async fn test_get_many() {
        let cache = InMemoryCache::new();

        cache
            .set("key1", &"value1", Duration::from_secs(60))
            .await
            .unwrap();
        cache
            .set("key2", &"value2", Duration::from_secs(60))
            .await
            .unwrap();

        let results: Vec<Option<String>> = cache.get_many(&["key1", "key2", "key3"]).await.unwrap();

        assert_eq!(results.len(), 3);
        assert_eq!(results[0], Some("value1".to_string()));
        assert_eq!(results[1], Some("value2".to_string()));
        assert_eq!(results[2], None);
    }

    #[tokio::test]
    async fn test_set_many() {
        let cache = InMemoryCache::new();

        let v1 = "value1".to_string();
        let v2 = "value2".to_string();
        let entries: Vec<(&str, &String)> = vec![("key1", &v1), ("key2", &v2)];

        cache
            .set_many(&entries, Duration::from_secs(60))
            .await
            .unwrap();

        let result1: Option<String> = cache.get("key1").await.unwrap();
        let result2: Option<String> = cache.get("key2").await.unwrap();

        assert_eq!(result1, Some("value1".to_string()));
        assert_eq!(result2, Some("value2".to_string()));
    }
}
