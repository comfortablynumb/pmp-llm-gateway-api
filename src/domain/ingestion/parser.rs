//! Document parser trait and types

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Debug;

use crate::domain::DomainError;

/// Content input for document parsing
#[derive(Debug, Clone)]
pub enum ParserContent {
    /// Text content (already decoded)
    Text(String),
    /// Binary content (needs decoding)
    Bytes(Vec<u8>),
}

impl ParserContent {
    /// Create text content
    pub fn text(content: impl Into<String>) -> Self {
        Self::Text(content.into())
    }

    /// Create binary content
    pub fn bytes(content: impl Into<Vec<u8>>) -> Self {
        Self::Bytes(content.into())
    }

    /// Get content as text, decoding bytes as UTF-8 if necessary
    pub fn as_text(&self) -> Result<String, DomainError> {
        match self {
            Self::Text(s) => Ok(s.clone()),
            Self::Bytes(b) => String::from_utf8(b.clone())
                .map_err(|e| DomainError::validation(format!("Invalid UTF-8: {}", e))),
        }
    }
}

/// Input for document parsing
#[derive(Debug, Clone)]
pub struct ParserInput {
    /// Document content (text or bytes)
    pub content: ParserContent,
    /// Optional filename for type detection
    pub filename: Option<String>,
    /// Additional metadata to include
    pub metadata: HashMap<String, serde_json::Value>,
}

impl ParserInput {
    /// Create input from text content
    pub fn from_text(content: impl Into<String>) -> Self {
        Self {
            content: ParserContent::text(content),
            filename: None,
            metadata: HashMap::new(),
        }
    }

    /// Create input from binary content
    pub fn from_bytes(content: impl Into<Vec<u8>>) -> Self {
        Self {
            content: ParserContent::bytes(content),
            filename: None,
            metadata: HashMap::new(),
        }
    }

    /// Set the filename
    pub fn with_filename(mut self, filename: impl Into<String>) -> Self {
        self.filename = Some(filename.into());
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

/// Metadata extracted from a document
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DocumentMetadata {
    /// Document title
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Document author
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    /// Creation timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<Utc>>,
    /// Last modified timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified_at: Option<DateTime<Utc>>,
    /// Source file or URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// MIME type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    /// Custom metadata fields
    #[serde(flatten)]
    pub custom: HashMap<String, serde_json::Value>,
}

impl DocumentMetadata {
    /// Create empty metadata
    pub fn new() -> Self {
        Self::default()
    }

    /// Set title
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set author
    pub fn with_author(mut self, author: impl Into<String>) -> Self {
        self.author = Some(author.into());
        self
    }

    /// Set source
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    /// Set MIME type
    pub fn with_mime_type(mut self, mime_type: impl Into<String>) -> Self {
        self.mime_type = Some(mime_type.into());
        self
    }

    /// Add custom metadata
    pub fn with_custom(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.custom.insert(key.into(), value);
        self
    }

    /// Merge with another metadata, preferring self's values
    pub fn merge(mut self, other: DocumentMetadata) -> Self {
        if self.title.is_none() {
            self.title = other.title;
        }

        if self.author.is_none() {
            self.author = other.author;
        }

        if self.created_at.is_none() {
            self.created_at = other.created_at;
        }

        if self.modified_at.is_none() {
            self.modified_at = other.modified_at;
        }

        if self.source.is_none() {
            self.source = other.source;
        }

        if self.mime_type.is_none() {
            self.mime_type = other.mime_type;
        }

        for (key, value) in other.custom {
            self.custom.entry(key).or_insert(value);
        }

        self
    }

    /// Convert to JSON value map
    pub fn to_json_map(&self) -> HashMap<String, serde_json::Value> {
        let mut map = HashMap::new();

        if let Some(ref title) = self.title {
            map.insert("title".to_string(), serde_json::Value::String(title.clone()));
        }

        if let Some(ref author) = self.author {
            map.insert("author".to_string(), serde_json::Value::String(author.clone()));
        }

        if let Some(ref created_at) = self.created_at {
            map.insert(
                "created_at".to_string(),
                serde_json::Value::String(created_at.to_rfc3339()),
            );
        }

        if let Some(ref modified_at) = self.modified_at {
            map.insert(
                "modified_at".to_string(),
                serde_json::Value::String(modified_at.to_rfc3339()),
            );
        }

        if let Some(ref source) = self.source {
            map.insert("source".to_string(), serde_json::Value::String(source.clone()));
        }

        if let Some(ref mime_type) = self.mime_type {
            map.insert("mime_type".to_string(), serde_json::Value::String(mime_type.clone()));
        }

        for (key, value) in &self.custom {
            map.insert(key.clone(), value.clone());
        }

        map
    }
}

/// Result of parsing a document
#[derive(Debug, Clone)]
pub struct ParsedDocument {
    /// Extracted text content
    pub content: String,
    /// Extracted metadata
    pub metadata: DocumentMetadata,
}

impl ParsedDocument {
    /// Create a parsed document
    pub fn new(content: impl Into<String>, metadata: DocumentMetadata) -> Self {
        Self {
            content: content.into(),
            metadata,
        }
    }

    /// Create a simple parsed document with just content
    pub fn from_content(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            metadata: DocumentMetadata::new(),
        }
    }
}

/// Trait for document parsers
#[async_trait]
pub trait DocumentParser: Send + Sync + Debug {
    /// Get supported file extensions (e.g., ["txt", "text"])
    fn supported_extensions(&self) -> &[&str];

