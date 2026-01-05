//! Document ingestion domain types and traits
//!
//! This module provides:
//! - `DocumentParser` trait for parsing various document formats
//! - `ChunkingStrategy` trait for splitting documents into chunks
//! - Configuration and result types for the ingestion pipeline

pub mod chunker;
pub mod parser;
pub mod pipeline;
pub mod validation;

// Re-export main types
pub use chunker::{Chunk, ChunkingConfig, ChunkingStrategy, ChunkMetadata};
pub use parser::{
    DocumentMetadata, DocumentParser, ParsedDocument, ParserContent, ParserInput,
};
pub use pipeline::{
    BatchIngestionResult, ChunkingType, IngestionConfig, IngestionError, IngestionResult,
    ParserType,
};
pub use validation::{
    detect_parser_from_filename, detect_parser_from_mime, validate_batch_size,
    validate_chunk_params, validate_document_id,
};

// Re-export mocks for testing
#[cfg(test)]
pub use chunker::mock::MockChunkingStrategy;
#[cfg(test)]
pub use parser::mock::MockDocumentParser;
