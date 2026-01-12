//! Document ingestion service for knowledge bases

use std::collections::HashMap;
use std::sync::Arc;

use uuid::Uuid;

use crate::domain::embedding::{EmbeddingProvider, EmbeddingRequest};
use crate::domain::ingestion::{ChunkingConfig, ChunkingType, IngestionResult, ParserInput, ParserType};
use crate::domain::knowledge_base::{
    CreateChunkRequest, CreateDocumentRequest, Document, DocumentChunk, DocumentSummary,
    KnowledgeBaseDocument, SourceInfo,
};
use crate::domain::model::ModelId;
use crate::domain::storage::Storage;
use crate::domain::knowledge_base::KnowledgeBaseId;
use crate::domain::{DomainError, KnowledgeBase, Model};
use crate::infrastructure::credentials::CredentialServiceTrait;
use crate::infrastructure::embedding::{HttpClient, OpenAiEmbeddingProvider};
use crate::infrastructure::ingestion::{ChunkerFactory, ParserFactory};
use crate::infrastructure::knowledge_base::KnowledgeBaseProviderRegistryTrait;

/// Request to ingest a document into a knowledge base
#[derive(Debug, Clone)]
pub struct IngestDocumentRequest {
    pub content: String,
    pub filename: Option<String>,
    pub metadata: HashMap<String, serde_json::Value>,
    pub source_id: Option<String>,
    pub parser_type: Option<ParserType>,
    pub chunking_type: Option<ChunkingType>,
    pub chunk_size: Option<usize>,
    pub chunk_overlap: Option<usize>,
}

impl Default for IngestDocumentRequest {
    fn default() -> Self {
        Self {
            content: String::new(),
            filename: None,
            metadata: HashMap::new(),
            source_id: None,
            parser_type: None,
            chunking_type: None,
            chunk_size: None,
            chunk_overlap: None,
        }
    }
}

impl IngestDocumentRequest {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            ..Default::default()
        }
    }

    pub fn with_filename(mut self, filename: impl Into<String>) -> Self {
        self.filename = Some(filename.into());
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    pub fn with_source_id(mut self, source_id: impl Into<String>) -> Self {
        self.source_id = Some(source_id.into());
        self
    }
}

/// Request to ingest a document using the new schema (with proper document/chunk separation)
#[derive(Debug, Clone)]
pub struct IngestDocumentV2Request {
    pub content: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub filename: Option<String>,
    pub content_type: Option<String>,
    pub metadata: HashMap<String, serde_json::Value>,
    pub parser_type: Option<ParserType>,
    pub chunking_type: Option<ChunkingType>,
    pub chunk_size: Option<usize>,
    pub chunk_overlap: Option<usize>,
}

impl Default for IngestDocumentV2Request {
    fn default() -> Self {
        Self {
            content: String::new(),
            title: None,
            description: None,
            filename: None,
            content_type: None,
            metadata: HashMap::new(),
            parser_type: None,
            chunking_type: None,
            chunk_size: None,
            chunk_overlap: None,
        }
    }
}

impl IngestDocumentV2Request {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            ..Default::default()
        }
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_filename(mut self, filename: impl Into<String>) -> Self {
        self.filename = Some(filename.into());
        self
    }

    pub fn with_content_type(mut self, content_type: impl Into<String>) -> Self {
        self.content_type = Some(content_type.into());
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

/// Stored document information (returned by list operations)
#[derive(Debug, Clone)]
pub struct StoredDocument {
    pub id: String,
    pub content: String,
    pub metadata: HashMap<String, serde_json::Value>,
    pub source: Option<String>,
    pub chunk_index: usize,
    pub total_chunks: usize,
}

/// Configuration for dynamic embedding provider creation
pub struct EmbeddingConfig {
    kb_storage: Arc<dyn Storage<KnowledgeBase>>,
    model_storage: Arc<dyn Storage<Model>>,
    credential_service: Arc<dyn CredentialServiceTrait>,
}

impl std::fmt::Debug for EmbeddingConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EmbeddingConfig").finish()
    }
}

