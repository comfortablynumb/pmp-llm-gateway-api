//! Ingestion pipeline types and configuration

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::chunker::ChunkingConfig;

/// Type of document parser to use
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ParserType {
    /// Plain text files
    PlainText,
    /// Markdown files
    Markdown,
    /// HTML files
    Html,
    /// JSON files (serializes entire content)
    Json,
    /// PDF files (not yet implemented)
    Pdf,
}

impl ParserType {
    /// Get file extensions associated with this parser type
    pub fn extensions(&self) -> &[&str] {
        match self {
            Self::PlainText => &["txt", "text"],
            Self::Markdown => &["md", "markdown"],
            Self::Html => &["html", "htm"],
            Self::Json => &["json"],
            Self::Pdf => &["pdf"],
        }
    }

    /// Get MIME types associated with this parser type
    pub fn mime_types(&self) -> &[&str] {
        match self {
            Self::PlainText => &["text/plain"],
            Self::Markdown => &["text/markdown", "text/x-markdown"],
            Self::Html => &["text/html"],
            Self::Json => &["application/json"],
            Self::Pdf => &["application/pdf"],
        }
    }
}

/// Type of chunking strategy to use
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ChunkingType {
    /// Fixed size chunks with character count
    #[default]
    FixedSize,
    /// Split by sentences
    Sentence,
    /// Split by paragraphs
    Paragraph,
    /// Recursive splitting (headers -> paragraphs -> sentences)
    Recursive,
}

/// Configuration for document ingestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestionConfig {
    /// Parser type (auto-detected from filename if None)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parser_type: Option<ParserType>,
    /// Chunking strategy
    #[serde(default)]
    pub chunking_type: ChunkingType,
    /// Chunking configuration
    #[serde(flatten)]
    pub chunking_config: ChunkingConfig,
    /// Batch size for embedding generation
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,
    /// Custom metadata to add to all chunks
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
    /// Source identifier for the document
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
}

fn default_batch_size() -> usize {
    100
}

impl Default for IngestionConfig {
    fn default() -> Self {
        Self {
            parser_type: None,
            chunking_type: ChunkingType::default(),
            chunking_config: ChunkingConfig::default(),
            batch_size: default_batch_size(),
            metadata: HashMap::new(),
            source_id: None,
        }
    }
}

impl IngestionConfig {
    /// Create a new ingestion configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the parser type
    pub fn with_parser_type(mut self, parser_type: ParserType) -> Self {
        self.parser_type = Some(parser_type);
        self
    }

    /// Set the chunking type
    pub fn with_chunking_type(mut self, chunking_type: ChunkingType) -> Self {
        self.chunking_type = chunking_type;
        self
    }

    /// Set chunk size
    pub fn with_chunk_size(mut self, size: usize) -> Self {
        self.chunking_config.chunk_size = size;
        self
    }

    /// Set chunk overlap
    pub fn with_chunk_overlap(mut self, overlap: usize) -> Self {
        self.chunking_config.chunk_overlap = overlap;
        self
    }

    /// Set batch size
    pub fn with_batch_size(mut self, batch_size: usize) -> Self {
        self.batch_size = batch_size;
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    /// Set source ID
    pub fn with_source_id(mut self, source_id: impl Into<String>) -> Self {
        self.source_id = Some(source_id.into());
        self
    }
}

/// Error that occurred during ingestion of a specific chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestionError {
    /// Chunk index where the error occurred (None if document-level)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunk_index: Option<usize>,
    /// Error message
    pub message: String,
}

impl IngestionError {
    /// Create a document-level error
    pub fn document(message: impl Into<String>) -> Self {
        Self {
            chunk_index: None,
            message: message.into(),
        }
    }

    /// Create a chunk-level error
    pub fn chunk(index: usize, message: impl Into<String>) -> Self {
        Self {
            chunk_index: Some(index),
            message: message.into(),
        }
    }
}

/// Result of ingesting a single document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestionResult {
    /// Document ID assigned during ingestion
    pub document_id: String,
    /// Number of chunks successfully created
    pub chunks_created: usize,
    /// Number of chunks that failed
    pub chunks_failed: usize,
    /// Errors that occurred during ingestion
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<IngestionError>,
}

impl IngestionResult {
    /// Create a successful result
    pub fn success(document_id: impl Into<String>, chunks_created: usize) -> Self {
        Self {
            document_id: document_id.into(),
            chunks_created,
            chunks_failed: 0,
            errors: Vec::new(),
        }
    }

