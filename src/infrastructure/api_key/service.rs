//! API Key service
//!
//! Provides high-level operations for API key management.

use std::sync::Arc;

use tracing::{debug, info, warn};

use crate::domain::api_key::{
    ApiKey, ApiKeyId, ApiKeyPermissions, ApiKeyRepository, ApiKeyStatus, RateLimitConfig,
};
use crate::domain::DomainError;

use super::generator::ApiKeyGenerator;
use super::rate_limiter::{RateLimitResult, RateLimiter};

/// Result of creating a new API key
#[derive(Debug)]
pub struct CreateApiKeyResult {
    /// The API key entity (without the secret)
    pub api_key: ApiKey,
    /// The full secret key (only returned once)
    pub secret: String,
}

/// API Key service for managing API keys
#[derive(Debug)]
pub struct ApiKeyService<R>
where
    R: ApiKeyRepository,
{
    repository: Arc<R>,
    generator: ApiKeyGenerator,
    rate_limiter: Arc<RateLimiter>,
}

impl<R: ApiKeyRepository> ApiKeyService<R> {
    /// Create a new API key service
    pub fn new(repository: Arc<R>) -> Self {
        Self {
            repository,
            generator: ApiKeyGenerator::production(),
            rate_limiter: Arc::new(RateLimiter::new()),
        }
    }

    /// Create with a custom generator
    pub fn with_generator(mut self, generator: ApiKeyGenerator) -> Self {
        self.generator = generator;
        self
    }

    /// Create with a custom rate limiter
    pub fn with_rate_limiter(mut self, rate_limiter: Arc<RateLimiter>) -> Self {
        self.rate_limiter = rate_limiter;
        self
    }

    /// Create a new API key
    pub async fn create(
        &self,
        id: ApiKeyId,
        name: impl Into<String>,
        permissions: ApiKeyPermissions,
        rate_limits: Option<RateLimitConfig>,
    ) -> Result<CreateApiKeyResult, DomainError> {
        let name = name.into();
        info!("Creating API key: id={}, name={}", id, name);

        let generated = self.generator.generate();

        let api_key = ApiKey::new(id.clone(), &name, &generated.hash, &generated.prefix)
            .with_permissions(permissions)
            .with_rate_limits(rate_limits.unwrap_or_default());

        let created = self.repository.create(api_key).await?;

        info!("API key created: id={}", id);

        Ok(CreateApiKeyResult {
            api_key: created,
            secret: generated.key,
        })
    }

    /// Create an API key with a known secret (for testing purposes)
    ///
    /// This is useful for integration tests where a deterministic key is needed.
    pub async fn create_with_secret(
        &self,
        id: ApiKeyId,
        name: impl Into<String>,
        secret: &str,
        permissions: ApiKeyPermissions,
        rate_limits: Option<RateLimitConfig>,
    ) -> Result<CreateApiKeyResult, DomainError> {
        let name = name.into();
        info!("Creating API key with known secret: id={}, name={}", id, name);

        let generated = self.generator.from_secret(secret);

        let api_key = ApiKey::new(id.clone(), &name, &generated.hash, &generated.prefix)
            .with_permissions(permissions)
            .with_rate_limits(rate_limits.unwrap_or_default());

        let created = self.repository.create(api_key).await?;

        info!("API key created: id={}", id);

        Ok(CreateApiKeyResult {
            api_key: created,
            secret: generated.key,
        })
    }

    /// Get an API key by ID
    pub async fn get(&self, id: &ApiKeyId) -> Result<Option<ApiKey>, DomainError> {
        self.repository.get(id).await
    }

    /// Validate an API key and check permissions
    pub async fn validate(&self, key_secret: &str) -> Result<Option<ApiKey>, DomainError> {
        let prefix = ApiKeyGenerator::extract_prefix(key_secret)
            .ok_or_else(|| DomainError::validation("Invalid API key format"))?;

        debug!("Validating API key with prefix: {}", prefix);

        let api_key = self.repository.get_by_prefix(prefix).await?;

        if let Some(ref key) = api_key {
            // Verify the key hash
            if !self.generator.verify_key(key_secret, key.secret_hash()) {
                debug!("API key hash verification failed");
                return Ok(None);
            }

            // Check if key is valid
            if !key.is_valid() {
                debug!("API key is not valid: status={:?}", key.status());
                return Ok(None);
            }

            // Record usage
            if let Err(e) = self.repository.record_usage(key.id()).await {
                warn!("Failed to record API key usage: {}", e);
            }
        }

        Ok(api_key)
    }

    /// Check rate limits for an API key
    pub async fn check_rate_limit(
        &self,
        key: &ApiKey,
        tokens: Option<u32>,
    ) -> RateLimitResult {
        self.rate_limiter
            .check_and_record(key.id().as_str(), key.rate_limits(), tokens)
            .await
    }

    /// Update an API key
    pub async fn update(&self, api_key: &ApiKey) -> Result<ApiKey, DomainError> {
        info!("Updating API key: id={}", api_key.id());
        self.repository.update(api_key).await
    }

