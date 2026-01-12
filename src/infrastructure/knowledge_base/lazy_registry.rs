//! Lazy knowledge base provider registry that auto-creates providers on demand

use std::sync::Arc;

use async_trait::async_trait;
use sqlx::PgPool;

use super::{KnowledgeBaseProviderRegistry, KnowledgeBaseProviderRegistryTrait, PgvectorConfig};
use crate::domain::knowledge_base::{KnowledgeBaseId, KnowledgeBaseProvider, KnowledgeBaseType};
use crate::domain::model::ModelId;
use crate::domain::storage::Storage;
use crate::domain::{DomainError, KnowledgeBase, Model, StoredCredential};
use crate::infrastructure::credentials::CredentialServiceTrait;
use crate::infrastructure::embedding::{HttpClient, OpenAiEmbeddingProvider};

/// Configuration for lazy provider creation
#[derive(Clone)]
pub struct LazyRegistryConfig {
    /// PostgreSQL pool for pgvector providers
    pub pg_pool: Option<PgPool>,
}

impl LazyRegistryConfig {
    pub fn new() -> Self {
        Self { pg_pool: None }
    }

    pub fn with_pg_pool(mut self, pool: PgPool) -> Self {
        self.pg_pool = Some(pool);
        self
    }
}

impl Default for LazyRegistryConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Adapter to make OpenAI embedding provider work with pgvector
#[derive(Debug)]
pub struct PgvectorEmbeddingAdapter {
    provider: OpenAiEmbeddingProvider<HttpClient>,
    model: String,
    dimensions: u32,
}

impl PgvectorEmbeddingAdapter {
    pub fn new(
        provider: OpenAiEmbeddingProvider<HttpClient>,
        model: String,
        dimensions: u32,
    ) -> Self {
        Self {
            provider,
            model,
            dimensions,
        }
    }
}

#[async_trait]
impl super::EmbeddingProvider for PgvectorEmbeddingAdapter {
    async fn embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>, DomainError> {
        use crate::domain::embedding::{EmbeddingProvider as DomainEmbeddingProvider, EmbeddingRequest};

        let request = EmbeddingRequest::batch(&self.model, texts);
        let response = self.provider.embed(request).await?;

        Ok(response
            .embeddings()
            .iter()
            .map(|e| e.vector().to_vec())
            .collect())
    }

    fn dimensions(&self) -> u32 {
        self.dimensions
    }
}

/// Lazy knowledge base provider registry
///
/// This registry lazily creates and caches knowledge base providers when they are first accessed.
/// It requires access to:
/// - KB storage (to get KB metadata/configuration)
/// - Model storage (to get embedding model configuration)
/// - Credential service (to get embedding API keys)
/// - PostgreSQL pool (for pgvector providers)
pub struct LazyKnowledgeBaseProviderRegistry {
    inner: Arc<KnowledgeBaseProviderRegistry>,
    kb_storage: Arc<dyn Storage<KnowledgeBase>>,
    model_storage: Arc<dyn Storage<Model>>,
    credential_service: Arc<dyn CredentialServiceTrait>,
    config: LazyRegistryConfig,
}

impl std::fmt::Debug for LazyKnowledgeBaseProviderRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LazyKnowledgeBaseProviderRegistry")
            .field("has_pg_pool", &self.config.pg_pool.is_some())
            .finish()
    }
}

impl LazyKnowledgeBaseProviderRegistry {
    /// Create a new lazy registry
    pub fn new(
        inner: Arc<KnowledgeBaseProviderRegistry>,
        kb_storage: Arc<dyn Storage<KnowledgeBase>>,
        model_storage: Arc<dyn Storage<Model>>,
        credential_service: Arc<dyn CredentialServiceTrait>,
        config: LazyRegistryConfig,
    ) -> Self {
        Self {
            inner,
            kb_storage,
            model_storage,
            credential_service,
            config,
        }
    }

    /// Try to create a provider for the given knowledge base
    async fn create_provider(
        &self,
        kb: &KnowledgeBase,
    ) -> Result<Arc<dyn KnowledgeBaseProvider>, DomainError> {
        match kb.kb_type() {
            KnowledgeBaseType::Pgvector => self.create_pgvector_provider(kb).await,
            KnowledgeBaseType::AwsKnowledgeBase => Err(DomainError::knowledge_base(
                "AWS Knowledge Base auto-registration not yet implemented".to_string(),
            )),
            other => Err(DomainError::knowledge_base(format!(
                "Provider type '{:?}' auto-registration not supported",
                other
            ))),
        }
    }

