//! Cache factory for runtime selection

use std::sync::Arc;
use std::time::Duration;

use crate::domain::cache::Cache;
use crate::domain::DomainError;

use super::in_memory::{InMemoryCache, InMemoryCacheConfig};
use super::redis::{RedisCache, RedisCacheConfig};

/// Supported cache types
#[derive(Debug, Clone, PartialEq)]
pub enum CacheType {
    /// In-memory cache using moka
    InMemory,
    /// Redis cache
    Redis,
}

impl Default for CacheType {
    fn default() -> Self {
        Self::InMemory
    }
}

impl std::fmt::Display for CacheType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CacheType::InMemory => write!(f, "in_memory"),
            CacheType::Redis => write!(f, "redis"),
        }
    }
}

impl std::str::FromStr for CacheType {
    type Err = DomainError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "in_memory" | "inmemory" | "memory" => Ok(CacheType::InMemory),
            "redis" => Ok(CacheType::Redis),
            _ => Err(DomainError::configuration(format!(
                "Unknown cache type: {}. Valid types: in_memory, redis",
                s
            ))),
        }
    }
}

/// Configuration for cache factory
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Type of cache to create
    pub cache_type: CacheType,
    /// Redis URL (required for Redis type)
    pub redis_url: Option<String>,
    /// Key prefix for namespacing
    pub key_prefix: Option<String>,
    /// Default TTL for entries
    pub default_ttl: Duration,
    /// Maximum capacity (for in-memory cache)
    pub max_capacity: Option<u64>,
    /// Time to idle (for in-memory cache)
    pub time_to_idle: Option<Duration>,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            cache_type: CacheType::InMemory,
            redis_url: None,
            key_prefix: None,
            default_ttl: Duration::from_secs(3600),
            max_capacity: Some(10_000),
            time_to_idle: None,
        }
    }
}

impl CacheConfig {
    /// Creates a new configuration for in-memory cache
    pub fn in_memory() -> Self {
        Self {
            cache_type: CacheType::InMemory,
            ..Default::default()
        }
    }

    /// Creates a new configuration for Redis cache
    pub fn redis(url: impl Into<String>) -> Self {
        Self {
            cache_type: CacheType::Redis,
            redis_url: Some(url.into()),
            ..Default::default()
        }
    }

    /// Sets the key prefix
    pub fn with_key_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.key_prefix = Some(prefix.into());
        self
    }

    /// Sets the default TTL
    pub fn with_default_ttl(mut self, ttl: Duration) -> Self {
        self.default_ttl = ttl;
        self
    }

    /// Sets the maximum capacity (in-memory only)
    pub fn with_max_capacity(mut self, capacity: u64) -> Self {
        self.max_capacity = Some(capacity);
        self
    }

    /// Sets the time-to-idle (in-memory only)
    pub fn with_time_to_idle(mut self, tti: Duration) -> Self {
        self.time_to_idle = Some(tti);
        self
    }

    /// Creates config from environment variables
    pub fn from_env() -> Result<Self, DomainError> {
        let cache_type = std::env::var("CACHE_TYPE")
            .unwrap_or_else(|_| "in_memory".to_string())
            .parse()?;

        let redis_url = std::env::var("REDIS_URL").ok();
        let key_prefix = std::env::var("CACHE_KEY_PREFIX").ok();

        let default_ttl = std::env::var("CACHE_DEFAULT_TTL_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .map(Duration::from_secs)
            .unwrap_or(Duration::from_secs(3600));

        let max_capacity = std::env::var("CACHE_MAX_CAPACITY")
            .ok()
            .and_then(|v| v.parse().ok());

        Ok(Self {
            cache_type,
            redis_url,
            key_prefix,
            default_ttl,
            max_capacity,
            time_to_idle: None,
        })
    }
}

/// Factory for creating cache instances
#[derive(Debug, Default)]
pub struct CacheFactory;

impl CacheFactory {
    /// Creates a new cache factory
    pub fn new() -> Self {
        Self
    }

