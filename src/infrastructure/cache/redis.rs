//! Redis cache implementation

use std::fmt;
use std::time::Duration;

use async_trait::async_trait;
use redis::aio::ConnectionManager;
use redis::{AsyncCommands, Client};

use crate::domain::cache::Cache;
use crate::domain::DomainError;

/// Configuration for Redis cache
#[derive(Debug, Clone)]
pub struct RedisCacheConfig {
    /// Redis connection URL (e.g., "redis://127.0.0.1:6379")
    pub url: String,
    /// Default TTL for entries without explicit TTL
    pub default_ttl: Duration,
    /// Key prefix for namespacing
    pub key_prefix: Option<String>,
    /// Connection timeout
    pub connection_timeout: Duration,
}

impl Default for RedisCacheConfig {
    fn default() -> Self {
        Self {
            url: "redis://127.0.0.1:6379".to_string(),
            default_ttl: Duration::from_secs(3600),
            key_prefix: None,
            connection_timeout: Duration::from_secs(5),
        }
    }
}

impl RedisCacheConfig {
    /// Creates a new configuration with the given URL
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            ..Default::default()
        }
    }

    /// Sets the default TTL
    pub fn with_default_ttl(mut self, ttl: Duration) -> Self {
        self.default_ttl = ttl;
        self
    }

    /// Sets the key prefix
    pub fn with_key_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.key_prefix = Some(prefix.into());
        self
    }

    /// Sets the connection timeout
    pub fn with_connection_timeout(mut self, timeout: Duration) -> Self {
        self.connection_timeout = timeout;
        self
    }
}

/// Redis cache implementation
///
/// Features:
/// - TTL support per entry
/// - Atomic operations (SETNX, INCR)
/// - Pattern-based key deletion
/// - Connection pooling via ConnectionManager
#[derive(Clone)]
pub struct RedisCache {
    connection: ConnectionManager,
    config: RedisCacheConfig,
}

impl fmt::Debug for RedisCache {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RedisCache")
            .field("config", &self.config)
            .field("connection", &"<ConnectionManager>")
            .finish()
    }
}

impl RedisCache {
    /// Creates a new Redis cache connection
    pub async fn new(config: RedisCacheConfig) -> Result<Self, DomainError> {
        let client = Client::open(config.url.as_str())
            .map_err(|e| DomainError::cache(format!("Failed to create Redis client: {}", e)))?;

        let connection = ConnectionManager::new(client)
            .await
            .map_err(|e| DomainError::cache(format!("Failed to connect to Redis: {}", e)))?;

        Ok(Self { connection, config })
    }

    /// Creates a Redis cache with default configuration
    pub async fn with_url(url: impl Into<String>) -> Result<Self, DomainError> {
        Self::new(RedisCacheConfig::new(url)).await
    }

    fn prefix_key(&self, key: &str) -> String {
        match &self.config.key_prefix {
            Some(prefix) => format!("{}:{}", prefix, key),
            None => key.to_string(),
        }
    }
}

#[async_trait]
impl Cache for RedisCache {
    async fn get_raw(&self, key: &str) -> Result<Option<String>, DomainError> {
        let prefixed_key = self.prefix_key(key);
        let mut conn = self.connection.clone();

        let result: Option<String> = conn.get(&prefixed_key).await.map_err(|e| {
            DomainError::cache(format!("Failed to get key '{}': {}", key, e))
        })?;

        Ok(result)
    }

    async fn set_raw(&self, key: &str, value: &str, ttl: Duration) -> Result<(), DomainError> {
        let prefixed_key = self.prefix_key(key);
        let mut conn = self.connection.clone();

        let ttl_secs = ttl.as_secs().max(1) as u64;

        let _: () = conn
            .set_ex(&prefixed_key, value, ttl_secs)
            .await
            .map_err(|e| DomainError::cache(format!("Failed to set key '{}': {}", key, e)))?;

        Ok(())
    }

