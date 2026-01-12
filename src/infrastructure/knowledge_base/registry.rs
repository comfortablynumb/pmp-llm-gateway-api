//! Knowledge base provider registry - manages provider instances

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;

use crate::domain::knowledge_base::KnowledgeBaseProvider;
use crate::domain::DomainError;

#[cfg(test)]
use crate::domain::knowledge_base::{KnowledgeBaseId, MockKnowledgeBaseProvider};

/// Registry for managing knowledge base provider instances
#[derive(Debug)]
pub struct KnowledgeBaseProviderRegistry {
    /// Cached provider instances by KB ID
    providers: RwLock<HashMap<String, Arc<dyn KnowledgeBaseProvider>>>,
}

impl Default for KnowledgeBaseProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl KnowledgeBaseProviderRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            providers: RwLock::new(HashMap::new()),
        }
    }

    /// Register a provider for a knowledge base
    pub async fn register(&self, provider: Arc<dyn KnowledgeBaseProvider>) {
        let kb_id = provider.knowledge_base_id().as_str().to_string();
        self.providers.write().await.insert(kb_id, provider);
    }

    /// Get a provider by knowledge base ID
    pub async fn get(&self, kb_id: &str) -> Option<Arc<dyn KnowledgeBaseProvider>> {
        self.providers.read().await.get(kb_id).cloned()
    }

    /// Get a provider by knowledge base ID, returning error if not found
    pub async fn get_required(&self, kb_id: &str) -> Result<Arc<dyn KnowledgeBaseProvider>, DomainError> {
        self.get(kb_id).await.ok_or_else(|| {
            DomainError::not_found(format!(
                "No provider registered for knowledge base '{}'",
                kb_id
            ))
        })
    }

    /// Check if a provider is registered for a knowledge base
    pub async fn has_provider(&self, kb_id: &str) -> bool {
        self.providers.read().await.contains_key(kb_id)
    }

    /// Remove a provider from the registry
    pub async fn unregister(&self, kb_id: &str) -> Option<Arc<dyn KnowledgeBaseProvider>> {
        self.providers.write().await.remove(kb_id)
    }

    /// List all registered knowledge base IDs
    pub async fn list_kb_ids(&self) -> Vec<String> {
        self.providers.read().await.keys().cloned().collect()
    }

    /// Get provider count
    pub async fn count(&self) -> usize {
        self.providers.read().await.len()
    }
}

/// Trait for the provider registry (for dependency injection)
#[async_trait::async_trait]
pub trait KnowledgeBaseProviderRegistryTrait: Send + Sync + std::fmt::Debug {
    /// Get a provider by knowledge base ID
    async fn get(&self, kb_id: &str) -> Option<Arc<dyn KnowledgeBaseProvider>>;

    /// Get a provider by knowledge base ID, returning error if not found
    async fn get_required(&self, kb_id: &str) -> Result<Arc<dyn KnowledgeBaseProvider>, DomainError>;

    /// Check if a provider is registered for a knowledge base
    async fn has_provider(&self, kb_id: &str) -> bool;

    /// Register a provider for a knowledge base
    async fn register(&self, provider: Arc<dyn KnowledgeBaseProvider>);
}

#[async_trait::async_trait]
impl KnowledgeBaseProviderRegistryTrait for KnowledgeBaseProviderRegistry {
    async fn get(&self, kb_id: &str) -> Option<Arc<dyn KnowledgeBaseProvider>> {
        KnowledgeBaseProviderRegistry::get(self, kb_id).await
    }

    async fn get_required(&self, kb_id: &str) -> Result<Arc<dyn KnowledgeBaseProvider>, DomainError> {
        KnowledgeBaseProviderRegistry::get_required(self, kb_id).await
    }

    async fn has_provider(&self, kb_id: &str) -> bool {
        KnowledgeBaseProviderRegistry::has_provider(self, kb_id).await
    }

    async fn register(&self, provider: Arc<dyn KnowledgeBaseProvider>) {
        KnowledgeBaseProviderRegistry::register(self, provider).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_register_and_get() {
        let registry = KnowledgeBaseProviderRegistry::new();
        let kb_id = KnowledgeBaseId::new("test-kb").unwrap();
        let provider: Arc<dyn KnowledgeBaseProvider> =
            Arc::new(MockKnowledgeBaseProvider::new(kb_id));

        registry.register(provider).await;

        let retrieved = registry.get("test-kb").await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().knowledge_base_id().as_str(), "test-kb");
    }

    #[tokio::test]
    async fn test_get_required_not_found() {
        let registry = KnowledgeBaseProviderRegistry::new();

        let result = registry.get_required("not-exists").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_has_provider() {
        let registry = KnowledgeBaseProviderRegistry::new();
        let kb_id = KnowledgeBaseId::new("test-kb").unwrap();
        let provider: Arc<dyn KnowledgeBaseProvider> =
            Arc::new(MockKnowledgeBaseProvider::new(kb_id));

        assert!(!registry.has_provider("test-kb").await);

        registry.register(provider).await;

        assert!(registry.has_provider("test-kb").await);
    }

    #[tokio::test]
    async fn test_unregister() {
        let registry = KnowledgeBaseProviderRegistry::new();
        let kb_id = KnowledgeBaseId::new("test-kb").unwrap();
        let provider: Arc<dyn KnowledgeBaseProvider> =
            Arc::new(MockKnowledgeBaseProvider::new(kb_id));

        registry.register(provider).await;
        assert!(registry.has_provider("test-kb").await);

        registry.unregister("test-kb").await;
        assert!(!registry.has_provider("test-kb").await);
    }
}