    /// Create a failed result
    pub fn failed(document_id: impl Into<String>, error: IngestionError) -> Self {
        Self {
            document_id: document_id.into(),
            chunks_created: 0,
            chunks_failed: 0,
            errors: vec![error],
        }
    }

    /// Check if the ingestion was fully successful
    pub fn is_success(&self) -> bool {
        self.errors.is_empty() && self.chunks_failed == 0
    }

    /// Check if the ingestion had any failures
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty() || self.chunks_failed > 0
    }

    /// Add an error
    pub fn add_error(&mut self, error: IngestionError) {
        self.errors.push(error);
    }
}

/// Result of batch ingestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchIngestionResult {
    /// Total documents processed
    pub total_documents: usize,
    /// Number of successful documents
    pub successful: usize,
    /// Number of failed documents
    pub failed: usize,
    /// Individual results for each document
    pub results: Vec<IngestionResult>,
}

impl BatchIngestionResult {
    /// Create an empty batch result
    pub fn new() -> Self {
        Self {
            total_documents: 0,
            successful: 0,
            failed: 0,
            results: Vec::new(),
        }
    }

    /// Add a result to the batch
    pub fn add(&mut self, result: IngestionResult) {
        self.total_documents += 1;

        if result.is_success() {
            self.successful += 1;
        } else {
            self.failed += 1;
        }

        self.results.push(result);
    }

    /// Check if all documents were processed successfully
    pub fn is_success(&self) -> bool {
        self.failed == 0
    }

    /// Get total chunks created across all documents
    pub fn total_chunks_created(&self) -> usize {
        self.results.iter().map(|r| r.chunks_created).sum()
    }
}

impl Default for BatchIngestionResult {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parser_type_extensions() {
        assert_eq!(ParserType::PlainText.extensions(), &["txt", "text"]);
        assert_eq!(ParserType::Markdown.extensions(), &["md", "markdown"]);
        assert_eq!(ParserType::Html.extensions(), &["html", "htm"]);
        assert_eq!(ParserType::Json.extensions(), &["json"]);
    }

    #[test]
    fn test_ingestion_config_builder() {
        let config = IngestionConfig::new()
            .with_parser_type(ParserType::Markdown)
            .with_chunking_type(ChunkingType::Sentence)
            .with_chunk_size(500)
            .with_chunk_overlap(50)
            .with_batch_size(50)
            .with_source_id("test-doc");

        assert_eq!(config.parser_type, Some(ParserType::Markdown));
        assert_eq!(config.chunking_type, ChunkingType::Sentence);
        assert_eq!(config.chunking_config.chunk_size, 500);
        assert_eq!(config.chunking_config.chunk_overlap, 50);
        assert_eq!(config.batch_size, 50);
        assert_eq!(config.source_id, Some("test-doc".to_string()));
    }

    #[test]
    fn test_ingestion_result_success() {
        let result = IngestionResult::success("doc-1", 5);
        assert!(result.is_success());
        assert!(!result.has_errors());
        assert_eq!(result.chunks_created, 5);
    }

    #[test]
    fn test_ingestion_result_failed() {
        let result = IngestionResult::failed("doc-1", IngestionError::document("parse failed"));
        assert!(!result.is_success());
        assert!(result.has_errors());
    }

    #[test]
    fn test_batch_ingestion_result() {
        let mut batch = BatchIngestionResult::new();
        batch.add(IngestionResult::success("doc-1", 5));
        batch.add(IngestionResult::success("doc-2", 3));
        batch.add(IngestionResult::failed(
            "doc-3",
            IngestionError::document("error"),
        ));

        assert_eq!(batch.total_documents, 3);
        assert_eq!(batch.successful, 2);
        assert_eq!(batch.failed, 1);
        assert_eq!(batch.total_chunks_created(), 8);
        assert!(!batch.is_success());
    }

    #[test]
    fn test_ingestion_error() {
        let doc_error = IngestionError::document("parse failed");
        assert!(doc_error.chunk_index.is_none());

        let chunk_error = IngestionError::chunk(5, "embedding failed");
        assert_eq!(chunk_error.chunk_index, Some(5));
    }

    #[test]
    fn test_config_serialization() {
        let config = IngestionConfig::new()
            .with_parser_type(ParserType::Markdown)
            .with_chunk_size(500);

        let json = serde_json::to_string(&config).unwrap();
        let parsed: IngestionConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.parser_type, Some(ParserType::Markdown));
        assert_eq!(parsed.chunking_config.chunk_size, 500);
    }
}