    /// Suspend an API key
    pub async fn suspend(&self, id: &ApiKeyId) -> Result<ApiKey, DomainError> {
        info!("Suspending API key: id={}", id);

        let mut key = self
            .repository
            .get(id)
            .await?
            .ok_or_else(|| DomainError::not_found(format!("API key '{}' not found", id)))?;

        key.suspend();
        self.repository.update(&key).await
    }

    /// Revoke an API key
    pub async fn revoke(&self, id: &ApiKeyId) -> Result<ApiKey, DomainError> {
        info!("Revoking API key: id={}", id);

        let mut key = self
            .repository
            .get(id)
            .await?
            .ok_or_else(|| DomainError::not_found(format!("API key '{}' not found", id)))?;

        key.revoke();

        // Also reset rate limits
        self.rate_limiter.reset(id.as_str()).await;

        self.repository.update(&key).await
    }

    /// Reactivate a suspended API key
    pub async fn activate(&self, id: &ApiKeyId) -> Result<ApiKey, DomainError> {
        info!("Activating API key: id={}", id);

        let mut key = self
            .repository
            .get(id)
            .await?
            .ok_or_else(|| DomainError::not_found(format!("API key '{}' not found", id)))?;

        if key.status() != ApiKeyStatus::Suspended {
            return Err(DomainError::validation(
                "Only suspended keys can be activated",
            ));
        }

        key.activate();
        self.repository.update(&key).await
    }

    /// Delete an API key
    pub async fn delete(&self, id: &ApiKeyId) -> Result<bool, DomainError> {
        info!("Deleting API key: id={}", id);

        // Reset rate limits
        self.rate_limiter.reset(id.as_str()).await;

        self.repository.delete(id).await
    }

    /// List all API keys
    pub async fn list(&self, status: Option<ApiKeyStatus>) -> Result<Vec<ApiKey>, DomainError> {
        self.repository.list(status).await
    }

    /// Count API keys
    pub async fn count(&self, status: Option<ApiKeyStatus>) -> Result<usize, DomainError> {
        self.repository.count(status).await
    }

    /// Update permissions for an API key
    pub async fn update_permissions(
        &self,
        id: &ApiKeyId,
        permissions: ApiKeyPermissions,
    ) -> Result<ApiKey, DomainError> {
        info!("Updating permissions for API key: id={}", id);

        let mut key = self
            .repository
            .get(id)
            .await?
            .ok_or_else(|| DomainError::not_found(format!("API key '{}' not found", id)))?;

        key.set_permissions(permissions);
        self.repository.update(&key).await
    }

    /// Update rate limits for an API key
    pub async fn update_rate_limits(
        &self,
        id: &ApiKeyId,
        rate_limits: RateLimitConfig,
    ) -> Result<ApiKey, DomainError> {
        info!("Updating rate limits for API key: id={}", id);

        let mut key = self
            .repository
            .get(id)
            .await?
            .ok_or_else(|| DomainError::not_found(format!("API key '{}' not found", id)))?;

        key.set_rate_limits(rate_limits);

        // Reset rate limit counters when limits change
        self.rate_limiter.reset(id.as_str()).await;

        self.repository.update(&key).await
    }

    /// Check if an API key has permission to access a model
    pub fn can_access_model(&self, key: &ApiKey, model_id: &str) -> bool {
        key.permissions().can_access_model(model_id)
    }

    /// Check if an API key has permission to access a knowledge base
    pub fn can_access_knowledge_base(&self, key: &ApiKey, kb_id: &str) -> bool {
        key.permissions().can_access_knowledge_base(kb_id)
    }

    /// Check if an API key has permission to access a prompt
    pub fn can_access_prompt(&self, key: &ApiKey, prompt_id: &str) -> bool {
        key.permissions().can_access_prompt(prompt_id)
    }

    /// Check if an API key has permission to access a chain
    pub fn can_access_chain(&self, key: &ApiKey, chain_id: &str) -> bool {
        key.permissions().can_access_chain(chain_id)
    }