impl EmbeddingConfig {
    pub fn new(
        kb_storage: Arc<dyn Storage<KnowledgeBase>>,
        model_storage: Arc<dyn Storage<Model>>,
        credential_service: Arc<dyn CredentialServiceTrait>,
    ) -> Self {
        Self {
            kb_storage,
            model_storage,
            credential_service,
        }
    }

    async fn get_kb(&self, kb_id: &str) -> Result<KnowledgeBase, DomainError> {
        let id = KnowledgeBaseId::new(kb_id)
            .map_err(|e| DomainError::validation(format!("Invalid KB ID '{}': {}", kb_id, e)))?;
        self.kb_storage
            .get(&id)
            .await?
            .ok_or_else(|| DomainError::not_found(format!("KnowledgeBase '{}'", kb_id)))
    }

    async fn create_embedding_provider(
        &self,
        kb: &KnowledgeBase,
    ) -> Result<DynamicEmbeddingProvider, DomainError> {
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
        let api_key = credential.api_key();

        // Use endpoint from credential if available, otherwise default OpenAI
        let base_url = credential
            .endpoint()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "https://api.openai.com".to_string());

        let http_client = HttpClient::new();
        let provider = OpenAiEmbeddingProvider::with_base_url(http_client, api_key, base_url);

        tracing::info!(
            kb_id = kb.id().as_str(),
            embedding_model_id = model.id().as_str(),
            provider_model = model.provider_model(),
            "Created embedding provider for document ingestion"
        );

        Ok(DynamicEmbeddingProvider::new(
            provider,
            model.provider_model().to_string(),
            kb.embedding().dimensions,
        ))
    }
}

/// Dynamic embedding provider that wraps OpenAI-compatible providers
pub struct DynamicEmbeddingProvider {
    provider: OpenAiEmbeddingProvider<HttpClient>,
    model: String,
    dimensions: u32,
}

impl std::fmt::Debug for DynamicEmbeddingProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DynamicEmbeddingProvider")
            .field("model", &self.model)
            .field("dimensions", &self.dimensions)
            .finish()
    }
}

impl DynamicEmbeddingProvider {
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

    pub async fn embed_texts(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>, DomainError> {
        let request = EmbeddingRequest::batch(&self.model, texts);
        let response = self.provider.embed(request).await?;

        Ok(response
            .embeddings()
            .iter()
            .map(|e| e.vector().to_vec())
            .collect())
    }

}

/// Document ingestion service
pub struct IngestionService {
    provider_registry: Arc<dyn KnowledgeBaseProviderRegistryTrait>,
    embedding_config: Option<EmbeddingConfig>,
    embedding_provider: Option<Arc<dyn EmbeddingProvider>>,
}

impl std::fmt::Debug for IngestionService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IngestionService")
            .field("has_embedding_config", &self.embedding_config.is_some())
            .field("has_static_embedding_provider", &self.embedding_provider.is_some())
            .finish()
    }
}

impl IngestionService {
    pub fn new(provider_registry: Arc<dyn KnowledgeBaseProviderRegistryTrait>) -> Self {
        Self {
            provider_registry,
            embedding_config: None,
            embedding_provider: None,
        }
    }

    /// Create with an embedding configuration for dynamic provider creation
    pub fn with_embedding_config(
        provider_registry: Arc<dyn KnowledgeBaseProviderRegistryTrait>,
        embedding_config: EmbeddingConfig,
    ) -> Self {
        Self {
            provider_registry,
            embedding_config: Some(embedding_config),
            embedding_provider: None,
        }
    }