    async fn set_nx_raw(&self, key: &str, value: &str, ttl: Duration) -> Result<bool, DomainError> {
        let prefixed_key = self.prefix_key(key);
        let mut conn = self.connection.clone();

        let ttl_secs = ttl.as_secs().max(1) as u64;

        // Use SET NX EX for atomic set-if-not-exists with TTL
        let result: Option<String> = redis::cmd("SET")
            .arg(&prefixed_key)
            .arg(value)
            .arg("NX")
            .arg("EX")
            .arg(ttl_secs)
            .query_async(&mut conn)
            .await
            .map_err(|e| DomainError::cache(format!("Failed to set_nx key '{}': {}", key, e)))?;

        // Redis returns "OK" if set, None if key existed
        Ok(result.is_some())
    }

    async fn delete(&self, key: &str) -> Result<bool, DomainError> {
        let prefixed_key = self.prefix_key(key);
        let mut conn = self.connection.clone();

        let deleted: i32 = conn.del(&prefixed_key).await.map_err(|e| {
            DomainError::cache(format!("Failed to delete key '{}': {}", key, e))
        })?;

        Ok(deleted > 0)
    }

    async fn delete_pattern(&self, pattern: &str) -> Result<usize, DomainError> {
        let prefixed_pattern = self.prefix_key(pattern);
        let mut conn = self.connection.clone();

        // Use SCAN to find matching keys (safer than KEYS for production)
        let mut cursor = 0u64;
        let mut total_deleted = 0usize;

        loop {
            let (new_cursor, keys): (u64, Vec<String>) = redis::cmd("SCAN")
                .arg(cursor)
                .arg("MATCH")
                .arg(&prefixed_pattern)
                .arg("COUNT")
                .arg(100)
                .query_async(&mut conn)
                .await
                .map_err(|e| {
                    DomainError::cache(format!(
                        "Failed to scan keys with pattern '{}': {}",
                        pattern, e
                    ))
                })?;

            if !keys.is_empty() {
                let deleted: i32 = conn.del(&keys).await.map_err(|e| {
                    DomainError::cache(format!("Failed to delete keys: {}", e))
                })?;
                total_deleted += deleted as usize;
            }

            cursor = new_cursor;

            if cursor == 0 {
                break;
            }
        }

        Ok(total_deleted)
    }

    async fn exists(&self, key: &str) -> Result<bool, DomainError> {
        let prefixed_key = self.prefix_key(key);
        let mut conn = self.connection.clone();

        let exists: bool = conn.exists(&prefixed_key).await.map_err(|e| {
            DomainError::cache(format!("Failed to check existence of key '{}': {}", key, e))
        })?;

        Ok(exists)
    }

    async fn expire(&self, key: &str, ttl: Duration) -> Result<bool, DomainError> {
        let prefixed_key = self.prefix_key(key);
        let mut conn = self.connection.clone();

        let ttl_secs = ttl.as_secs().max(1) as i64;

        let updated: bool = conn.expire(&prefixed_key, ttl_secs).await.map_err(|e| {
            DomainError::cache(format!("Failed to update TTL for key '{}': {}", key, e))
        })?;

        Ok(updated)
    }

    async fn ttl(&self, key: &str) -> Result<Option<Duration>, DomainError> {
        let prefixed_key = self.prefix_key(key);
        let mut conn = self.connection.clone();

        let ttl_secs: i64 = conn.ttl(&prefixed_key).await.map_err(|e| {
            DomainError::cache(format!("Failed to get TTL for key '{}': {}", key, e))
        })?;

        // Redis returns -2 if key doesn't exist, -1 if no TTL
        if ttl_secs < 0 {
            Ok(None)
        } else {
            Ok(Some(Duration::from_secs(ttl_secs as u64)))
        }
    }

    async fn clear(&self) -> Result<(), DomainError> {
        // If we have a prefix, only clear prefixed keys
        // Otherwise, flush the entire database (use with caution!)
        match &self.config.key_prefix {
            Some(_) => {
                self.delete_pattern("*").await?;
            }
            None => {
                let mut conn = self.connection.clone();
                redis::cmd("FLUSHDB")
                    .query_async::<()>(&mut conn)
                    .await
                    .map_err(|e| DomainError::cache(format!("Failed to flush database: {}", e)))?;
            }
        }

        Ok(())
    }

