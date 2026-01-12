//! Knowledge base provider trait

use std::fmt::Debug;

use async_trait::async_trait;

use super::document::{
    CreateDocumentRequest, DocumentChunk, DocumentSummary, KnowledgeBaseDocument,
};
use super::entity::{KnowledgeBaseId, SearchResult};
use super::filter::MetadataFilter;
use crate::domain::error::DomainError;
use uuid::Uuid;

/// Document to be added to a knowledge base
#[derive(Debug, Clone)]
pub struct Document {
    /// Unique identifier for the document
    pub id: String,
    /// Document content text
    pub content: String,
    /// Optional metadata key-value pairs
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
    /// Optional source reference
    pub source: Option<String>,
}

impl Document {
    /// Create a new document
    pub fn new(id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            content: content.into(),
            metadata: std::collections::HashMap::new(),
            source: None,
        }
    }

    /// Add metadata to the document
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    /// Set all metadata
    pub fn with_all_metadata(
        mut self,
        metadata: std::collections::HashMap<String, serde_json::Value>,
    ) -> Self {
        self.metadata = metadata;
        self
    }

    /// Set source reference
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }
}

/// Search parameters for knowledge base queries
#[derive(Debug, Clone, Default)]
pub struct SearchParams {
    /// Query text to search for
    pub query: String,
    /// Number of results to return
    pub top_k: u32,
    /// Minimum similarity threshold (0.0 - 1.0)
    pub similarity_threshold: f32,
    /// Optional metadata filter
    pub filter: Option<MetadataFilter>,
    /// Whether to include embeddings in results
    pub include_embeddings: bool,
    /// Whether to include metadata in results
    pub include_metadata: bool,
}

impl SearchParams {
    /// Create new search parameters
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            top_k: 10,
            similarity_threshold: 0.7,
            filter: None,
            include_embeddings: false,
            include_metadata: true,
        }
    }

    /// Set number of results
    pub fn with_top_k(mut self, top_k: u32) -> Self {
        self.top_k = top_k;
        self
    }

    /// Set similarity threshold
    pub fn with_similarity_threshold(mut self, threshold: f32) -> Self {
        self.similarity_threshold = threshold;
        self
    }

    /// Set metadata filter
    pub fn with_filter(mut self, filter: MetadataFilter) -> Self {
        self.filter = Some(filter);
        self
    }

    /// Set whether to include embeddings
    pub fn with_include_embeddings(mut self, include: bool) -> Self {
        self.include_embeddings = include;
        self
    }

    /// Set whether to include metadata
    pub fn with_include_metadata(mut self, include: bool) -> Self {
        self.include_metadata = include;
        self
    }
}

/// Result of adding documents to a knowledge base
#[derive(Debug, Clone)]
pub struct AddDocumentsResult {
    /// Number of documents successfully added
    pub added: usize,
    /// Number of documents that failed
    pub failed: usize,
    /// IDs of documents that failed with error messages
    pub errors: Vec<(String, String)>,
}

impl AddDocumentsResult {
    /// Create a successful result
    pub fn success(added: usize) -> Self {
        Self {
            added,
            failed: 0,
            errors: Vec::new(),
        }
    }

    /// Create a result with partial failures
    pub fn partial(added: usize, errors: Vec<(String, String)>) -> Self {
        Self {
            added,
            failed: errors.len(),
            errors,
        }
    }
}

/// Result of deleting documents from a knowledge base
#[derive(Debug, Clone)]
pub struct DeleteDocumentsResult {
    /// Number of documents deleted
    pub deleted: usize,
    /// Number of documents not found
    pub not_found: usize,
}

impl DeleteDocumentsResult {
    /// Create a new delete result
    pub fn new(deleted: usize, not_found: usize) -> Self {
        Self { deleted, not_found }
    }
}

/// Provider trait for knowledge base operations
///
/// Implementations should handle the specific backend (pgvector, AWS, etc.)
/// and translate between the common interface and backend-specific operations.
#[async_trait]
pub trait KnowledgeBaseProvider: Send + Sync + Debug {
    /// Get the knowledge base ID this provider is configured for
    fn knowledge_base_id(&self) -> &KnowledgeBaseId;

