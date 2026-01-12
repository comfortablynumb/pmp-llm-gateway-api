//! Knowledge base document and chunk entities

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// A document stored in a knowledge base
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeBaseDocument {
    id: Uuid,
    kb_id: String,
    title: Option<String>,
    description: Option<String>,
    source_filename: Option<String>,
    content_type: Option<String>,
    original_size_bytes: Option<i64>,
    chunk_count: i32,
    metadata: HashMap<String, serde_json::Value>,
    disabled: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl KnowledgeBaseDocument {
    /// Create a new document
    pub fn new(kb_id: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            kb_id: kb_id.into(),
            title: None,
            description: None,
            source_filename: None,
            content_type: None,
            original_size_bytes: None,
            chunk_count: 0,
            metadata: HashMap::new(),
            disabled: false,
            created_at: now,
            updated_at: now,
        }
    }

    /// Create with specific ID (for loading from DB)
    pub fn with_id(mut self, id: Uuid) -> Self {
        self.id = id;
        self
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_source_filename(mut self, filename: impl Into<String>) -> Self {
        self.source_filename = Some(filename.into());
        self
    }

    pub fn with_content_type(mut self, content_type: impl Into<String>) -> Self {
        self.content_type = Some(content_type.into());
        self
    }

    pub fn with_original_size(mut self, size: i64) -> Self {
        self.original_size_bytes = Some(size);
        self
    }

    pub fn with_chunk_count(mut self, count: i32) -> Self {
        self.chunk_count = count;
        self
    }

    pub fn with_metadata(mut self, metadata: HashMap<String, serde_json::Value>) -> Self {
        self.metadata = metadata;
        self
    }

    pub fn with_timestamps(mut self, created_at: DateTime<Utc>, updated_at: DateTime<Utc>) -> Self {
        self.created_at = created_at;
        self.updated_at = updated_at;
        self
    }

    pub fn with_disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    // Getters
    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn kb_id(&self) -> &str {
        &self.kb_id
    }

    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    pub fn source_filename(&self) -> Option<&str> {
        self.source_filename.as_deref()
    }

    pub fn content_type(&self) -> Option<&str> {
        self.content_type.as_deref()
    }

    pub fn original_size_bytes(&self) -> Option<i64> {
        self.original_size_bytes
    }

    pub fn chunk_count(&self) -> i32 {
        self.chunk_count
    }

    pub fn metadata(&self) -> &HashMap<String, serde_json::Value> {
        &self.metadata
    }

    pub fn is_disabled(&self) -> bool {
        self.disabled
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    /// Update the chunk count
    pub fn set_chunk_count(&mut self, count: i32) {
        self.chunk_count = count;
        self.updated_at = Utc::now();
    }

    /// Disable the document
    pub fn disable(&mut self) {
        self.disabled = true;
        self.updated_at = Utc::now();
    }

    /// Enable the document
    pub fn enable(&mut self) {
        self.disabled = false;
        self.updated_at = Utc::now();
    }
}

/// A chunk of a document stored with its embedding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentChunk {
    id: Uuid,
    document_id: Uuid,
    kb_id: String,
    chunk_index: i32,
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    embedding: Option<Vec<f32>>,
    token_count: Option<i32>,
    metadata: HashMap<String, serde_json::Value>,
    created_at: DateTime<Utc>,
}

impl DocumentChunk {
    /// Create a new chunk
    pub fn new(
        document_id: Uuid,
        kb_id: impl Into<String>,
        chunk_index: i32,
        content: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            document_id,
            kb_id: kb_id.into(),
            chunk_index,
            content: content.into(),
            embedding: None,
            token_count: None,
            metadata: HashMap::new(),
            created_at: Utc::now(),
        }
    }

    /// Create with specific ID (for loading from DB)
    pub fn with_id(mut self, id: Uuid) -> Self {
        self.id = id;
        self
    }

    pub fn with_embedding(mut self, embedding: Vec<f32>) -> Self {
        self.embedding = Some(embedding);
        self
    }

    pub fn with_token_count(mut self, count: i32) -> Self {
        self.token_count = Some(count);
        self
    }

    pub fn with_metadata(mut self, metadata: HashMap<String, serde_json::Value>) -> Self {
        self.metadata = metadata;
        self
    }

    pub fn with_created_at(mut self, created_at: DateTime<Utc>) -> Self {
        self.created_at = created_at;
        self
    }

    // Getters
    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn document_id(&self) -> Uuid {
        self.document_id
    }

    pub fn kb_id(&self) -> &str {
        &self.kb_id
    }

    pub fn chunk_index(&self) -> i32 {
        self.chunk_index
    }

    pub fn content(&self) -> &str {
        &self.content
    }

    pub fn embedding(&self) -> Option<&[f32]> {
        self.embedding.as_deref()
    }

    pub fn token_count(&self) -> Option<i32> {
        self.token_count
    }

    pub fn metadata(&self) -> &HashMap<String, serde_json::Value> {
        &self.metadata
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}

/// Request to create a new document with its chunks
#[derive(Debug, Clone)]
pub struct CreateDocumentRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub source_filename: Option<String>,
    pub content_type: Option<String>,
    pub original_content: String,
    pub metadata: HashMap<String, serde_json::Value>,
    pub chunks: Vec<CreateChunkRequest>,
}

/// Request to create a chunk
#[derive(Debug, Clone)]
pub struct CreateChunkRequest {
    pub content: String,
    pub embedding: Vec<f32>,
    pub chunk_index: i32,
    pub token_count: Option<i32>,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Summary of a document for listing
#[derive(Debug, Clone, Serialize)]
pub struct DocumentSummary {
    pub id: Uuid,
    pub title: Option<String>,
    pub source_filename: Option<String>,
    pub chunk_count: i32,
    pub disabled: bool,
    pub created_at: DateTime<Utc>,
}

impl From<&KnowledgeBaseDocument> for DocumentSummary {
    fn from(doc: &KnowledgeBaseDocument) -> Self {
        Self {
            id: doc.id,
            title: doc.title.clone(),
            source_filename: doc.source_filename.clone(),
            chunk_count: doc.chunk_count,
            disabled: doc.disabled,
            created_at: doc.created_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_document() {
        let doc = KnowledgeBaseDocument::new("test-kb")
            .with_title("Test Document")
            .with_source_filename("test.txt");

        assert_eq!(doc.kb_id(), "test-kb");
        assert_eq!(doc.title(), Some("Test Document"));
        assert_eq!(doc.source_filename(), Some("test.txt"));
        assert!(!doc.is_disabled());
        assert_eq!(doc.chunk_count(), 0);
    }

    #[test]
    fn test_create_chunk() {
        let doc_id = Uuid::new_v4();
        let chunk = DocumentChunk::new(doc_id, "test-kb", 0, "Test content")
            .with_embedding(vec![0.1, 0.2, 0.3]);

        assert_eq!(chunk.document_id(), doc_id);
        assert_eq!(chunk.kb_id(), "test-kb");
        assert_eq!(chunk.chunk_index(), 0);
        assert_eq!(chunk.content(), "Test content");
        assert!(chunk.embedding().is_some());
    }

    #[test]
    fn test_document_disable() {
        let mut doc = KnowledgeBaseDocument::new("test-kb");
        assert!(!doc.is_disabled());

        doc.disable();
        assert!(doc.is_disabled());

        doc.enable();
        assert!(!doc.is_disabled());
    }
}
