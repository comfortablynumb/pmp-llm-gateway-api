//! Ingestion pipeline service

use std::collections::HashMap;
use std::sync::Arc;

use crate::domain::ingestion::{
    detect_parser_from_filename, BatchIngestionResult, Chunk, ChunkingConfig, ChunkingStrategy,
    ChunkingType, DocumentParser, IngestionConfig, IngestionError, IngestionResult, ParserInput,
    ParserType,
};
use crate::domain::knowledge_base::{Document, KnowledgeBaseProvider};
use crate::domain::DomainError;

use super::chunkers::FixedSizeChunker;
use super::parsers::PlainTextParser;

/// Ingestion pipeline for processing documents into knowledge bases
#[derive(Debug)]
pub struct IngestionPipeline<K>
where
    K: KnowledgeBaseProvider,
{
    knowledge_base: Arc<K>,
}

impl<K: KnowledgeBaseProvider> IngestionPipeline<K> {
    /// Create a new ingestion pipeline
    pub fn new(knowledge_base: Arc<K>) -> Self {
        Self { knowledge_base }
    }

    /// Ingest a single document
    pub async fn ingest(
        &self,
        input: ParserInput,
        config: &IngestionConfig,
    ) -> Result<IngestionResult, DomainError> {
        let document_id = self.generate_document_id(&input, config);

        let parser = self.get_parser(&input, config)?;
        let parsed = match parser.parse(input).await {
            Ok(p) => p,
            Err(e) => {
                return Ok(IngestionResult::failed(
                    document_id,
                    IngestionError::document(e.to_string()),
                ));
            }
        };

        let chunker = self.get_chunker(config);
        let chunking_config = ChunkingConfig {
            chunk_size: config.chunking_config.chunk_size,
            chunk_overlap: config.chunking_config.chunk_overlap,
            min_chunk_size: config.chunking_config.min_chunk_size,
        };

        let chunks = match chunker.chunk(&parsed.content, &chunking_config) {
            Ok(c) => c,
            Err(e) => {
                return Ok(IngestionResult::failed(
                    document_id,
                    IngestionError::document(format!("Chunking failed: {}", e)),
                ));
            }
        };

        if chunks.is_empty() {
            return Ok(IngestionResult::success(&document_id, 0));
        }

        let documents = self.create_documents(
            &document_id,
            &chunks,
            &parsed.metadata.to_json_map(),
            &config.metadata,
        );

        let add_result = self.knowledge_base.add_documents(documents).await?;

        let mut result = IngestionResult::success(&document_id, add_result.added);
        result.chunks_failed = add_result.failed;

        for (chunk_id, error) in add_result.errors {
            let chunk_index = self.extract_chunk_index(&chunk_id);
            result.add_error(IngestionError::chunk(chunk_index.unwrap_or(0), error));
        }

        Ok(result)
    }

    /// Ingest multiple documents in batch
    pub async fn ingest_batch(
        &self,
        inputs: Vec<ParserInput>,
        config: &IngestionConfig,
    ) -> Result<BatchIngestionResult, DomainError> {
        let mut batch_result = BatchIngestionResult::new();

        for input in inputs {
            let result = self.ingest(input, config).await?;
            batch_result.add(result);
        }

        Ok(batch_result)
    }

    /// Update an existing document by deleting old chunks and re-ingesting
    pub async fn update_document(
        &self,
        document_id: &str,
        input: ParserInput,
        config: &IngestionConfig,
    ) -> Result<IngestionResult, DomainError> {
        self.delete_document(document_id).await?;

        let config_with_id = IngestionConfig {
            source_id: Some(document_id.to_string()),
            ..config.clone()
        };

        self.ingest(input, &config_with_id).await
    }