    /// Get the provider type name
    fn provider_type(&self) -> &'static str;

    /// Search the knowledge base
    async fn search(&self, params: SearchParams) -> Result<Vec<SearchResult>, DomainError>;

    /// Add documents to the knowledge base
    async fn add_documents(&self, documents: Vec<Document>) -> Result<AddDocumentsResult, DomainError>;

    /// Delete documents by their IDs
    async fn delete_documents(&self, ids: Vec<String>) -> Result<DeleteDocumentsResult, DomainError>;

    /// Delete documents matching a metadata filter
    async fn delete_by_filter(&self, filter: MetadataFilter) -> Result<DeleteDocumentsResult, DomainError>;

    /// Get a document by ID
    async fn get_document(&self, id: &str) -> Result<Option<SearchResult>, DomainError>;

    /// Check if the knowledge base is healthy and accessible
    async fn health_check(&self) -> Result<bool, DomainError>;

    /// Get the total document count in the knowledge base
    async fn document_count(&self) -> Result<usize, DomainError>;

    /// List all documents with a given source
    async fn list_by_source(&self, source: &str) -> Result<Vec<SearchResult>, DomainError>;

    /// Delete all documents with a given source
    async fn delete_by_source(&self, source: &str) -> Result<DeleteDocumentsResult, DomainError>;

    /// List all unique sources in the knowledge base
    async fn list_sources(&self) -> Result<Vec<SourceInfo>, DomainError>;

    /// Ensure the storage schema exists (create tables/indexes)
    async fn ensure_schema(&self) -> Result<(), DomainError>;

    // ========================================================================
    // New document-based methods (for the new schema)
    // ========================================================================

    /// Create a document with its chunks (new schema)
    async fn create_document(
        &self,
        request: CreateDocumentRequest,
    ) -> Result<KnowledgeBaseDocument, DomainError>;

    /// Get a document by ID (new schema)
    async fn get_document_by_id(&self, id: Uuid) -> Result<Option<KnowledgeBaseDocument>, DomainError>;

    /// List all documents in the knowledge base (new schema)
    async fn list_documents(&self) -> Result<Vec<DocumentSummary>, DomainError>;

    /// Get chunks for a document (new schema)
    async fn get_document_chunks(&self, document_id: Uuid) -> Result<Vec<DocumentChunk>, DomainError>;

    /// Delete a document and its chunks (new schema)
    async fn delete_document_by_id(&self, id: Uuid) -> Result<bool, DomainError>;

    /// Disable a document (soft delete - excludes from search) (new schema)
    async fn disable_document(&self, id: Uuid) -> Result<bool, DomainError>;

    /// Enable a previously disabled document (new schema)
    async fn enable_document(&self, id: Uuid) -> Result<bool, DomainError>;
}

/// Information about a document source
#[derive(Debug, Clone)]
pub struct SourceInfo {
    /// Source identifier
    pub source: String,
    /// Number of documents/chunks from this source
    pub document_count: usize,
}

#[cfg(test)]
pub mod mock {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use tokio::sync::RwLock;

    /// Mock knowledge base provider for testing
    #[derive(Debug)]
    pub struct MockKnowledgeBaseProvider {
        id: KnowledgeBaseId,
        documents: Arc<RwLock<Vec<SearchResult>>>,
        fixed_search_results: Arc<RwLock<Option<Vec<SearchResult>>>>,
        search_count: AtomicUsize,
        should_fail: Arc<RwLock<bool>>,
    }

    impl MockKnowledgeBaseProvider {
        /// Create a new mock provider
        pub fn new(id: KnowledgeBaseId) -> Self {
            Self {
                id,
                documents: Arc::new(RwLock::new(Vec::new())),
                fixed_search_results: Arc::new(RwLock::new(None)),
                search_count: AtomicUsize::new(0),
                should_fail: Arc::new(RwLock::new(false)),
            }
        }

