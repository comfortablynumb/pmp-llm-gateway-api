//! Knowledge base provider implementations

mod aws;
mod factory;
mod pgvector;

pub use aws::{AwsKnowledgeBase, AwsKnowledgeBaseConfig};
pub use factory::{KnowledgeBaseFactory, KnowledgeBaseProviderConfig};
pub use pgvector::{
    DistanceMetric, EmbeddingProvider, PgvectorConfig, PgvectorKnowledgeBase,
};

#[cfg(test)]
pub use pgvector::mock::MockEmbeddingProvider;