    /// Delete all chunks for a document
    pub async fn delete_document(&self, document_id: &str) -> Result<usize, DomainError> {
        use crate::domain::knowledge_base::{FilterBuilder, FilterValue};

        let filter = FilterBuilder::new()
            .eq("document_id", FilterValue::String(document_id.to_string()))
            .build()
            .ok_or_else(|| DomainError::validation("Failed to build filter"))?;

        let result = self.knowledge_base.delete_by_filter(filter).await?;
        Ok(result.deleted)
    }

    fn generate_document_id(&self, input: &ParserInput, config: &IngestionConfig) -> String {
        if let Some(ref id) = config.source_id {
            return id.clone();
        }

        if let Some(ref filename) = input.filename {
            return filename.clone();
        }

        uuid::Uuid::new_v4().to_string()
    }

    fn get_parser(
        &self,
        input: &ParserInput,
        config: &IngestionConfig,
    ) -> Result<Box<dyn DocumentParser>, DomainError> {
        let parser_type = config
            .parser_type
            .clone()
            .or_else(|| input.filename.as_ref().and_then(|f| detect_parser_from_filename(f)))
            .unwrap_or(ParserType::PlainText);

        match parser_type {
            ParserType::PlainText => Ok(Box::new(PlainTextParser::new())),
            ParserType::Markdown => Ok(Box::new(super::parsers::MarkdownParser::new())),
            ParserType::Html => Ok(Box::new(super::parsers::HtmlParser::new())),
            ParserType::Json => Ok(Box::new(super::parsers::JsonParser::new())),
            ParserType::Pdf => Err(DomainError::validation(
                "PDF parsing is not yet implemented",
            )),
        }
    }

    fn get_chunker(&self, config: &IngestionConfig) -> Box<dyn ChunkingStrategy> {
        match config.chunking_type {
            ChunkingType::FixedSize => Box::new(FixedSizeChunker::new()),
            ChunkingType::Sentence => Box::new(super::chunkers::SentenceChunker::new()),
            ChunkingType::Paragraph => Box::new(super::chunkers::ParagraphChunker::new()),
            ChunkingType::Recursive => Box::new(super::chunkers::RecursiveChunker::new()),
        }
    }

    fn create_documents(
        &self,
        document_id: &str,
        chunks: &[Chunk],
        doc_metadata: &HashMap<String, serde_json::Value>,
        custom_metadata: &HashMap<String, serde_json::Value>,
    ) -> Vec<Document> {
        chunks
            .iter()
            .map(|chunk| {
                let chunk_id = format!("{}_chunk_{}", document_id, chunk.metadata.chunk_index);

                let mut metadata = doc_metadata.clone();

                for (key, value) in &chunk.metadata.to_json_map() {
                    metadata.insert(key.clone(), value.clone());
                }

                for (key, value) in custom_metadata {
                    metadata.insert(key.clone(), value.clone());
                }

                metadata.insert(
                    "document_id".to_string(),
                    serde_json::Value::String(document_id.to_string()),
                );

                Document {
                    id: chunk_id,
                    content: chunk.content.clone(),
                    metadata,
                    source: Some(document_id.to_string()),
                }
            })
            .collect()
    }