    /// Create a pgvector provider
    async fn create_pgvector_provider(
        &self,
        kb: &KnowledgeBase,
    ) -> Result<Arc<dyn KnowledgeBaseProvider>, DomainError> {
        // Try to get PostgreSQL pool from credential first, then fall back to global config
        let pool = self.get_pgvector_pool(kb).await?;

        // Get embedding model ID from connection config
        let embedding_model_id = kb
            .connection_config()
            .and_then(|cc| cc.get("embedding_model_id"))
            .ok_or_else(|| {
                DomainError::knowledge_base(format!(
                    "Knowledge base '{}' has no embedding_model_id configured",
                    kb.id().as_str()
                ))
            })?;

        // Parse the model ID
        let model_id = ModelId::new(embedding_model_id).map_err(|e| {
            DomainError::knowledge_base(format!(
                "Invalid embedding_model_id '{}': {}",
                embedding_model_id, e
            ))
        })?;

        // Get the embedding model
        let model = self
            .model_storage
            .get(&model_id)
            .await?
            .ok_or_else(|| {
                DomainError::knowledge_base(format!(
                    "Embedding model '{}' not found for knowledge base '{}'",
                    embedding_model_id,
                    kb.id().as_str()
                ))
            })?;

        // Get the credential from the model
        let credential = self
            .credential_service
            .get(model.credential_id())
            .await?
            .ok_or_else(|| {
                DomainError::knowledge_base(format!(
                    "Credential '{}' not found for embedding model '{}'",
                    model.credential_id(),
                    embedding_model_id
                ))
            })?;

        // Create embedding provider from model and credential
        let embedding_provider = self.create_embedding_provider(&model, &credential, kb)?;

        // Create pgvector config
        let pgvector_config = PgvectorConfig::new(kb.embedding().dimensions);

        // Create the provider
        // Note: Tables are created via migrations (db/migrations/20260112000001_create_knowledge_base_documents.sql)
        let provider = super::PgvectorKnowledgeBase::new(
            kb.id().clone(),
            pool,
            pgvector_config,
            embedding_provider,
        );

        Ok(Arc::new(provider))
    }

    /// Create an embedding provider from a model and credential
    fn create_embedding_provider(
        &self,
        model: &Model,
        credential: &StoredCredential,
        kb: &KnowledgeBase,
    ) -> Result<PgvectorEmbeddingAdapter, DomainError> {
        let api_key = credential.api_key();

        // Use endpoint from credential if available, otherwise default OpenAI
        let base_url = credential
            .endpoint()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "https://api.openai.com".to_string());

        let http_client = HttpClient::new();

        // Create OpenAI-compatible embedding provider
        let provider = OpenAiEmbeddingProvider::with_base_url(http_client, api_key, base_url);

        tracing::info!(
            kb_id = kb.id().as_str(),
            embedding_model_id = model.id().as_str(),
            provider_model = model.provider_model(),
            "Created embedding provider for knowledge base"
        );

        Ok(PgvectorEmbeddingAdapter::new(
            provider,
            model.provider_model().to_string(),
            kb.embedding().dimensions,
        ))
    }

    /// Get PostgreSQL pool for a pgvector knowledge base
    ///
    /// First tries to get the connection from a credential specified in connection_config,
    /// then falls back to the global pg_pool from LazyRegistryConfig.
    async fn get_pgvector_pool(&self, kb: &KnowledgeBase) -> Result<PgPool, DomainError> {
        // Check for credential_id in connection_config
        if let Some(credential_id) = kb
            .connection_config()
            .and_then(|cc| cc.get("credential_id"))
        {
            tracing::info!(
                kb_id = kb.id().as_str(),
                credential_id = credential_id,
                "Using credential for pgvector connection"
            );

            // Get the credential
            let credential = self
                .credential_service
                .get(credential_id)
                .await?
                .ok_or_else(|| {
                    DomainError::knowledge_base(format!(
                        "Database credential '{}' not found for knowledge base '{}'",
                        credential_id,
                        kb.id().as_str()
                    ))
                })?;

            // The api_key contains the PostgreSQL connection string
            let connection_string = credential.api_key();

            // Create a new pool for this credential
            let pool = PgPool::connect(connection_string).await.map_err(|e| {
                DomainError::knowledge_base(format!(
                    "Failed to connect to PostgreSQL using credential '{}': {}",
                    credential_id, e
                ))
            })?;

            return Ok(pool);
        }

        // Fall back to global pg_pool
        self.config.pg_pool.clone().ok_or_else(|| {
            DomainError::knowledge_base(format!(
                "No database credential configured for knowledge base '{}' and DATABASE_URL not set. \
                 Either add 'credential_id' to the KB's connection_config pointing to a Pgvector credential, \
                 or set the DATABASE_URL environment variable.",
                kb.id().as_str()
            ))
        })
    }
}