        /// Set fixed search results (returned regardless of query)
        pub fn with_search_results(self, results: Vec<SearchResult>) -> Self {
            // Use blocking write since we're in a sync context during construction
            *futures::executor::block_on(self.fixed_search_results.write()) = Some(results);
            self
        }

        /// Get the number of search calls
        pub fn search_count(&self) -> usize {
            self.search_count.load(Ordering::SeqCst)
        }

        /// Set whether operations should fail
        pub async fn set_should_fail(&self, fail: bool) {
            *self.should_fail.write().await = fail;
        }

        /// Add a mock document directly
        pub async fn add_mock_result(&self, result: SearchResult) {
            self.documents.write().await.push(result);
        }

        async fn check_should_fail(&self) -> Result<(), DomainError> {
            if *self.should_fail.read().await {
                return Err(DomainError::KnowledgeBase(
                    "Mock provider configured to fail".to_string(),
                ));
            }
            Ok(())
        }
    }

    #[async_trait]
    impl KnowledgeBaseProvider for MockKnowledgeBaseProvider {
        fn knowledge_base_id(&self) -> &KnowledgeBaseId {
            &self.id
        }

        fn provider_type(&self) -> &'static str {
            "mock"
        }

        async fn search(&self, params: SearchParams) -> Result<Vec<SearchResult>, DomainError> {
            self.check_should_fail().await?;
            self.search_count.fetch_add(1, Ordering::SeqCst);

            // If fixed search results are set, return them directly
            if let Some(ref fixed) = *self.fixed_search_results.read().await {
                return Ok(fixed
                    .iter()
                    .take(params.top_k as usize)
                    .cloned()
                    .collect());
            }

            let docs = self.documents.read().await;
            let results: Vec<SearchResult> = docs
                .iter()
                .filter(|doc| {
                    doc.content
                        .to_lowercase()
                        .contains(&params.query.to_lowercase())
                })
                .filter(|doc| doc.score >= params.similarity_threshold)
                .take(params.top_k as usize)
                .cloned()
                .collect();

            Ok(results)
        }

        async fn add_documents(
            &self,
            documents: Vec<Document>,
        ) -> Result<AddDocumentsResult, DomainError> {
            self.check_should_fail().await?;

            let mut docs = self.documents.write().await;
            let count = documents.len();

            for doc in documents {
                let result = SearchResult::new(&doc.id, &doc.content, 1.0)
                    .with_all_metadata(doc.metadata);

                let result = if let Some(source) = doc.source {
                    result.with_source(source)
                } else {
                    result
                };

                docs.push(result);
            }

            Ok(AddDocumentsResult::success(count))
        }

        async fn delete_documents(
            &self,
            ids: Vec<String>,
        ) -> Result<DeleteDocumentsResult, DomainError> {
            self.check_should_fail().await?;

            let mut docs = self.documents.write().await;
            let initial_len = docs.len();

            docs.retain(|doc| !ids.contains(&doc.id));

            let deleted = initial_len - docs.len();
            let not_found = ids.len() - deleted;

            Ok(DeleteDocumentsResult::new(deleted, not_found))
        }

        async fn delete_by_filter(
            &self,
            _filter: MetadataFilter,
        ) -> Result<DeleteDocumentsResult, DomainError> {
            self.check_should_fail().await?;
            // Simplified: just return empty result for mock
            Ok(DeleteDocumentsResult::new(0, 0))
        }

        async fn get_document(&self, id: &str) -> Result<Option<SearchResult>, DomainError> {
            self.check_should_fail().await?;

            let docs = self.documents.read().await;
            Ok(docs.iter().find(|doc| doc.id == id).cloned())
        }

        async fn health_check(&self) -> Result<bool, DomainError> {
            self.check_should_fail().await?;
            Ok(true)
        }

        async fn document_count(&self) -> Result<usize, DomainError> {
            self.check_should_fail().await?;
            Ok(self.documents.read().await.len())
        }

        async fn list_by_source(&self, source: &str) -> Result<Vec<SearchResult>, DomainError> {
            self.check_should_fail().await?;

            let docs = self.documents.read().await;
            Ok(docs
                .iter()
                .filter(|doc| doc.source.as_deref() == Some(source))
                .cloned()
                .collect())
        }