    /// Create with a static embedding provider for the new schema
    pub fn with_embedding_provider(
        provider_registry: Arc<dyn KnowledgeBaseProviderRegistryTrait>,
        embedding_provider: Arc<dyn EmbeddingProvider>,
    ) -> Self {
        Self {
            provider_registry,
            embedding_config: None,
            embedding_provider: Some(embedding_provider),
        }
    }

    /// Set the embedding provider
    pub fn set_embedding_provider(&mut self, provider: Arc<dyn EmbeddingProvider>) {
        self.embedding_provider = Some(provider);
    }

    /// Ingest a document into a knowledge base
    pub async fn ingest(
        &self,
        kb_id: &str,
        request: IngestDocumentRequest,
    ) -> Result<IngestionResult, DomainError> {
        // Get the provider for this knowledge base
        let provider = self.provider_registry.get_required(kb_id).await?;

        // Build parser input
        let mut parser_input = ParserInput::from_text(request.content.clone());

        if let Some(filename) = &request.filename {
            parser_input = parser_input.with_filename(filename);
        }

        for (key, value) in &request.metadata {
            parser_input = parser_input.with_metadata(key, value.clone());
        }

        // Determine parser type
        let parser_type = request.parser_type.unwrap_or_else(|| {
            request
                .filename
                .as_ref()
                .and_then(|f| ParserFactory::detect_from_filename(f))
                .unwrap_or(ParserType::PlainText)
        });

        // Build chunking config
        let chunking_config = ChunkingConfig::new(
            request.chunk_size.unwrap_or(1000),
            request.chunk_overlap.unwrap_or(200),
        );

        let chunking_type = request.chunking_type.unwrap_or(ChunkingType::FixedSize);

        // Parse the document
        let parser = ParserFactory::create(parser_type)?;
        let parsed = parser
            .parse(parser_input.clone())
            .await
            .map_err(|e| DomainError::validation(format!("Failed to parse document: {}", e)))?;

        // Chunk the document
        let chunker = ChunkerFactory::create(chunking_type);
        let chunks = chunker
            .chunk(&parsed.content, &chunking_config)
            .map_err(|e| DomainError::validation(format!("Failed to chunk document: {}", e)))?;

        // Generate document/source ID
        let source_id = request
            .source_id
            .clone()
            .or_else(|| request.filename.clone())
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        // Convert chunks to Document objects
        let total_chunks = chunks.len();
        let documents: Vec<Document> = chunks
            .into_iter()
            .enumerate()
            .map(|(idx, chunk)| {
                let mut metadata = request.metadata.clone();
                metadata.insert("chunk_index".to_string(), serde_json::json!(idx));
                metadata.insert("total_chunks".to_string(), serde_json::json!(total_chunks));
                metadata.insert(
                    "char_start".to_string(),
                    serde_json::json!(chunk.metadata.char_start),
                );
                metadata.insert(
                    "char_end".to_string(),
                    serde_json::json!(chunk.metadata.char_end),
                );

                // Add document metadata from parser
                if let Some(title) = &parsed.metadata.title {
                    metadata.insert("title".to_string(), serde_json::json!(title));
                }

                if let Some(author) = &parsed.metadata.author {
                    metadata.insert("author".to_string(), serde_json::json!(author));
                }

                let doc_id = format!("{}_{}", source_id, idx);

                Document::new(doc_id, chunk.content)
                    .with_all_metadata(metadata)
                    .with_source(&source_id)
            })
            .collect();

        // Add documents to the knowledge base via the provider
        let result = provider.add_documents(documents).await?;

        Ok(IngestionResult {
            document_id: source_id,
            chunks_created: result.added,
            chunks_failed: result.failed,
            errors: result
                .errors
                .into_iter()
                .map(|(id, msg)| crate::domain::ingestion::IngestionError::chunk(0, format!("{}: {}", id, msg)))
                .collect(),
        })
    }

    /// Get all documents for a knowledge base (grouped by source)
    pub async fn list_sources(&self, kb_id: &str) -> Result<Vec<SourceInfo>, DomainError> {
        let provider = self.provider_registry.get_required(kb_id).await?;
        provider.list_sources().await
    }

