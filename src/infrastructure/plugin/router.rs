//! Provider Router
//!
//! Routes requests to the appropriate LLM provider based on model configuration.

use crate::domain::credentials::{Credential, CredentialType};
use crate::domain::llm::LlmProvider;
use crate::domain::plugin::{LlmProviderConfig, LlmProviderPlugin, PluginError};
use crate::domain::Model;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

/// Cache key for provider instances
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ProviderCacheKey {
    credential_type: String,
    credential_id: String,
}

impl ProviderCacheKey {
    fn new(credential_type: &CredentialType, credential_id: &str) -> Self {
        Self {
            credential_type: format!("{:?}", credential_type),
            credential_id: credential_id.to_string(),
        }
    }
}

/// Routes model requests to the appropriate LLM provider
#[derive(Debug)]
pub struct ProviderRouter {
    /// LLM provider plugins indexed by credential type
    llm_plugins: RwLock<HashMap<String, Arc<dyn LlmProviderPlugin>>>,

    /// Cached provider instances
    provider_cache: RwLock<HashMap<ProviderCacheKey, Arc<dyn LlmProvider>>>,

    /// Maximum cache size (providers are evicted LRU-style when exceeded)
    max_cache_size: usize,
}

impl ProviderRouter {
    /// Create a new provider router
    pub fn new() -> Self {
        Self {
            llm_plugins: RwLock::new(HashMap::new()),
            provider_cache: RwLock::new(HashMap::new()),
            max_cache_size: 100,
        }
    }

    /// Create a new provider router with custom cache size
    pub fn with_cache_size(max_cache_size: usize) -> Self {
        Self {
            llm_plugins: RwLock::new(HashMap::new()),
            provider_cache: RwLock::new(HashMap::new()),
            max_cache_size,
        }
    }

    /// Register an LLM provider plugin
    pub async fn register_llm_plugin(&self, plugin: Arc<dyn LlmProviderPlugin>) {
        let mut plugins = self.llm_plugins.write().await;

        for cred_type in plugin.supported_credential_types() {
            let key = credential_type_to_key(&cred_type);
            info!(
                credential_type = %key,
                plugin_id = %plugin.metadata().id,
                "Registering LLM plugin for credential type"
            );
            plugins.insert(key, plugin.clone());
        }
    }

    /// Get or create an LLM provider for a model
    ///
    /// This method:
    /// 1. Looks up the model's credential type
    /// 2. Finds the appropriate plugin for that credential type
    /// 3. Creates (or retrieves from cache) a provider instance
    ///
    /// # Arguments
    /// * `model` - The model configuration
    /// * `credential` - The credential to use for authentication
    ///
    /// # Returns
    /// * `Ok(Arc<dyn LlmProvider>)` - The provider instance
    /// * `Err(PluginError)` - If no plugin supports the credential type or creation fails
    pub async fn get_provider(
        &self,
        model: &Model,
        credential: &Credential,
    ) -> Result<Arc<dyn LlmProvider>, PluginError> {
        let credential_type = credential.credential_type();
        let credential_id = model.credential_id();

        // Check cache first
        let cache_key = ProviderCacheKey::new(&credential_type, credential_id);

        {
            let cache = self.provider_cache.read().await;
            if let Some(provider) = cache.get(&cache_key) {
                debug!(
                    credential_id = %credential_id,
                    "Returning cached provider"
                );
                return Ok(provider.clone());
            }
        }

        // Find plugin for credential type
        let plugin = self.get_plugin_for_credential_type(&credential_type).await?;

        // Create provider configuration
        let config = LlmProviderConfig::new(
            credential_type.clone(),
            credential_id,
            credential.api_key(),
        )
        .with_params(
            credential
                .additional_params()
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        );

        // Create provider instance
        let provider = plugin.create_llm_provider(config).await?;

        // Cache the provider
        self.cache_provider(cache_key, provider.clone()).await;

        Ok(provider)
    }

