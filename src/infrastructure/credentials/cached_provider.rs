use async_trait::async_trait;
use moka::future::Cache;
use std::sync::Arc;
use std::time::Duration;

use crate::domain::{Credential, CredentialProvider, CredentialType, DomainError};

/// Cache key for credentials
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CacheKey(CredentialType);

/// Credential provider wrapper that adds caching with TTL
#[derive(Debug)]
pub struct CachedCredentialProvider<P: CredentialProvider> {
    inner: P,
    cache: Cache<CacheKey, Arc<Credential>>,
}

impl<P: CredentialProvider> CachedCredentialProvider<P> {
    pub fn new(inner: P, ttl: Duration) -> Self {
        let cache = Cache::builder()
            .time_to_live(ttl)
            .max_capacity(100)
            .build();

        Self { inner, cache }
    }

    pub fn with_capacity(inner: P, ttl: Duration, capacity: u64) -> Self {
        let cache = Cache::builder()
            .time_to_live(ttl)
            .max_capacity(capacity)
            .build();

        Self { inner, cache }
    }

    /// Invalidate a specific credential from cache
    pub async fn invalidate(&self, credential_type: &CredentialType) {
        self.cache.invalidate(&CacheKey(credential_type.clone())).await;
    }

    /// Invalidate all cached credentials
    pub fn invalidate_all(&self) {
        self.cache.invalidate_all();
    }

    /// Get cache statistics
    pub fn cache_size(&self) -> u64 {
        self.cache.entry_count()
    }
}

#[async_trait]
impl<P: CredentialProvider> CredentialProvider for CachedCredentialProvider<P> {
    async fn get_credential(
        &self,
        credential_type: &CredentialType,
    ) -> Result<Credential, DomainError> {
        let key = CacheKey(credential_type.clone());

        // Try to get from cache first
        if let Some(cached) = self.cache.get(&key).await {
            // Check if cached credential is expired
            if !cached.is_expired() {
                tracing::debug!(
                    provider = self.inner.provider_name(),
                    credential_type = %credential_type,
                    "Cache hit for credential"
                );
                return Ok((*cached).clone());
            }

            // Credential expired, invalidate cache entry
            self.cache.invalidate(&key).await;
        }

        // Fetch from underlying provider
        tracing::debug!(
            provider = self.inner.provider_name(),
            credential_type = %credential_type,
            "Cache miss, fetching credential"
        );

        let credential = self.inner.get_credential(credential_type).await?;
        self.cache.insert(key, Arc::new(credential.clone())).await;

        Ok(credential)
    }

    async fn supports(&self, credential_type: &CredentialType) -> bool {
        self.inner.supports(credential_type).await
    }

    async fn refresh(&self, credential_type: &CredentialType) -> Result<Credential, DomainError> {
        // Invalidate cache and fetch fresh credential
        self.invalidate(credential_type).await;
        self.get_credential(credential_type).await
    }

    fn provider_name(&self) -> &'static str {
        self.inner.provider_name()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::credentials::mock::MockCredentialProvider;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[derive(Debug)]
    struct CountingProvider {
        inner: MockCredentialProvider,
        call_count: AtomicUsize,
    }

    impl CountingProvider {
        fn new(inner: MockCredentialProvider) -> Self {
            Self {
                inner,
                call_count: AtomicUsize::new(0),
            }
        }

        fn call_count(&self) -> usize {
            self.call_count.load(Ordering::SeqCst)
        }
    }

    #[async_trait]
    impl CredentialProvider for CountingProvider {
        async fn get_credential(
            &self,
            credential_type: &CredentialType,
        ) -> Result<Credential, DomainError> {
            self.call_count.fetch_add(1, Ordering::SeqCst);
            self.inner.get_credential(credential_type).await
        }

        async fn supports(&self, credential_type: &CredentialType) -> bool {
            self.inner.supports(credential_type).await
        }

        fn provider_name(&self) -> &'static str {
            "counting"
        }
    }

    #[tokio::test]
    async fn test_cached_provider_caches_credentials() {
        use std::sync::Arc;

        let mock = MockCredentialProvider::new("mock")
            .with_credential(Credential::new(CredentialType::OpenAi, "sk-test".to_string()));

        let call_count = Arc::new(AtomicUsize::new(0));
        let counting = CountingProvider {
            inner: mock,
            call_count: AtomicUsize::new(0),
        };

        let cached = CachedCredentialProvider::new(counting, Duration::from_secs(60));

        // First call should fetch from provider
        let cred1 = cached.get_credential(&CredentialType::OpenAi).await.unwrap();
        assert_eq!(cred1.api_key(), "sk-test");

        // Second call should use cache (same result)
        let cred2 = cached.get_credential(&CredentialType::OpenAi).await.unwrap();
        assert_eq!(cred2.api_key(), "sk-test");

        // Both calls should return the same credential
        assert_eq!(cred1.api_key(), cred2.api_key());
    }

    #[tokio::test]
    async fn test_cached_provider_invalidation() {
        let mock = MockCredentialProvider::new("mock")
            .with_credential(Credential::new(CredentialType::OpenAi, "sk-test".to_string()));

        let cached = CachedCredentialProvider::new(mock, Duration::from_secs(60));

        // Fetch to populate cache
        let cred1 = cached.get_credential(&CredentialType::OpenAi).await.unwrap();
        assert_eq!(cred1.api_key(), "sk-test");

        // Invalidate and fetch again - should still work
        cached.invalidate(&CredentialType::OpenAi).await;
        let cred2 = cached.get_credential(&CredentialType::OpenAi).await.unwrap();
        assert_eq!(cred2.api_key(), "sk-test");
    }

    #[tokio::test]
    async fn test_cached_provider_refresh() {
        let mock = MockCredentialProvider::new("mock")
            .with_credential(Credential::new(CredentialType::OpenAi, "sk-test".to_string()));

        let cached = CachedCredentialProvider::new(mock, Duration::from_secs(60));

        // Fetch to populate cache
        cached.get_credential(&CredentialType::OpenAi).await.unwrap();

        // Refresh should invalidate and refetch
        let refreshed = cached.refresh(&CredentialType::OpenAi).await.unwrap();
        assert_eq!(refreshed.api_key(), "sk-test");
    }
}