#[async_trait]
impl KnowledgeBaseProviderRegistryTrait for LazyKnowledgeBaseProviderRegistry {
    async fn register(&self, provider: Arc<dyn KnowledgeBaseProvider>) {
        self.inner.register(provider).await;
    }

    async fn get(&self, kb_id: &str) -> Option<Arc<dyn KnowledgeBaseProvider>> {
        // Parse KB ID
        let kb_id_parsed = match KnowledgeBaseId::new(kb_id) {
            Ok(id) => id,
            Err(_) => return None,
        };

        // Get KB metadata
        let kb = match self.kb_storage.get(&kb_id_parsed).await {
            Ok(Some(kb)) => kb,
            _ => {
                // KB not in storage - check if provider is pre-registered
                return self.inner.get(kb_id).await;
            }
        };

        // Check if KB is enabled
        if !kb.is_enabled() {
            tracing::warn!(kb_id = kb_id, "Knowledge base is disabled");
            return None;
        }

        // For credential-based connections, always recreate to pick up credential changes
        let uses_credential = kb
            .connection_config()
            .and_then(|cc| cc.get("credential_id"))
            .is_some();

        // Check if provider already exists (only for non-credential connections)
        if !uses_credential {
            if let Some(provider) = self.inner.get(kb_id).await {
                return Some(provider);
            }
        }

        // Try to create and register the provider
        match self.create_provider(&kb).await {
            Ok(provider) => {
                tracing::info!(
                    kb_id = kb_id,
                    provider_type = provider.provider_type(),
                    uses_credential = uses_credential,
                    "Created knowledge base provider"
                );

                // Only cache providers that don't use credential-based connections
                if !uses_credential {
                    self.inner.register(provider.clone()).await;
                }

                Some(provider)
            }
            Err(e) => {
                tracing::error!(
                    kb_id = kb_id,
                    error = %e,
                    "Failed to create knowledge base provider"
                );
                None
            }
        }
    }

    async fn get_required(
        &self,
        kb_id: &str,
    ) -> Result<Arc<dyn KnowledgeBaseProvider>, DomainError> {
        // Parse KB ID
        let kb_id_parsed = KnowledgeBaseId::new(kb_id).map_err(|e| {
            DomainError::knowledge_base(format!("Invalid knowledge base ID '{}': {}", kb_id, e))
        })?;

        // Get KB metadata
        let kb = self
            .kb_storage
            .get(&kb_id_parsed)
            .await?
            .ok_or_else(|| {
                DomainError::knowledge_base(format!("Knowledge base '{}' not found", kb_id))
            })?;

        // Check if KB is enabled
        if !kb.is_enabled() {
            return Err(DomainError::knowledge_base(format!(
                "Knowledge base '{}' is disabled",
                kb_id
            )));
        }

        // For credential-based connections, always recreate to pick up credential changes
        let uses_credential = kb
            .connection_config()
            .and_then(|cc| cc.get("credential_id"))
            .is_some();

        // Check if provider already exists (only for non-credential connections)
        if !uses_credential {
            if let Some(provider) = self.inner.get(kb_id).await {
                return Ok(provider);
            }
        }

        // Try to create provider - surface actual error instead of generic message
        let provider = self.create_provider(&kb).await?;

        tracing::info!(
            kb_id = kb_id,
            provider_type = provider.provider_type(),
            uses_credential = uses_credential,
            "Created knowledge base provider"
        );

        // Only cache providers that don't use credential-based connections
        if !uses_credential {
            self.inner.register(provider.clone()).await;
        }

        Ok(provider)
    }

