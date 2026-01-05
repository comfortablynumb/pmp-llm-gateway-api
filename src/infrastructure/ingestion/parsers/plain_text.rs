//! Plain text document parser

use async_trait::async_trait;

use crate::domain::ingestion::{
    DocumentMetadata, DocumentParser, ParsedDocument, ParserInput,
};
use crate::domain::DomainError;

/// Parser for plain text files
#[derive(Debug, Clone, Default)]
pub struct PlainTextParser;

impl PlainTextParser {
    /// Create a new plain text parser
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl DocumentParser for PlainTextParser {
    fn supported_extensions(&self) -> &[&str] {
        &["txt", "text"]
    }

    fn supported_mime_types(&self) -> &[&str] {
        &["text/plain"]
    }

    async fn parse(&self, input: ParserInput) -> Result<ParsedDocument, DomainError> {
        let content = input.content.as_text()?;

        let mut metadata = DocumentMetadata::new().with_mime_type("text/plain");

        if let Some(ref filename) = input.filename {
            metadata = metadata.with_source(filename.clone());
        }

        for (key, value) in input.metadata {
            metadata = metadata.with_custom(key, value);
        }

        Ok(ParsedDocument::new(content, metadata))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ingestion::ParserContent;

    #[tokio::test]
    async fn test_parse_text_content() {
        let parser = PlainTextParser::new();
        let input = ParserInput::from_text("Hello, World!");

        let result = parser.parse(input).await.unwrap();

        assert_eq!(result.content, "Hello, World!");
        assert_eq!(result.metadata.mime_type, Some("text/plain".to_string()));
    }

    #[tokio::test]
    async fn test_parse_bytes_content() {
        let parser = PlainTextParser::new();
        let input = ParserInput::from_bytes(b"Hello from bytes".to_vec());

        let result = parser.parse(input).await.unwrap();

        assert_eq!(result.content, "Hello from bytes");
    }

    #[tokio::test]
    async fn test_parse_with_filename() {
        let parser = PlainTextParser::new();
        let input = ParserInput::from_text("content").with_filename("test.txt");

        let result = parser.parse(input).await.unwrap();

        assert_eq!(result.metadata.source, Some("test.txt".to_string()));
    }

    #[tokio::test]
    async fn test_parse_with_metadata() {
        let parser = PlainTextParser::new();
        let input = ParserInput::from_text("content")
            .with_metadata("custom_key", serde_json::Value::String("custom_value".to_string()));

        let result = parser.parse(input).await.unwrap();

        assert_eq!(
            result.metadata.custom.get("custom_key"),
            Some(&serde_json::Value::String("custom_value".to_string()))
        );
    }

    #[tokio::test]
    async fn test_parse_invalid_utf8() {
        let parser = PlainTextParser::new();
        let input = ParserInput {
            content: ParserContent::Bytes(vec![0xff, 0xfe]),
            filename: None,
            metadata: std::collections::HashMap::new(),
        };

        let result = parser.parse(input).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_supported_extensions() {
        let parser = PlainTextParser::new();
        assert_eq!(parser.supported_extensions(), &["txt", "text"]);
    }

    #[test]
    fn test_supported_mime_types() {
        let parser = PlainTextParser::new();
        assert_eq!(parser.supported_mime_types(), &["text/plain"]);
    }

    #[test]
    fn test_supports_file() {
        let parser = PlainTextParser::new();
        assert!(parser.supports_file("document.txt"));
        assert!(parser.supports_file("document.TXT"));
        assert!(parser.supports_file("document.text"));
        assert!(!parser.supports_file("document.md"));
    }

    #[test]
    fn test_supports_mime() {
        let parser = PlainTextParser::new();
        assert!(parser.supports_mime("text/plain"));
        assert!(parser.supports_mime("text/plain; charset=utf-8"));
        assert!(!parser.supports_mime("text/html"));
    }
}