    /// Check if an API key has admin permissions
    pub fn is_admin(&self, key: &ApiKey) -> bool {
        key.permissions().admin
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::api_key::{ResourcePermission};
    use crate::infrastructure::api_key::InMemoryApiKeyRepository;

    fn create_service() -> ApiKeyService<InMemoryApiKeyRepository> {
        let repo = Arc::new(InMemoryApiKeyRepository::new());
        ApiKeyService::new(repo).with_generator(ApiKeyGenerator::test())
    }

    #[tokio::test]
    async fn test_create_api_key() {
        let service = create_service();
        let id = ApiKeyId::new("test-key").unwrap();
        let permissions = ApiKeyPermissions::read_only();

        let result = service.create(id.clone(), "Test Key", permissions, None).await.unwrap();

        assert!(result.secret.starts_with("pk_test_"));
        assert_eq!(result.api_key.name(), "Test Key");
        assert!(result.api_key.is_valid());
    }

    #[tokio::test]
    async fn test_validate_api_key() {
        let service = create_service();
        let id = ApiKeyId::new("test-key").unwrap();

        let created = service
            .create(id, "Test Key", ApiKeyPermissions::read_only(), None)
            .await
            .unwrap();

        let validated = service.validate(&created.secret).await.unwrap();
        assert!(validated.is_some());
        assert_eq!(validated.unwrap().name(), "Test Key");
    }

    #[tokio::test]
    async fn test_validate_invalid_key() {
        let service = create_service();

        let validated = service.validate("pk_test_invalid_key").await.unwrap();
        assert!(validated.is_none());
    }

    #[tokio::test]
    async fn test_suspend_and_activate() {
        let service = create_service();
        let id = ApiKeyId::new("test-key").unwrap();

        let created = service
            .create(id.clone(), "Test Key", ApiKeyPermissions::read_only(), None)
            .await
            .unwrap();

        // Suspend
        let suspended = service.suspend(&id).await.unwrap();
        assert_eq!(suspended.status(), ApiKeyStatus::Suspended);

        // Validate should fail
        let validated = service.validate(&created.secret).await.unwrap();
        assert!(validated.is_none());

        // Activate
        let activated = service.activate(&id).await.unwrap();
        assert_eq!(activated.status(), ApiKeyStatus::Active);

        // Validate should succeed again
        let validated = service.validate(&created.secret).await.unwrap();
        assert!(validated.is_some());
    }

    #[tokio::test]
    async fn test_revoke() {
        let service = create_service();
        let id = ApiKeyId::new("test-key").unwrap();

        let created = service
            .create(id.clone(), "Test Key", ApiKeyPermissions::read_only(), None)
            .await
            .unwrap();

        let revoked = service.revoke(&id).await.unwrap();
        assert_eq!(revoked.status(), ApiKeyStatus::Revoked);

        // Validate should fail
        let validated = service.validate(&created.secret).await.unwrap();
        assert!(validated.is_none());
    }

    #[tokio::test]
    async fn test_rate_limiting() {
        let service = create_service();
        let id = ApiKeyId::new("test-key").unwrap();
        let rate_limits = RateLimitConfig::new(2, 100, 1000);

        let created = service
            .create(id, "Test Key", ApiKeyPermissions::read_only(), Some(rate_limits))
            .await
            .unwrap();

        // First two requests should succeed
        let result1 = service.check_rate_limit(&created.api_key, None).await;
        assert!(result1.allowed);

        let result2 = service.check_rate_limit(&created.api_key, None).await;
        assert!(result2.allowed);

        // Third should be rate limited
        let result3 = service.check_rate_limit(&created.api_key, None).await;
        assert!(!result3.allowed);
    }

    #[tokio::test]
    async fn test_permission_checking() {
        let service = create_service();
        let id = ApiKeyId::new("test-key").unwrap();

        let permissions = ApiKeyPermissions::new()
            .with_models(ResourcePermission::specific(vec!["gpt-4", "gpt-3.5-turbo"]))
            .with_knowledge_bases(ResourcePermission::all());

        let created = service
            .create(id, "Test Key", permissions, None)
            .await
            .unwrap();

        assert!(service.can_access_model(&created.api_key, "gpt-4"));
        assert!(service.can_access_model(&created.api_key, "gpt-3.5-turbo"));
        assert!(!service.can_access_model(&created.api_key, "claude-3"));

        assert!(service.can_access_knowledge_base(&created.api_key, "any-kb"));
    }

    #[tokio::test]
    async fn test_update_permissions() {
        let service = create_service();
        let id = ApiKeyId::new("test-key").unwrap();

        service
            .create(id.clone(), "Test Key", ApiKeyPermissions::new(), None)
            .await
            .unwrap();

        let new_permissions = ApiKeyPermissions::full_access();
        let updated = service.update_permissions(&id, new_permissions).await.unwrap();

        assert!(updated.permissions().admin);
    }

    #[tokio::test]
    async fn test_list_and_count() {
        let service = create_service();

        service
            .create(ApiKeyId::new("key-1").unwrap(), "Key 1", ApiKeyPermissions::new(), None)
            .await
            .unwrap();
        service
            .create(ApiKeyId::new("key-2").unwrap(), "Key 2", ApiKeyPermissions::new(), None)
            .await
            .unwrap();

        let all = service.list(None).await.unwrap();
        assert_eq!(all.len(), 2);

        let count = service.count(None).await.unwrap();
        assert_eq!(count, 2);
    }

    #[tokio::test]
    async fn test_delete() {
        let service = create_service();
        let id = ApiKeyId::new("test-key").unwrap();

        service
            .create(id.clone(), "Test Key", ApiKeyPermissions::new(), None)
            .await
            .unwrap();

        let deleted = service.delete(&id).await.unwrap();
        assert!(deleted);

        let found = service.get(&id).await.unwrap();
        assert!(found.is_none());
    }
}
