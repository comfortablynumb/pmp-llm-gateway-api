//! Knowledge base provider implementations

mod aws;
mod factory;
mod in_memory;
mod lazy_registry;
mod pgvector;
mod registry;

pub use aws::{AwsKnowledgeBase, AwsKnowledgeBaseConfig};
pub use factory::{KnowledgeBaseFactory, KnowledgeBaseProviderConfig};
pub use in_memory::InMemoryKnowledgeBaseProvider;
pub use lazy_registry::{LazyKnowledgeBaseProviderRegistry, LazyRegistryConfig};
pub use pgvector::{DistanceMetric, EmbeddingProvider, PgvectorConfig, PgvectorKnowledgeBase};
pub use registry::{KnowledgeBaseProviderRegistry, KnowledgeBaseProviderRegistryTrait};

#[cfg(test)]
pub use pgvector::mock::MockEmbeddingProvider;
