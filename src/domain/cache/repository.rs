//! Cache trait definition

use std::fmt::Debug;
use std::time::Duration;

use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};

use crate::domain::DomainError;

/// Cache entry metadata
#[derive(Debug, Clone)]
pub struct CacheEntryMeta {
    /// Time-to-live remaining (if known)
    pub ttl_remaining: Option<Duration>,
    /// Size in bytes (if known)
    pub size_bytes: Option<usize>,
}

/// Generic cache trait for key-value operations with TTL support
///
/// This trait uses JSON strings internally to be dyn-compatible.
/// Use the helper methods for typed get/set operations.
#[async_trait]
pub trait Cache: Send + Sync + Debug {
    /// Gets a raw JSON value from the cache
    async fn get_raw(&self, key: &str) -> Result<Option<String>, DomainError>;

    /// Sets a raw JSON value in the cache with a TTL
    async fn set_raw(&self, key: &str, value: &str, ttl: Duration) -> Result<(), DomainError>;

    /// Sets a value only if the key doesn't exist
    async fn set_nx_raw(
        &self,
        key: &str,
        value: &str,
        ttl: Duration,
    ) -> Result<bool, DomainError> {
        if self.exists(key).await? {
            Ok(false)
        } else {
            self.set_raw(key, value, ttl).await?;
            Ok(true)
        }
    }

    /// Deletes a value from the cache
    async fn delete(&self, key: &str) -> Result<bool, DomainError>;

    /// Deletes multiple keys matching a pattern
    async fn delete_pattern(&self, pattern: &str) -> Result<usize, DomainError>;

    /// Checks if a key exists in the cache
    async fn exists(&self, key: &str) -> Result<bool, DomainError> {
        Ok(self.get_raw(key).await?.is_some())
    }

    /// Updates the TTL for an existing key
    async fn expire(&self, key: &str, ttl: Duration) -> Result<bool, DomainError>;

    /// Gets the remaining TTL for a key
    async fn ttl(&self, key: &str) -> Result<Option<Duration>, DomainError>;

    /// Clears all entries from the cache
    async fn clear(&self) -> Result<(), DomainError>;

    /// Returns approximate number of entries in the cache
    async fn size(&self) -> Result<usize, DomainError>;

    /// Increments a numeric value, returning the new value
    async fn increment(&self, key: &str, delta: i64) -> Result<i64, DomainError>;
}

/// Extension trait providing typed get/set operations
pub trait CacheExt: Cache {
    /// Gets a typed value from the cache
    fn get<'a, V>(
        &'a self,
        key: &'a str,
    ) -> impl std::future::Future<Output = Result<Option<V>, DomainError>> + Send
    where
        V: DeserializeOwned + Send,
    {
        async move {
            match self.get_raw(key).await? {
                Some(data) => {
                    let value: V = serde_json::from_str(&data).map_err(|e| {
                        DomainError::cache(format!("Failed to deserialize cache value: {}", e))
                    })?;
                    Ok(Some(value))
                }
                None => Ok(None),
            }
        }
    }

    /// Sets a typed value in the cache with a TTL
    fn set<'a, V>(
        &'a self,
        key: &'a str,
        value: &'a V,
        ttl: Duration,
    ) -> impl std::future::Future<Output = Result<(), DomainError>> + Send
    where
        V: Serialize + Send + Sync,
    {
        async move {
            let data = serde_json::to_string(value).map_err(|e| {
                DomainError::cache(format!("Failed to serialize cache value: {}", e))
            })?;
            self.set_raw(key, &data, ttl).await
        }
    }

    /// Sets a value only if the key doesn't exist
    fn set_nx<'a, V>(
        &'a self,
        key: &'a str,
        value: &'a V,
        ttl: Duration,
    ) -> impl std::future::Future<Output = Result<bool, DomainError>> + Send
    where
        V: Serialize + Send + Sync,
    {
        async move {
            let data = serde_json::to_string(value).map_err(|e| {
                DomainError::cache(format!("Failed to serialize cache value: {}", e))
            })?;
            self.set_nx_raw(key, &data, ttl).await
        }
    }

    /// Gets multiple values at once
    fn get_many<'a, V>(
        &'a self,
        keys: &'a [&'a str],
    ) -> impl std::future::Future<Output = Result<Vec<Option<V>>, DomainError>> + Send
    where
        V: DeserializeOwned + Send,
    {
        async move {
            let mut results = Vec::with_capacity(keys.len());

            for key in keys {
                results.push(self.get(key).await?);
            }

            Ok(results)
        }
    }

    /// Sets multiple values at once
    fn set_many<'a, V>(
        &'a self,
        entries: &'a [(&'a str, &'a V)],
        ttl: Duration,
    ) -> impl std::future::Future<Output = Result<(), DomainError>> + Send
    where
        V: Serialize + Send + Sync,
    {
        async move {
            for (key, value) in entries {
                self.set(key, value, ttl).await?;
            }

            Ok(())
        }
    }
}

// Blanket implementation for all types implementing Cache
impl<T: Cache + ?Sized> CacheExt for T {}

#[cfg(test)]
pub mod mock {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    /// Mock cache for testing
    #[derive(Debug)]
    pub struct MockCache {
        entries: Mutex<HashMap<String, (String, Option<Duration>)>>,
        error: Mutex<Option<String>>,
    }