    fn extract_chunk_index(&self, chunk_id: &str) -> Option<usize> {
        chunk_id
            .rsplit("_chunk_")
            .next()
            .and_then(|s| s.parse().ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::knowledge_base::{KnowledgeBaseId, MockKnowledgeBaseProvider};

    fn create_mock_kb() -> Arc<MockKnowledgeBaseProvider> {
        let id = KnowledgeBaseId::new("test-kb").unwrap();
        Arc::new(MockKnowledgeBaseProvider::new(id))
    }

    #[tokio::test]
    async fn test_ingest_simple_document() {
        let kb = create_mock_kb();
        let pipeline = IngestionPipeline::new(kb);

        let input = ParserInput::from_text("Hello, World!");
        let config = IngestionConfig::new();

        let result = pipeline.ingest(input, &config).await.unwrap();

        assert!(result.is_success());
        assert_eq!(result.chunks_created, 1);
    }

    #[tokio::test]
    async fn test_ingest_with_source_id() {
        let kb = create_mock_kb();
        let pipeline = IngestionPipeline::new(kb);

        let input = ParserInput::from_text("Test content");
        let config = IngestionConfig::new().with_source_id("my-doc-123");

        let result = pipeline.ingest(input, &config).await.unwrap();

        assert_eq!(result.document_id, "my-doc-123");
    }

    #[tokio::test]
    async fn test_ingest_with_filename() {
        let kb = create_mock_kb();
        let pipeline = IngestionPipeline::new(kb);

        let input = ParserInput::from_text("Test content").with_filename("document.txt");
        let config = IngestionConfig::new();

        let result = pipeline.ingest(input, &config).await.unwrap();

        assert_eq!(result.document_id, "document.txt");
    }

    #[tokio::test]
    async fn test_ingest_empty_document() {
        let kb = create_mock_kb();
        let pipeline = IngestionPipeline::new(kb);

        let input = ParserInput::from_text("");
        let config = IngestionConfig::new();

        let result = pipeline.ingest(input, &config).await.unwrap();

        assert!(result.is_success());
        assert_eq!(result.chunks_created, 0);
    }

    #[tokio::test]
    async fn test_ingest_large_document() {
        let kb = create_mock_kb();
        let pipeline = IngestionPipeline::new(kb);

        let content = "This is a test sentence. ".repeat(100);
        let input = ParserInput::from_text(content);
        let config = IngestionConfig::new()
            .with_chunk_size(100)
            .with_chunk_overlap(20);

        let result = pipeline.ingest(input, &config).await.unwrap();

        assert!(result.is_success());
        assert!(result.chunks_created > 1);
    }

    #[tokio::test]
    async fn test_ingest_batch() {
        let kb = create_mock_kb();
        let pipeline = IngestionPipeline::new(kb);

        let inputs = vec![
            ParserInput::from_text("Document 1").with_filename("doc1.txt"),
            ParserInput::from_text("Document 2").with_filename("doc2.txt"),
            ParserInput::from_text("Document 3").with_filename("doc3.txt"),
        ];
        let config = IngestionConfig::new();

        let result = pipeline.ingest_batch(inputs, &config).await.unwrap();

        assert_eq!(result.total_documents, 3);
        assert_eq!(result.successful, 3);
        assert_eq!(result.failed, 0);
    }

    #[tokio::test]
    async fn test_ingest_with_custom_metadata() {
        let kb = create_mock_kb();
        let pipeline = IngestionPipeline::new(kb);

        let input = ParserInput::from_text("Test content");
        let config = IngestionConfig::new()
            .with_metadata("category", serde_json::Value::String("test".to_string()))
            .with_metadata("version", serde_json::Value::Number(1.into()));

        let result = pipeline.ingest(input, &config).await.unwrap();

        assert!(result.is_success());
    }

    #[test]
    fn test_extract_chunk_index() {
        let kb = create_mock_kb();
        let pipeline = IngestionPipeline::new(kb);

        assert_eq!(
            pipeline.extract_chunk_index("doc123_chunk_5"),
            Some(5)
        );
        assert_eq!(
            pipeline.extract_chunk_index("doc_chunk_0"),
            Some(0)
        );
        assert_eq!(pipeline.extract_chunk_index("invalid"), None);
    }

    #[test]
    fn test_generate_document_id() {
        let kb = create_mock_kb();
        let pipeline = IngestionPipeline::new(kb);

        let input = ParserInput::from_text("test");

        let config_with_source = IngestionConfig::new().with_source_id("explicit-id");
        assert_eq!(
            pipeline.generate_document_id(&input, &config_with_source),
            "explicit-id"
        );

        let input_with_filename = ParserInput::from_text("test").with_filename("file.txt");
        let config = IngestionConfig::new();
        assert_eq!(
            pipeline.generate_document_id(&input_with_filename, &config),
            "file.txt"
        );
    }
}