    async fn size(&self) -> Result<usize, DomainError> {
        let mut conn = self.connection.clone();

        match &self.config.key_prefix {
            Some(_) => {
                // Count keys matching our prefix
                let pattern = self.prefix_key("*");
                let mut cursor = 0u64;
                let mut count = 0usize;

                loop {
                    let (new_cursor, keys): (u64, Vec<String>) = redis::cmd("SCAN")
                        .arg(cursor)
                        .arg("MATCH")
                        .arg(&pattern)
                        .arg("COUNT")
                        .arg(1000)
                        .query_async(&mut conn)
                        .await
                        .map_err(|e| DomainError::cache(format!("Failed to scan keys: {}", e)))?;

                    count += keys.len();
                    cursor = new_cursor;

                    if cursor == 0 {
                        break;
                    }
                }

                Ok(count)
            }
            None => {
                let size: usize = redis::cmd("DBSIZE")
                    .query_async(&mut conn)
                    .await
                    .map_err(|e| {
                        DomainError::cache(format!("Failed to get database size: {}", e))
                    })?;
                Ok(size)
            }
        }
    }

    async fn increment(&self, key: &str, delta: i64) -> Result<i64, DomainError> {
        let prefixed_key = self.prefix_key(key);
        let mut conn = self.connection.clone();

        let new_value: i64 = conn.incr(&prefixed_key, delta).await.map_err(|e| {
            DomainError::cache(format!("Failed to increment key '{}': {}", key, e))
        })?;

        Ok(new_value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::cache::CacheExt;

    // Note: These tests require a running Redis instance
    // Run with: cargo test --features redis-tests -- --ignored

    fn get_test_config() -> RedisCacheConfig {
        RedisCacheConfig::new("redis://127.0.0.1:6379")
            .with_key_prefix("test")
            .with_default_ttl(Duration::from_secs(60))
    }

    #[tokio::test]
    #[ignore = "Requires running Redis instance"]
    async fn test_redis_set_and_get() {
        let cache = RedisCache::new(get_test_config()).await.unwrap();

        cache
            .set("key1", &"value1", Duration::from_secs(60))
            .await
            .unwrap();

        let result: Option<String> = cache.get("key1").await.unwrap();
        assert_eq!(result, Some("value1".to_string()));

        // Cleanup
        cache.delete("key1").await.unwrap();
    }

    #[tokio::test]
    #[ignore = "Requires running Redis instance"]
    async fn test_redis_delete() {
        let cache = RedisCache::new(get_test_config()).await.unwrap();

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
    #[ignore = "Requires running Redis instance"]
    async fn test_redis_increment() {
        let cache = RedisCache::new(get_test_config()).await.unwrap();

        let val = cache.increment("counter", 5).await.unwrap();
        assert_eq!(val, 5);

        let val = cache.increment("counter", 3).await.unwrap();
        assert_eq!(val, 8);

        // Cleanup
        cache.delete("counter").await.unwrap();
    }

    #[tokio::test]
    #[ignore = "Requires running Redis instance"]
    async fn test_redis_set_nx() {
        let cache = RedisCache::new(get_test_config()).await.unwrap();

        // First set should succeed
        let result = cache
            .set_nx("nx_key", &"value1", Duration::from_secs(60))
            .await
            .unwrap();
        assert!(result);

        // Second set should fail
        let result = cache
            .set_nx("nx_key", &"value2", Duration::from_secs(60))
            .await
            .unwrap();
        assert!(!result);

        // Cleanup
        cache.delete("nx_key").await.unwrap();
    }

    #[tokio::test]
    #[ignore = "Requires running Redis instance"]
    async fn test_redis_ttl() {
        let cache = RedisCache::new(get_test_config()).await.unwrap();

        cache
            .set("ttl_key", &"value1", Duration::from_secs(60))
            .await
            .unwrap();

        let ttl = cache.ttl("ttl_key").await.unwrap();
        assert!(ttl.is_some());
        assert!(ttl.unwrap().as_secs() > 50);

        // Cleanup
        cache.delete("ttl_key").await.unwrap();
    }

    #[test]
    fn test_key_prefix() {
        let config = RedisCacheConfig::new("redis://localhost").with_key_prefix("myapp");

        // Can't actually test Redis operations without a connection,
        // but we can test the prefix logic
        assert_eq!(config.key_prefix, Some("myapp".to_string()));
    }
}