    impl Default for MockCache {
        fn default() -> Self {
            Self::new()
        }
    }

    impl MockCache {
        pub fn new() -> Self {
            Self {
                entries: Mutex::new(HashMap::new()),
                error: Mutex::new(None),
            }
        }

        pub fn with_entry<V: Serialize>(self, key: &str, value: &V, ttl: Option<Duration>) -> Self {
            let json = serde_json::to_string(value).unwrap();
            self.entries
                .lock()
                .unwrap()
                .insert(key.to_string(), (json, ttl));
            self
        }

        pub fn with_error(self, error: impl Into<String>) -> Self {
            *self.error.lock().unwrap() = Some(error.into());
            self
        }

        fn check_error(&self) -> Result<(), DomainError> {
            if let Some(error) = self.error.lock().unwrap().clone() {
                return Err(DomainError::cache(error));
            }
            Ok(())
        }
    }

    #[async_trait]
    impl Cache for MockCache {
        async fn get_raw(&self, key: &str) -> Result<Option<String>, DomainError> {
            self.check_error()?;
            let entries = self.entries.lock().unwrap();

            Ok(entries.get(key).map(|(json, _)| json.clone()))
        }

        async fn set_raw(&self, key: &str, value: &str, ttl: Duration) -> Result<(), DomainError> {
            self.check_error()?;
            self.entries
                .lock()
                .unwrap()
                .insert(key.to_string(), (value.to_string(), Some(ttl)));
            Ok(())
        }

        async fn delete(&self, key: &str) -> Result<bool, DomainError> {
            self.check_error()?;
            Ok(self.entries.lock().unwrap().remove(key).is_some())
        }

        async fn delete_pattern(&self, pattern: &str) -> Result<usize, DomainError> {
            self.check_error()?;

            let pattern_regex = pattern.replace('*', ".*");
            let regex = regex::Regex::new(&pattern_regex)
                .map_err(|e| DomainError::cache(format!("Invalid pattern: {}", e)))?;

            let mut entries = self.entries.lock().unwrap();
            let keys_to_remove: Vec<String> = entries
                .keys()
                .filter(|k| regex.is_match(k))
                .cloned()
                .collect();

            let count = keys_to_remove.len();

            for key in keys_to_remove {
                entries.remove(&key);
            }

            Ok(count)
        }

        async fn expire(&self, key: &str, ttl: Duration) -> Result<bool, DomainError> {
            self.check_error()?;
            let mut entries = self.entries.lock().unwrap();

            if let Some((json, _)) = entries.get(key) {
                let json = json.clone();
                entries.insert(key.to_string(), (json, Some(ttl)));
                Ok(true)
            } else {
                Ok(false)
            }
        }

        async fn ttl(&self, key: &str) -> Result<Option<Duration>, DomainError> {
            self.check_error()?;
            let entries = self.entries.lock().unwrap();

            Ok(entries.get(key).and_then(|(_, ttl)| *ttl))
        }

        async fn clear(&self) -> Result<(), DomainError> {
            self.check_error()?;
            self.entries.lock().unwrap().clear();
            Ok(())
        }

        async fn size(&self) -> Result<usize, DomainError> {
            self.check_error()?;
            Ok(self.entries.lock().unwrap().len())
        }

        async fn increment(&self, key: &str, delta: i64) -> Result<i64, DomainError> {
            self.check_error()?;
            let mut entries = self.entries.lock().unwrap();

            let current: i64 = entries
                .get(key)
                .map(|(json, _)| serde_json::from_str(json).unwrap_or(0))
                .unwrap_or(0);

            let new_value = current + delta;
            let json = serde_json::to_string(&new_value).unwrap();
            entries.insert(key.to_string(), (json, None));

            Ok(new_value)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[tokio::test]
        async fn test_mock_cache_set_get() {
            let cache = MockCache::new();
            cache
                .set("key1", &"value1", Duration::from_secs(60))
                .await
                .unwrap();

            let result: Option<String> = cache.get("key1").await.unwrap();
            assert_eq!(result, Some("value1".to_string()));
        }

        #[tokio::test]
        async fn test_mock_cache_get_missing() {
            let cache = MockCache::new();

            let result: Option<String> = cache.get("missing").await.unwrap();
            assert!(result.is_none());
        }

        #[tokio::test]
        async fn test_mock_cache_delete() {
            let cache = MockCache::new();
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
        async fn test_mock_cache_with_error() {
            let cache = MockCache::new().with_error("Test error");

            let result: Result<Option<String>, _> = cache.get("key").await;
            assert!(result.is_err());
        }

        #[tokio::test]
        async fn test_mock_cache_increment() {
            let cache = MockCache::new();

            let val = cache.increment("counter", 5).await.unwrap();
            assert_eq!(val, 5);

            let val = cache.increment("counter", 3).await.unwrap();
            assert_eq!(val, 8);

            let val = cache.increment("counter", -2).await.unwrap();
            assert_eq!(val, 6);
        }

        #[tokio::test]
        async fn test_mock_cache_clear() {
            let cache = MockCache::new();
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
        async fn test_mock_cache_delete_pattern() {
            let cache = MockCache::new();
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

            let deleted = cache.delete_pattern("user:*:profile").await.unwrap();
            assert_eq!(deleted, 2);

            let size = cache.size().await.unwrap();
            assert_eq!(size, 1);
        }
    }
}