        async fn delete_by_source(&self, source: &str) -> Result<DeleteDocumentsResult, DomainError> {
            self.check_should_fail().await?;

            let mut docs = self.documents.write().await;
            let initial_len = docs.len();

            docs.retain(|doc| doc.source.as_deref() != Some(source));

            let deleted = initial_len - docs.len();
            Ok(DeleteDocumentsResult::new(deleted, 0))
        }

        async fn list_sources(&self) -> Result<Vec<SourceInfo>, DomainError> {
            self.check_should_fail().await?;

            let docs = self.documents.read().await;
            let mut sources: std::collections::HashMap<String, usize> =
                std::collections::HashMap::new();

            for doc in docs.iter() {
                if let Some(source) = &doc.source {
                    *sources.entry(source.clone()).or_insert(0) += 1;
                }
            }

            Ok(sources
                .into_iter()
                .map(|(source, document_count)| SourceInfo {
                    source,
                    document_count,
                })
                .collect())
        }

        async fn ensure_schema(&self) -> Result<(), DomainError> {
            self.check_should_fail().await?;
            // Mock provider doesn't need schema setup
            Ok(())
        }

        async fn create_document(
            &self,
            _request: CreateDocumentRequest,
        ) -> Result<KnowledgeBaseDocument, DomainError> {
            self.check_should_fail().await?;
            Ok(KnowledgeBaseDocument::new(self.id.as_str()))
        }

        async fn get_document_by_id(&self, _id: Uuid) -> Result<Option<KnowledgeBaseDocument>, DomainError> {
            self.check_should_fail().await?;
            Ok(None)
        }

        async fn list_documents(&self) -> Result<Vec<DocumentSummary>, DomainError> {
            self.check_should_fail().await?;
            Ok(Vec::new())
        }

        async fn get_document_chunks(&self, _document_id: Uuid) -> Result<Vec<DocumentChunk>, DomainError> {
            self.check_should_fail().await?;
            Ok(Vec::new())
        }

        async fn delete_document_by_id(&self, _id: Uuid) -> Result<bool, DomainError> {
            self.check_should_fail().await?;
            Ok(true)
        }

        async fn disable_document(&self, _id: Uuid) -> Result<bool, DomainError> {
            self.check_should_fail().await?;
            Ok(true)
        }

        async fn enable_document(&self, _id: Uuid) -> Result<bool, DomainError> {
            self.check_should_fail().await?;
            Ok(true)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[tokio::test]
        async fn test_mock_provider_add_and_search() {
            let id = KnowledgeBaseId::new("test-kb").unwrap();
            let provider = MockKnowledgeBaseProvider::new(id);

            let doc = Document::new("doc1", "Hello world content");
            provider.add_documents(vec![doc]).await.unwrap();

            let params = SearchParams::new("hello");
            let results = provider.search(params).await.unwrap();

            assert_eq!(results.len(), 1);
            assert_eq!(results[0].id, "doc1");
            assert_eq!(provider.search_count(), 1);
        }

        #[tokio::test]
        async fn test_mock_provider_delete() {
            let id = KnowledgeBaseId::new("test-kb").unwrap();
            let provider = MockKnowledgeBaseProvider::new(id);

            let docs = vec![
                Document::new("doc1", "First document"),
                Document::new("doc2", "Second document"),
            ];
            provider.add_documents(docs).await.unwrap();

            let result = provider
                .delete_documents(vec!["doc1".to_string()])
                .await
                .unwrap();

            assert_eq!(result.deleted, 1);
            assert_eq!(result.not_found, 0);
            assert_eq!(provider.document_count().await.unwrap(), 1);
        }

        #[tokio::test]
        async fn test_mock_provider_failure() {
            let id = KnowledgeBaseId::new("test-kb").unwrap();
            let provider = MockKnowledgeBaseProvider::new(id);
            provider.set_should_fail(true).await;

            let result = provider.search(SearchParams::new("test")).await;
            assert!(result.is_err());
        }
    }
}