    /// Get documents by source ID
    pub async fn get_documents_by_source(
        &self,
        kb_id: &str,
        source: &str,
    ) -> Result<Vec<StoredDocument>, DomainError> {
        let provider = self.provider_registry.get_required(kb_id).await?;
        let results = provider.list_by_source(source).await?;

        Ok(results
            .into_iter()
            .map(|r| {
                let chunk_index = r
                    .metadata
                    .get("chunk_index")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as usize;

                let total_chunks = r
                    .metadata
                    .get("total_chunks")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(1) as usize;

                StoredDocument {
                    id: r.id,
                    content: r.content,
                    metadata: r.metadata,
                    source: r.source,
                    chunk_index,
                    total_chunks,
                }
            })
            .collect())
    }

    /// Get document count for a knowledge base
    pub async fn document_count(&self, kb_id: &str) -> Result<usize, DomainError> {
        let provider = self.provider_registry.get_required(kb_id).await?;
        provider.document_count().await
    }

    /// Delete a document by source ID
    pub async fn delete_by_source(&self, kb_id: &str, source: &str) -> Result<usize, DomainError> {
        let provider = self.provider_registry.get_required(kb_id).await?;
        let result = provider.delete_by_source(source).await?;
        Ok(result.deleted)
    }

    /// Initialize/ensure schema for a knowledge base (create tables, indexes)
    pub async fn ensure_schema(&self, kb_id: &str) -> Result<(), DomainError> {
        let provider = self.provider_registry.get_required(kb_id).await?;
        provider.ensure_schema().await
    }

    // ========================================================================
    // New schema methods (document/chunk separation)
    // ========================================================================