    /// Get the plugin that supports a credential type
    async fn get_plugin_for_credential_type(
        &self,
        credential_type: &CredentialType,
    ) -> Result<Arc<dyn LlmProviderPlugin>, PluginError> {
        let key = credential_type_to_key(credential_type);
        let plugins = self.llm_plugins.read().await;

        plugins
            .get(&key)
            .cloned()
            .ok_or_else(|| PluginError::no_plugin_for_credential_type(&key))
    }

    /// Cache a provider instance
    async fn cache_provider(&self, key: ProviderCacheKey, provider: Arc<dyn LlmProvider>) {
        let mut cache = self.provider_cache.write().await;

        // Simple eviction: if cache is full, clear oldest half
        if cache.len() >= self.max_cache_size {
            let to_remove: Vec<_> = cache
                .keys()
                .take(self.max_cache_size / 2)
                .cloned()
                .collect();

            for k in to_remove {
                cache.remove(&k);
            }
        }

        cache.insert(key, provider);
    }

    /// Clear the provider cache
    pub async fn clear_cache(&self) {
        let mut cache = self.provider_cache.write().await;
        cache.clear();
    }

    /// Get cache statistics
    pub async fn cache_stats(&self) -> CacheStats {
        let cache = self.provider_cache.read().await;
        CacheStats {
            size: cache.len(),
            max_size: self.max_cache_size,
        }
    }

    /// List all registered credential types
    pub async fn list_supported_credential_types(&self) -> Vec<String> {
        let plugins = self.llm_plugins.read().await;
        plugins.keys().cloned().collect()
    }

    /// Check if a credential type is supported
    pub async fn supports_credential_type(&self, credential_type: &CredentialType) -> bool {
        let key = credential_type_to_key(credential_type);
        let plugins = self.llm_plugins.read().await;
        plugins.contains_key(&key)
    }
}

impl Default for ProviderRouter {
    fn default() -> Self {
        Self::new()
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub size: usize,
    pub max_size: usize,
}

/// Convert CredentialType to a string key
fn credential_type_to_key(cred_type: &CredentialType) -> String {
    match cred_type {
        CredentialType::OpenAi => "openai".to_string(),
        CredentialType::Anthropic => "anthropic".to_string(),
        CredentialType::AzureOpenAi => "azure_openai".to_string(),
        CredentialType::AwsBedrock => "aws_bedrock".to_string(),
        CredentialType::Pgvector => "pgvector".to_string(),
        CredentialType::AwsKnowledgeBase => "aws_knowledge_base".to_string(),
        CredentialType::Pinecone => "pinecone".to_string(),
        CredentialType::HttpApiKey => "http_api_key".to_string(),
        CredentialType::Custom(name) => format!("custom_{}", name),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_credential_type_to_key() {
        assert_eq!(credential_type_to_key(&CredentialType::OpenAi), "openai");
        assert_eq!(
            credential_type_to_key(&CredentialType::Anthropic),
            "anthropic"
        );
        assert_eq!(
            credential_type_to_key(&CredentialType::AzureOpenAi),
            "azure_openai"
        );
        assert_eq!(
            credential_type_to_key(&CredentialType::AwsBedrock),
            "aws_bedrock"
        );
    }

    #[test]
    fn test_provider_cache_key() {
        let key1 = ProviderCacheKey::new(&CredentialType::OpenAi, "openai-default");
        let key2 = ProviderCacheKey::new(&CredentialType::OpenAi, "openai-default");
        let key3 = ProviderCacheKey::new(&CredentialType::OpenAi, "openai-prod");

        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
    }

    #[tokio::test]
    async fn test_router_creation() {
        let router = ProviderRouter::new();
        let types = router.list_supported_credential_types().await;
        assert!(types.is_empty());

        let stats = router.cache_stats().await;
        assert_eq!(stats.size, 0);
        assert_eq!(stats.max_size, 100);
    }

    #[tokio::test]
    async fn test_router_with_cache_size() {
        let router = ProviderRouter::with_cache_size(50);
        let stats = router.cache_stats().await;
        assert_eq!(stats.max_size, 50);
    }
}
