//! Factory for creating parsers and chunkers

use std::sync::Arc;

use crate::domain::ingestion::{
    ChunkingStrategy, ChunkingType, DocumentParser, ParserType,
};
use crate::domain::DomainError;

use super::chunkers::{FixedSizeChunker, ParagraphChunker, RecursiveChunker, SentenceChunker};
use super::parsers::{HtmlParser, JsonParser, MarkdownParser, PlainTextParser};

/// Factory for creating document parsers
#[derive(Debug, Default)]
pub struct ParserFactory;

impl ParserFactory {
    /// Create a parser for the given type
    pub fn create(parser_type: ParserType) -> Result<Arc<dyn DocumentParser>, DomainError> {
        match parser_type {
            ParserType::PlainText => Ok(Arc::new(PlainTextParser::new())),
            ParserType::Markdown => Ok(Arc::new(MarkdownParser::new())),
            ParserType::Html => Ok(Arc::new(HtmlParser::new())),
            ParserType::Json => Ok(Arc::new(JsonParser::new())),
            ParserType::Pdf => Err(DomainError::validation(
                "PDF parsing is not yet implemented",
            )),
        }
    }

    /// Detect parser type from filename extension
    pub fn detect_from_filename(filename: &str) -> Option<ParserType> {
        crate::domain::ingestion::detect_parser_from_filename(filename)
    }

    /// Detect parser type from MIME type
    pub fn detect_from_mime(mime: &str) -> Option<ParserType> {
        crate::domain::ingestion::detect_parser_from_mime(mime)
    }

    /// Get a list of all supported file extensions
    pub fn supported_extensions() -> Vec<&'static str> {
        vec!["txt", "text", "md", "markdown", "html", "htm", "json"]
    }

    /// Get a list of all supported MIME types
    pub fn supported_mime_types() -> Vec<&'static str> {
        vec![
            "text/plain",
            "text/markdown",
            "text/x-markdown",
            "text/html",
            "application/json",
        ]
    }
}

/// Factory for creating chunking strategies
#[derive(Debug, Default)]
pub struct ChunkerFactory;

impl ChunkerFactory {
    /// Create a chunker for the given type
    pub fn create(chunking_type: ChunkingType) -> Arc<dyn ChunkingStrategy> {
        match chunking_type {
            ChunkingType::FixedSize => Arc::new(FixedSizeChunker::new()),
            ChunkingType::Sentence => Arc::new(SentenceChunker::new()),
            ChunkingType::Paragraph => Arc::new(ParagraphChunker::new()),
            ChunkingType::Recursive => Arc::new(RecursiveChunker::new()),
        }
    }

    /// Get a list of all available chunking types
    pub fn available_types() -> Vec<ChunkingType> {
        vec![
            ChunkingType::FixedSize,
            ChunkingType::Sentence,
            ChunkingType::Paragraph,
            ChunkingType::Recursive,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parser_factory_plain_text() {
        let parser = ParserFactory::create(ParserType::PlainText).unwrap();
        assert!(parser.supports_file("test.txt"));
    }

    #[test]
    fn test_parser_factory_markdown() {
        let parser = ParserFactory::create(ParserType::Markdown).unwrap();
        assert!(parser.supports_file("test.md"));
    }

    #[test]
    fn test_parser_factory_html() {
        let parser = ParserFactory::create(ParserType::Html).unwrap();
        assert!(parser.supports_file("test.html"));
    }

    #[test]
    fn test_parser_factory_json() {
        let parser = ParserFactory::create(ParserType::Json).unwrap();
        assert!(parser.supports_file("test.json"));
    }

    #[test]
    fn test_parser_factory_pdf_not_implemented() {
        let result = ParserFactory::create(ParserType::Pdf);
        assert!(result.is_err());
    }

    #[test]
    fn test_parser_factory_detect_from_filename() {
        assert_eq!(
            ParserFactory::detect_from_filename("file.txt"),
            Some(ParserType::PlainText)
        );
        assert_eq!(
            ParserFactory::detect_from_filename("file.md"),
            Some(ParserType::Markdown)
        );
        assert_eq!(
            ParserFactory::detect_from_filename("file.html"),
            Some(ParserType::Html)
        );
        assert_eq!(
            ParserFactory::detect_from_filename("file.json"),
            Some(ParserType::Json)
        );
        assert_eq!(ParserFactory::detect_from_filename("file.xyz"), None);
    }

    #[test]
    fn test_parser_factory_detect_from_mime() {
        assert_eq!(
            ParserFactory::detect_from_mime("text/plain"),
            Some(ParserType::PlainText)
        );
        assert_eq!(
            ParserFactory::detect_from_mime("text/markdown"),
            Some(ParserType::Markdown)
        );
        assert_eq!(
            ParserFactory::detect_from_mime("text/html"),
            Some(ParserType::Html)
        );
        assert_eq!(
            ParserFactory::detect_from_mime("application/json"),
            Some(ParserType::Json)
        );
    }

    #[test]
    fn test_parser_factory_supported_extensions() {
        let extensions = ParserFactory::supported_extensions();
        assert!(extensions.contains(&"txt"));
        assert!(extensions.contains(&"md"));
        assert!(extensions.contains(&"html"));
        assert!(extensions.contains(&"json"));
    }

    #[test]
    fn test_chunker_factory_fixed_size() {
        let chunker = ChunkerFactory::create(ChunkingType::FixedSize);
        assert_eq!(chunker.name(), "fixed_size");
    }

    #[test]
    fn test_chunker_factory_sentence() {
        let chunker = ChunkerFactory::create(ChunkingType::Sentence);
        assert_eq!(chunker.name(), "sentence");
    }

    #[test]
    fn test_chunker_factory_paragraph() {
        let chunker = ChunkerFactory::create(ChunkingType::Paragraph);
        assert_eq!(chunker.name(), "paragraph");
    }

    #[test]
    fn test_chunker_factory_recursive() {
        let chunker = ChunkerFactory::create(ChunkingType::Recursive);
        assert_eq!(chunker.name(), "recursive");
    }

    #[test]
    fn test_chunker_factory_available_types() {
        let types = ChunkerFactory::available_types();
        assert_eq!(types.len(), 4);
        assert!(types.contains(&ChunkingType::FixedSize));
        assert!(types.contains(&ChunkingType::Sentence));
        assert!(types.contains(&ChunkingType::Paragraph));
        assert!(types.contains(&ChunkingType::Recursive));
    }
}