    /// Ingest a document using the new schema (with proper document/chunk separation)
    pub async fn ingest_document(
        &self,
        kb_id: &str,
        request: IngestDocumentV2Request,
    ) -> Result<KnowledgeBaseDocument, DomainError> {
        // Get the KB provider
        let provider = self.provider_registry.get_required(kb_id).await?;

        // Get or create embedding provider
        // Priority: static provider > dynamic from embedding_config
        let dynamic_provider: Option<DynamicEmbeddingProvider>;

        if self.embedding_provider.is_none() && self.embedding_config.is_some() {
            // Use dynamic embedding provider from KB config
            let config = self.embedding_config.as_ref().unwrap();
            let kb = config.get_kb(kb_id).await?;
            dynamic_provider = Some(config.create_embedding_provider(&kb).await?);
        } else {
            dynamic_provider = None;
        }

        // Validate we have some embedding provider
        if self.embedding_provider.is_none() && dynamic_provider.is_none() {
            return Err(DomainError::validation(
                "Embedding provider not configured. Configure embedding_config or set a static embedding provider.",
            ));
        }

        // Build parser input
        let mut parser_input = ParserInput::from_text(request.content.clone());

        if let Some(filename) = &request.filename {
            parser_input = parser_input.with_filename(filename);
        }

        for (key, value) in &request.metadata {
            parser_input = parser_input.with_metadata(key, value.clone());
        }

        // Determine parser type
        let parser_type = request.parser_type.unwrap_or_else(|| {
            request
                .filename
                .as_ref()
                .and_then(|f| ParserFactory::detect_from_filename(f))
                .unwrap_or(ParserType::PlainText)
        });

        // Build chunking config
        let chunking_config = ChunkingConfig::new(
            request.chunk_size.unwrap_or(1000),
            request.chunk_overlap.unwrap_or(200),
        );
        let chunking_type = request.chunking_type.unwrap_or(ChunkingType::FixedSize);

        // Parse the document
        let parser = ParserFactory::create(parser_type)?;
        let parsed = parser
            .parse(parser_input.clone())
            .await
            .map_err(|e| DomainError::validation(format!("Failed to parse document: {}", e)))?;

        // Chunk the document
        let chunker = ChunkerFactory::create(chunking_type);
        let chunks = chunker
            .chunk(&parsed.content, &chunking_config)
            .map_err(|e| DomainError::validation(format!("Failed to chunk document: {}", e)))?;

        // Generate embeddings for all chunks
        let chunk_contents: Vec<String> = chunks.iter().map(|c| c.content.clone()).collect();

        let embeddings: Vec<Vec<f32>> = if let Some(dp) = &dynamic_provider {
            // Use dynamic provider
            dp.embed_texts(chunk_contents)
                .await
                .map_err(|e| DomainError::validation(format!("Failed to generate embeddings: {}", e)))?
        } else if let Some(static_provider) = &self.embedding_provider {
            // Use static provider
            let embedding_request =
                EmbeddingRequest::batch(static_provider.default_model(), chunk_contents);

            let embedding_response = static_provider
                .embed(embedding_request)
                .await
                .map_err(|e| DomainError::validation(format!("Failed to generate embeddings: {}", e)))?;

            embedding_response
                .into_embeddings()
                .into_iter()
                .map(|e| e.into_vector())
                .collect()
        } else {
            // Should not reach here due to earlier validation
            return Err(DomainError::validation("No embedding provider available"));
        };

        if embeddings.len() != chunks.len() {
            return Err(DomainError::validation(format!(
                "Embedding count mismatch: expected {}, got {}",
                chunks.len(),
                embeddings.len()
            )));
        }

        // Build chunk requests
        let chunk_requests: Vec<CreateChunkRequest> = chunks
            .into_iter()
            .zip(embeddings.into_iter())
            .enumerate()
            .map(|(idx, (chunk, embedding))| {
                let mut chunk_metadata = request.metadata.clone();
                chunk_metadata.insert(
                    "char_start".to_string(),
                    serde_json::json!(chunk.metadata.char_start),
                );
                chunk_metadata.insert(
                    "char_end".to_string(),
                    serde_json::json!(chunk.metadata.char_end),
                );

                CreateChunkRequest {
                    content: chunk.content,
                    embedding,
                    chunk_index: idx as i32,
                    token_count: None, // Could calculate tokens if needed
                    metadata: chunk_metadata,
                }
            })
            .collect();

        // Build the document request
        let create_request = CreateDocumentRequest {
            title: request
                .title
                .or_else(|| parsed.metadata.title.clone())
                .or_else(|| request.filename.clone()),
            description: request.description,
            source_filename: request.filename,
            content_type: request.content_type,
            original_content: request.content,
            metadata: request.metadata,
            chunks: chunk_requests,
        };

        // Create the document via the provider
        provider.create_document(create_request).await
    }

    /// List all documents in a knowledge base (new schema)
    pub async fn list_documents_v2(
        &self,
        kb_id: &str,
    ) -> Result<Vec<DocumentSummary>, DomainError> {
        let provider = self.provider_registry.get_required(kb_id).await?;
        provider.list_documents().await
    }

    /// Get a document by ID (new schema)
    pub async fn get_document_v2(
        &self,
        kb_id: &str,
        document_id: Uuid,
    ) -> Result<Option<KnowledgeBaseDocument>, DomainError> {
        let provider = self.provider_registry.get_required(kb_id).await?;
        provider.get_document_by_id(document_id).await
    }

    /// Get chunks for a document (new schema)
    pub async fn get_document_chunks(
        &self,
        kb_id: &str,
        document_id: Uuid,
    ) -> Result<Vec<DocumentChunk>, DomainError> {
        let provider = self.provider_registry.get_required(kb_id).await?;
        provider.get_document_chunks(document_id).await
    }

    /// Delete a document and its chunks (new schema)
    pub async fn delete_document_v2(
        &self,
        kb_id: &str,
        document_id: Uuid,
    ) -> Result<bool, DomainError> {
        let provider = self.provider_registry.get_required(kb_id).await?;
        provider.delete_document_by_id(document_id).await
    }