    /// Get supported MIME types (e.g., ["text/plain"])
    fn supported_mime_types(&self) -> &[&str];

    /// Parse a document and extract text content and metadata
    async fn parse(&self, input: ParserInput) -> Result<ParsedDocument, DomainError>;

    /// Check if this parser supports a given filename
    fn supports_file(&self, filename: &str) -> bool {
        let ext = filename
            .rsplit('.')
            .next()
            .map(|s| s.to_lowercase())
            .unwrap_or_default();

        self.supported_extensions()
            .iter()
            .any(|e| e.eq_ignore_ascii_case(&ext))
    }

    /// Check if this parser supports a given MIME type
    fn supports_mime(&self, mime: &str) -> bool {
        self.supported_mime_types()
            .iter()
            .any(|m| mime.starts_with(*m))
    }
}

#[cfg(test)]
pub mod mock {
    use super::*;
    use std::sync::Mutex;

    /// Mock document parser for testing
    #[derive(Debug)]
    pub struct MockDocumentParser {
        extensions: Vec<&'static str>,
        mime_types: Vec<&'static str>,
        result: Mutex<Option<Result<ParsedDocument, String>>>,
    }

    impl MockDocumentParser {
        pub fn new() -> Self {
            Self {
                extensions: vec!["txt"],
                mime_types: vec!["text/plain"],
                result: Mutex::new(None),
            }
        }

        pub fn with_extensions(mut self, extensions: Vec<&'static str>) -> Self {
            self.extensions = extensions;
            self
        }

        pub fn with_mime_types(mut self, mime_types: Vec<&'static str>) -> Self {
            self.mime_types = mime_types;
            self
        }

        pub fn with_result(self, result: ParsedDocument) -> Self {
            *self.result.lock().unwrap() = Some(Ok(result));
            self
        }

        pub fn with_error(self, error: impl Into<String>) -> Self {
            *self.result.lock().unwrap() = Some(Err(error.into()));
            self
        }
    }

    impl Default for MockDocumentParser {
        fn default() -> Self {
            Self::new()
        }
    }

    #[async_trait]
    impl DocumentParser for MockDocumentParser {
        fn supported_extensions(&self) -> &[&str] {
            &self.extensions
        }

        fn supported_mime_types(&self) -> &[&str] {
            &self.mime_types
        }

        async fn parse(&self, input: ParserInput) -> Result<ParsedDocument, DomainError> {
            if let Some(result) = self.result.lock().unwrap().take() {
                return result.map_err(DomainError::validation);
            }

            let content = input.content.as_text()?;
            Ok(ParsedDocument::new(content, DocumentMetadata::new()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parser_content_text() {
        let content = ParserContent::text("hello world");
        assert_eq!(content.as_text().unwrap(), "hello world");
    }

    #[test]
    fn test_parser_content_bytes() {
        let content = ParserContent::bytes(b"hello world".to_vec());
        assert_eq!(content.as_text().unwrap(), "hello world");
    }

    #[test]
    fn test_parser_content_invalid_utf8() {
        let content = ParserContent::bytes(vec![0xff, 0xfe]);
        assert!(content.as_text().is_err());
    }

    #[test]
    fn test_parser_input_builder() {
        let input = ParserInput::from_text("content")
            .with_filename("test.txt")
            .with_metadata("key", serde_json::Value::String("value".to_string()));

        assert_eq!(input.filename, Some("test.txt".to_string()));
        assert!(input.metadata.contains_key("key"));
    }

    #[test]
    fn test_document_metadata_merge() {
        let meta1 = DocumentMetadata::new().with_title("Title 1");
        let meta2 = DocumentMetadata::new()
            .with_title("Title 2")
            .with_author("Author 2");

        let merged = meta1.merge(meta2);
        assert_eq!(merged.title, Some("Title 1".to_string()));
        assert_eq!(merged.author, Some("Author 2".to_string()));
    }

    #[test]
    fn test_document_metadata_to_json() {
        let meta = DocumentMetadata::new()
            .with_title("Test")
            .with_source("file.txt");

        let map = meta.to_json_map();
        assert_eq!(
            map.get("title"),
            Some(&serde_json::Value::String("Test".to_string()))
        );
        assert_eq!(
            map.get("source"),
            Some(&serde_json::Value::String("file.txt".to_string()))
        );
    }

    #[tokio::test]
    async fn test_mock_parser() {
        let parser = mock::MockDocumentParser::new()
            .with_result(ParsedDocument::from_content("parsed content"));

        let input = ParserInput::from_text("raw content");
        let result = parser.parse(input).await.unwrap();

        assert_eq!(result.content, "parsed content");
    }
}
