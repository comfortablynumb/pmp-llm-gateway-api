//! Knowledge base provider factory

use std::sync::Arc;

use sqlx::postgres::PgPool;

use crate::domain::knowledge_base::{KnowledgeBaseId, KnowledgeBaseProvider, KnowledgeBaseType};
use crate::domain::DomainError;

use super::aws::{AwsKnowledgeBase, AwsKnowledgeBaseConfig};
use super::pgvector::{EmbeddingProvider, PgvectorConfig, PgvectorKnowledgeBase};

/// Factory for creating knowledge base providers
#[derive(Debug)]
pub struct KnowledgeBaseFactory;

impl KnowledgeBaseFactory {
    /// Create a new pgvector-based knowledge base provider
    pub fn create_pgvector<E: EmbeddingProvider + 'static>(
        id: KnowledgeBaseId,
        pool: PgPool,
        config: PgvectorConfig,
        embedding_provider: E,
    ) -> Arc<dyn KnowledgeBaseProvider> {
        Arc::new(PgvectorKnowledgeBase::new(id, pool, config, embedding_provider))
    }

    /// Create a new AWS Bedrock Knowledge Base provider
    pub async fn create_aws(
        id: KnowledgeBaseId,
        config: AwsKnowledgeBaseConfig,
    ) -> Result<Arc<dyn KnowledgeBaseProvider>, DomainError> {
        let provider = AwsKnowledgeBase::new(id, config).await?;
        Ok(Arc::new(provider))
    }

    /// Create a knowledge base provider from configuration
    pub async fn create(
        id: KnowledgeBaseId,
        kb_type: &KnowledgeBaseType,
        config: KnowledgeBaseProviderConfig,
    ) -> Result<Arc<dyn KnowledgeBaseProvider>, DomainError> {
        match (kb_type, config) {
            (KnowledgeBaseType::Pgvector, KnowledgeBaseProviderConfig::Pgvector(cfg)) => {
                Err(DomainError::knowledge_base(
                    "Pgvector requires an embedding provider. Use create_pgvector() directly."
                        .to_string(),
                ))
            }
            (KnowledgeBaseType::AwsKnowledgeBase, KnowledgeBaseProviderConfig::Aws(cfg)) => {
                Self::create_aws(id, cfg).await
            }
            (KnowledgeBaseType::Pinecone, _) => Err(DomainError::knowledge_base(
                "Pinecone provider not yet implemented".to_string(),
            )),
            (KnowledgeBaseType::Weaviate, _) => Err(DomainError::knowledge_base(
                "Weaviate provider not yet implemented".to_string(),
            )),
            (KnowledgeBaseType::Qdrant, _) => Err(DomainError::knowledge_base(
                "Qdrant provider not yet implemented".to_string(),
            )),
            _ => Err(DomainError::knowledge_base(format!(
                "Configuration mismatch for knowledge base type: {}",
                kb_type
            ))),
        }
    }
}

/// Configuration for creating knowledge base providers
#[derive(Debug, Clone)]
pub enum KnowledgeBaseProviderConfig {
    /// pgvector configuration (pool must be provided separately)
    Pgvector(PgvectorConfig),
    /// AWS Knowledge Base configuration
    Aws(AwsKnowledgeBaseConfig),
    /// Pinecone configuration (not yet implemented)
    Pinecone,
    /// Weaviate configuration (not yet implemented)
    Weaviate,
    /// Qdrant configuration (not yet implemented)
    Qdrant,
}

impl From<PgvectorConfig> for KnowledgeBaseProviderConfig {
    fn from(config: PgvectorConfig) -> Self {
        Self::Pgvector(config)
    }
}

impl From<AwsKnowledgeBaseConfig> for KnowledgeBaseProviderConfig {
    fn from(config: AwsKnowledgeBaseConfig) -> Self {
        Self::Aws(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::knowledge_base::pgvector::mock::MockEmbeddingProvider;

    #[test]
    fn test_pgvector_config_conversion() {
        let config = PgvectorConfig::new(1536);
        let provider_config: KnowledgeBaseProviderConfig = config.into();

        assert!(matches!(
            provider_config,
            KnowledgeBaseProviderConfig::Pgvector(_)
        ));
    }

    #[test]
    fn test_aws_config_conversion() {
        let config = AwsKnowledgeBaseConfig::new("kb-123");
        let provider_config: KnowledgeBaseProviderConfig = config.into();

        assert!(matches!(
            provider_config,
            KnowledgeBaseProviderConfig::Aws(_)
        ));
    }
}