    /// Creates a cache instance based on configuration
    pub async fn create(&self, config: &CacheConfig) -> Result<Arc<dyn Cache>, DomainError> {
        match config.cache_type {
            CacheType::InMemory => {
                let mut in_memory_config = InMemoryCacheConfig::default()
                    .with_default_ttl(config.default_ttl);

                if let Some(capacity) = config.max_capacity {
                    in_memory_config = in_memory_config.with_max_capacity(capacity);
                }

                if let Some(tti) = config.time_to_idle {
                    in_memory_config = in_memory_config.with_time_to_idle(tti);
                }

                let cache = InMemoryCache::with_config(in_memory_config);
                Ok(Arc::new(cache))
            }
            CacheType::Redis => {
                let url = config.redis_url.clone().ok_or_else(|| {
                    DomainError::configuration("Redis URL is required for Redis cache type")
                })?;

                let mut redis_config =
                    RedisCacheConfig::new(url).with_default_ttl(config.default_ttl);

                if let Some(prefix) = &config.key_prefix {
                    redis_config = redis_config.with_key_prefix(prefix.clone());
                }

                let cache = RedisCache::new(redis_config).await?;
                Ok(Arc::new(cache))
            }
        }
    }

    /// Creates an in-memory cache with default settings
    pub fn create_in_memory(&self) -> Arc<dyn Cache> {
        Arc::new(InMemoryCache::new())
    }

    /// Creates an in-memory cache with custom configuration
    pub fn create_in_memory_with_config(&self, config: InMemoryCacheConfig) -> Arc<dyn Cache> {
        Arc::new(InMemoryCache::with_config(config))
    }

    /// Creates a Redis cache
    pub async fn create_redis(&self, url: impl Into<String>) -> Result<Arc<dyn Cache>, DomainError> {
        let config = RedisCacheConfig::new(url);
        let cache = RedisCache::new(config).await?;
        Ok(Arc::new(cache))
    }

    /// Creates a Redis cache with custom configuration
    pub async fn create_redis_with_config(
        &self,
        config: RedisCacheConfig,
    ) -> Result<Arc<dyn Cache>, DomainError> {
        let cache = RedisCache::new(config).await?;
        Ok(Arc::new(cache))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::cache::CacheExt;

    #[test]
    fn test_cache_type_from_str() {
        assert_eq!("in_memory".parse::<CacheType>().unwrap(), CacheType::InMemory);
        assert_eq!("inmemory".parse::<CacheType>().unwrap(), CacheType::InMemory);
        assert_eq!("memory".parse::<CacheType>().unwrap(), CacheType::InMemory);
        assert_eq!("redis".parse::<CacheType>().unwrap(), CacheType::Redis);
        assert_eq!("REDIS".parse::<CacheType>().unwrap(), CacheType::Redis);
    }

    #[test]
    fn test_cache_type_from_str_invalid() {
        let result = "invalid".parse::<CacheType>();
        assert!(result.is_err());
    }

    #[test]
    fn test_cache_config_in_memory() {
        let config = CacheConfig::in_memory()
            .with_max_capacity(1000)
            .with_default_ttl(Duration::from_secs(300));

        assert_eq!(config.cache_type, CacheType::InMemory);
        assert_eq!(config.max_capacity, Some(1000));
        assert_eq!(config.default_ttl, Duration::from_secs(300));
    }

    #[test]
    fn test_cache_config_redis() {
        let config = CacheConfig::redis("redis://localhost:6379")
            .with_key_prefix("myapp")
            .with_default_ttl(Duration::from_secs(600));

        assert_eq!(config.cache_type, CacheType::Redis);
        assert_eq!(config.redis_url, Some("redis://localhost:6379".to_string()));
        assert_eq!(config.key_prefix, Some("myapp".to_string()));
    }

    #[tokio::test]
    async fn test_factory_create_in_memory() {
        let factory = CacheFactory::new();
        let config = CacheConfig::in_memory();

        let cache = factory.create(&config).await.unwrap();

        // Test basic operations
        cache
            .set("test", &"value", Duration::from_secs(60))
            .await
            .unwrap();

        let result: Option<String> = cache.get("test").await.unwrap();
        assert_eq!(result, Some("value".to_string()));
    }

    #[tokio::test]
    async fn test_factory_create_redis_missing_url() {
        let factory = CacheFactory::new();
        let config = CacheConfig {
            cache_type: CacheType::Redis,
            redis_url: None,
            ..Default::default()
        };

        let result = factory.create(&config).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_cache_type_display() {
        assert_eq!(CacheType::InMemory.to_string(), "in_memory");
        assert_eq!(CacheType::Redis.to_string(), "redis");
    }
}
