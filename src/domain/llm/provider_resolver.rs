//! Provider resolver trait for resolving model IDs to LLM providers

use async_trait::async_trait;
use std::fmt::Debug;
use std::sync::Arc;

use super::LlmProvider;
use crate::domain::DomainError;

/// Trait for resolving model IDs to LLM provider instances.
///
/// This abstraction allows the workflow executor to dynamically
/// select the appropriate provider based on model configuration,
/// rather than using a single hardcoded provider.
#[async_trait]
pub trait ProviderResolver: Send + Sync + Debug {
    /// Resolve a model ID to an LLM provider instance.
    ///
    /// # Arguments
    /// * `model_id` - The model identifier to resolve
    ///
    /// # Returns
    /// * `Ok(Arc<dyn LlmProvider>)` - The provider instance for the model
    /// * `Err(DomainError)` - If the model or provider cannot be resolved
    async fn resolve(&self, model_id: &str) -> Result<Arc<dyn LlmProvider>, DomainError>;
}

/// A simple provider resolver that always returns the same provider.
///
/// Useful for testing or when all models use the same provider.
#[derive(Debug)]
pub struct StaticProviderResolver {
    provider: Arc<dyn LlmProvider>,
}

impl StaticProviderResolver {
    /// Create a new static provider resolver.
    pub fn new(provider: Arc<dyn LlmProvider>) -> Self {
        Self { provider }
    }
}

#[async_trait]
impl ProviderResolver for StaticProviderResolver {
    async fn resolve(&self, _model_id: &str) -> Result<Arc<dyn LlmProvider>, DomainError> {
        Ok(self.provider.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::llm::MockLlmProvider;

    #[tokio::test]
    async fn test_static_provider_resolver() {
        let mock_provider = Arc::new(MockLlmProvider::new("mock"));
        let resolver = StaticProviderResolver::new(mock_provider.clone());

        let resolved = resolver.resolve("any-model").await.unwrap();
        assert_eq!(resolved.provider_name(), mock_provider.provider_name());
    }

    #[tokio::test]
    async fn test_static_resolver_returns_same_provider_for_different_models() {
        let mock_provider = Arc::new(MockLlmProvider::new("mock"));
        let resolver = StaticProviderResolver::new(mock_provider);

        let provider1 = resolver.resolve("model-a").await.unwrap();
        let provider2 = resolver.resolve("model-b").await.unwrap();

        assert_eq!(provider1.provider_name(), provider2.provider_name());
    }
}