    async fn has_provider(&self, kb_id: &str) -> bool {
        // Check inner registry first
        if self.inner.has_provider(kb_id).await {
            return true;
        }

        // Check if KB exists in storage (could be lazily created)
        let kb_id_parsed = match KnowledgeBaseId::new(kb_id) {
            Ok(id) => id,
            Err(_) => return false,
        };

        matches!(self.kb_storage.exists(&kb_id_parsed).await, Ok(true))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::credentials::{CredentialId, CredentialType};
    use crate::domain::storage::mock::MockStorage;
    use crate::domain::EmbeddingConfig;
    use std::collections::HashMap;

    /// Mock credential service for testing
    #[derive(Debug)]
    struct MockCredentialService {
        credentials: tokio::sync::RwLock<HashMap<String, StoredCredential>>,
    }

    impl MockCredentialService {
        fn new() -> Self {
            Self {
                credentials: tokio::sync::RwLock::new(HashMap::new()),
            }
        }

        async fn add_credential(&self, cred: StoredCredential) {
            self.credentials
                .write()
                .await
                .insert(cred.id().as_str().to_string(), cred);
        }
    }

    #[async_trait]
    impl CredentialServiceTrait for MockCredentialService {
        async fn get(&self, id: &str) -> Result<Option<StoredCredential>, DomainError> {
            Ok(self.credentials.read().await.get(id).cloned())
        }

        async fn list(&self) -> Result<Vec<StoredCredential>, DomainError> {
            Ok(self.credentials.read().await.values().cloned().collect())
        }
    }

    #[tokio::test]
    async fn test_lazy_registry_no_pg_pool() {
        let inner = Arc::new(KnowledgeBaseProviderRegistry::new());
        let kb_storage = Arc::new(MockStorage::<KnowledgeBase>::new());
        let model_storage = Arc::new(MockStorage::<Model>::new());
        let cred_service = Arc::new(MockCredentialService::new());
        let config = LazyRegistryConfig::new(); // No PG pool

        let registry = LazyKnowledgeBaseProviderRegistry::new(
            inner,
            kb_storage.clone(),
            model_storage.clone(),
            cred_service.clone(),
            config,
        );

        // Create a pgvector KB with embedding_model_id
        let kb_id = KnowledgeBaseId::new("test-kb").unwrap();
        let mut kb = KnowledgeBase::new(
            kb_id,
            "Test KB",
            KnowledgeBaseType::Pgvector,
            EmbeddingConfig::new("text-embedding-3-small", 1536),
        );
        let mut conn_config = HashMap::new();
        conn_config.insert(
            "embedding_model_id".to_string(),
            "text-embedding-3-small".to_string(),
        );
        kb = kb.with_connection_config(conn_config);
        kb_storage.save(kb).await.unwrap();

        // Add model
        let model = Model::new(
            ModelId::new("text-embedding-3-small").unwrap(),
            "Text Embedding 3 Small",
            CredentialType::OpenAi,
            "text-embedding-3-small",
            "test-cred",
        );
        model_storage.save(model).await.unwrap();

        // Add credential
        let cred = StoredCredential::new(
            CredentialId::new("test-cred").unwrap(),
            "Test Credential",
            CredentialType::OpenAi,
            "sk-test-key",
        );
        cred_service.add_credential(cred).await;

        // Try to get provider - should fail because no PG pool
        let result = registry.get("test-kb").await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_lazy_registry_kb_not_found() {
        let inner = Arc::new(KnowledgeBaseProviderRegistry::new());
        let kb_storage = Arc::new(MockStorage::<KnowledgeBase>::new());
        let model_storage = Arc::new(MockStorage::<Model>::new());
        let cred_service = Arc::new(MockCredentialService::new());
        let config = LazyRegistryConfig::new();

        let registry = LazyKnowledgeBaseProviderRegistry::new(
            inner,
            kb_storage,
            model_storage,
            cred_service,
            config,
        );

        // Try to get non-existent KB
        let result = registry.get("nonexistent-kb").await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_lazy_registry_uses_existing_provider() {
        let inner = Arc::new(KnowledgeBaseProviderRegistry::new());
        let kb_storage = Arc::new(MockStorage::<KnowledgeBase>::new());
        let model_storage = Arc::new(MockStorage::<Model>::new());
        let cred_service = Arc::new(MockCredentialService::new());
        let config = LazyRegistryConfig::new();

        // Pre-register a mock provider
        let kb_id = KnowledgeBaseId::new("test-kb").unwrap();
        let mock_provider: Arc<dyn KnowledgeBaseProvider> = Arc::new(
            crate::infrastructure::knowledge_base::InMemoryKnowledgeBaseProvider::new(kb_id),
        );
        inner.register(mock_provider.clone()).await;

        let registry = LazyKnowledgeBaseProviderRegistry::new(
            inner,
            kb_storage,
            model_storage,
            cred_service,
            config,
        );

        // Should return the existing provider
        let result = registry.get("test-kb").await;
        assert!(result.is_some());
    }
}
