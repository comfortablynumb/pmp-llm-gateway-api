//! Knowledge Base domain - Vector search and retrieval

mod document;
mod entity;
mod filter;
mod provider;
mod validation;

pub use document::{
    CreateChunkRequest, CreateDocumentRequest, DocumentChunk, DocumentSummary,
    KnowledgeBaseDocument,
};
pub use entity::{
    EmbeddingConfig, KnowledgeBase, KnowledgeBaseConfig, KnowledgeBaseId, KnowledgeBaseType,
    SearchQuery, SearchResult,
};
pub use filter::{
    FilterBuilder, FilterCondition, FilterConnector, FilterOperator, FilterValue, MetadataFilter,
};
pub use provider::{
    AddDocumentsResult, DeleteDocumentsResult, Document, KnowledgeBaseProvider, SearchParams,
    SourceInfo,
};
pub use validation::{validate_knowledge_base_id, KnowledgeBaseValidationError};

#[cfg(test)]
pub use provider::mock::MockKnowledgeBaseProvider;