    /// Disable a document (excludes from search) (new schema)
    pub async fn disable_document(
        &self,
        kb_id: &str,
        document_id: Uuid,
    ) -> Result<bool, DomainError> {
        let provider = self.provider_registry.get_required(kb_id).await?;
        provider.disable_document(document_id).await
    }

    /// Enable a previously disabled document (new schema)
    pub async fn enable_document(
        &self,
        kb_id: &str,
        document_id: Uuid,
    ) -> Result<bool, DomainError> {
        let provider = self.provider_registry.get_required(kb_id).await?;
        provider.enable_document(document_id).await
    }
}

/// Trait for ingestion service (for dependency injection)
#[async_trait::async_trait]
pub trait IngestionServiceTrait: Send + Sync + std::fmt::Debug {
    async fn ingest(
        &self,
        kb_id: &str,
        request: IngestDocumentRequest,
    ) -> Result<IngestionResult, DomainError>;

    async fn list_sources(&self, kb_id: &str) -> Result<Vec<SourceInfo>, DomainError>;

    async fn get_documents_by_source(
        &self,
        kb_id: &str,
        source: &str,
    ) -> Result<Vec<StoredDocument>, DomainError>;

    async fn document_count(&self, kb_id: &str) -> Result<usize, DomainError>;

    async fn delete_by_source(&self, kb_id: &str, source: &str) -> Result<usize, DomainError>;

    async fn ensure_schema(&self, kb_id: &str) -> Result<(), DomainError>;

    // New schema methods
    async fn ingest_document(
        &self,
        kb_id: &str,
        request: IngestDocumentV2Request,
    ) -> Result<KnowledgeBaseDocument, DomainError>;

    async fn list_documents_v2(&self, kb_id: &str) -> Result<Vec<DocumentSummary>, DomainError>;

    async fn get_document_v2(
        &self,
        kb_id: &str,
        document_id: Uuid,
    ) -> Result<Option<KnowledgeBaseDocument>, DomainError>;

    async fn get_document_chunks(
        &self,
        kb_id: &str,
        document_id: Uuid,
    ) -> Result<Vec<DocumentChunk>, DomainError>;

    async fn delete_document_v2(&self, kb_id: &str, document_id: Uuid) -> Result<bool, DomainError>;

    async fn disable_document(&self, kb_id: &str, document_id: Uuid) -> Result<bool, DomainError>;

    async fn enable_document(&self, kb_id: &str, document_id: Uuid) -> Result<bool, DomainError>;
}

#[async_trait::async_trait]
impl IngestionServiceTrait for IngestionService {
    async fn ingest(
        &self,
        kb_id: &str,
        request: IngestDocumentRequest,
    ) -> Result<IngestionResult, DomainError> {
        IngestionService::ingest(self, kb_id, request).await
    }

    async fn list_sources(&self, kb_id: &str) -> Result<Vec<SourceInfo>, DomainError> {
        IngestionService::list_sources(self, kb_id).await
    }

    async fn get_documents_by_source(
        &self,
        kb_id: &str,
        source: &str,
    ) -> Result<Vec<StoredDocument>, DomainError> {
        IngestionService::get_documents_by_source(self, kb_id, source).await
    }

    async fn document_count(&self, kb_id: &str) -> Result<usize, DomainError> {
        IngestionService::document_count(self, kb_id).await
    }

    async fn delete_by_source(&self, kb_id: &str, source: &str) -> Result<usize, DomainError> {
        IngestionService::delete_by_source(self, kb_id, source).await
    }

    async fn ensure_schema(&self, kb_id: &str) -> Result<(), DomainError> {
        IngestionService::ensure_schema(self, kb_id).await
    }

