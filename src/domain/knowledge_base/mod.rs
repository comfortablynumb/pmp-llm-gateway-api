//! Knowledge Base domain - Vector search and retrieval

mod entity;
mod filter;
mod provider;
mod validation;

pub use entity::{
    EmbeddingConfig, KnowledgeBase, KnowledgeBaseConfig, KnowledgeBaseId, KnowledgeBaseType,
    SearchQuery, SearchResult,
};
pub use filter::{
    FilterBuilder, FilterCondition, FilterConnector, FilterOperator, FilterValue, MetadataFilter,
};
pub use provider::{
    AddDocumentsResult, DeleteDocumentsResult, Document, KnowledgeBaseProvider, SearchParams,
};
pub use validation::{validate_knowledge_base_id, KnowledgeBaseValidationError};

#[cfg(test)]
pub use provider::mock::MockKnowledgeBaseProvider;
