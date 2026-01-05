//! Document ingestion infrastructure
//!
//! This module provides implementations for document parsing, chunking,
//! and the ingestion pipeline.

pub mod chunkers;
pub mod factory;
pub mod parsers;
pub mod pipeline;

// Re-export parsers
pub use parsers::{HtmlParser, JsonParser, MarkdownParser, PlainTextParser};

// Re-export chunkers
pub use chunkers::{FixedSizeChunker, ParagraphChunker, RecursiveChunker, SentenceChunker};

// Re-export factories
pub use factory::{ChunkerFactory, ParserFactory};

// Re-export pipeline
pub use pipeline::IngestionPipeline;