    async fn ingest_document(
        &self,
        kb_id: &str,
        request: IngestDocumentV2Request,
    ) -> Result<KnowledgeBaseDocument, DomainError> {
        IngestionService::ingest_document(self, kb_id, request).await
    }

    async fn list_documents_v2(&self, kb_id: &str) -> Result<Vec<DocumentSummary>, DomainError> {
        IngestionService::list_documents_v2(self, kb_id).await
    }

    async fn get_document_v2(
        &self,
        kb_id: &str,
        document_id: Uuid,
    ) -> Result<Option<KnowledgeBaseDocument>, DomainError> {
        IngestionService::get_document_v2(self, kb_id, document_id).await
    }

    async fn get_document_chunks(
        &self,
        kb_id: &str,
        document_id: Uuid,
    ) -> Result<Vec<DocumentChunk>, DomainError> {
        IngestionService::get_document_chunks(self, kb_id, document_id).await
    }

    async fn delete_document_v2(&self, kb_id: &str, document_id: Uuid) -> Result<bool, DomainError> {
        IngestionService::delete_document_v2(self, kb_id, document_id).await
    }

    async fn disable_document(&self, kb_id: &str, document_id: Uuid) -> Result<bool, DomainError> {
        IngestionService::disable_document(self, kb_id, document_id).await
    }

    async fn enable_document(&self, kb_id: &str, document_id: Uuid) -> Result<bool, DomainError> {
        IngestionService::enable_document(self, kb_id, document_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::knowledge_base::{KnowledgeBaseId, KnowledgeBaseProvider, MockKnowledgeBaseProvider};
    use crate::infrastructure::knowledge_base::KnowledgeBaseProviderRegistry;

    async fn create_service() -> IngestionService {
        let registry = Arc::new(KnowledgeBaseProviderRegistry::new());

        // Register a mock provider for testing
        let kb_id = KnowledgeBaseId::new("test-kb").unwrap();
        let provider: Arc<dyn KnowledgeBaseProvider> =
            Arc::new(MockKnowledgeBaseProvider::new(kb_id));
        registry.register(provider).await;

        IngestionService::new(registry)
    }

    #[tokio::test]
    async fn test_ingest_document() {
        let service = create_service().await;

        let request = IngestDocumentRequest::new("This is a test document with some content.")
            .with_filename("test.txt")
            .with_source_id("doc-1")
            .with_metadata("category", serde_json::json!("test"));

        let result = service.ingest("test-kb", request).await.unwrap();

        assert_eq!(result.document_id, "doc-1");
        assert!(result.chunks_created > 0);
        assert_eq!(result.chunks_failed, 0);
    }

    #[tokio::test]
    async fn test_list_sources() {
        let service = create_service().await;

        // Ingest a document first
        let request = IngestDocumentRequest::new("Test content").with_source_id("doc-1");
        service.ingest("test-kb", request).await.unwrap();

        let sources = service.list_sources("test-kb").await.unwrap();
        assert!(!sources.is_empty());
    }

    #[tokio::test]
    async fn test_document_count() {
        let service = create_service().await;

        let request = IngestDocumentRequest::new("Test content").with_source_id("doc-1");
        service.ingest("test-kb", request).await.unwrap();

        let count = service.document_count("test-kb").await.unwrap();
        assert!(count > 0);
    }

    #[tokio::test]
    async fn test_delete_by_source() {
        let service = create_service().await;

        let request = IngestDocumentRequest::new("Test content").with_source_id("doc-1");
        service.ingest("test-kb", request).await.unwrap();

        let deleted = service.delete_by_source("test-kb", "doc-1").await.unwrap();
        assert!(deleted > 0);

        let count = service.document_count("test-kb").await.unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_ingest_no_provider() {
        let registry = Arc::new(KnowledgeBaseProviderRegistry::new());
        let service = IngestionService::new(registry);

        let request = IngestDocumentRequest::new("Test content");
        let result = service.ingest("nonexistent-kb", request).await;

        assert!(result.is_err());
    }
}
