//! Chunking strategy trait and types

use serde::{Deserialize, Serialize};
use std::fmt::Debug;

use crate::domain::DomainError;

/// Configuration for chunking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkingConfig {
    /// Target chunk size in characters
    pub chunk_size: usize,
    /// Overlap between consecutive chunks in characters
    pub chunk_overlap: usize,
    /// Minimum chunk size (chunks smaller than this are merged)
    pub min_chunk_size: usize,
}

impl ChunkingConfig {
    /// Create a new chunking configuration
    pub fn new(chunk_size: usize, chunk_overlap: usize) -> Self {
        Self {
            chunk_size,
            chunk_overlap,
            min_chunk_size: 50,
        }
    }

    /// Set minimum chunk size
    pub fn with_min_chunk_size(mut self, min_size: usize) -> Self {
        self.min_chunk_size = min_size;
        self
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), DomainError> {
        if self.chunk_size == 0 {
            return Err(DomainError::validation("chunk_size must be greater than 0"));
        }

        if self.chunk_overlap >= self.chunk_size {
            return Err(DomainError::validation(
                "chunk_overlap must be less than chunk_size",
            ));
        }

        if self.min_chunk_size > self.chunk_size {
            return Err(DomainError::validation(
                "min_chunk_size must be less than or equal to chunk_size",
            ));
        }

        Ok(())
    }
}

impl Default for ChunkingConfig {
    fn default() -> Self {
        Self {
            chunk_size: 1000,
            chunk_overlap: 200,
            min_chunk_size: 50,
        }
    }
}

/// Metadata for a chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkMetadata {
    /// Index of this chunk (0-based)
    pub chunk_index: usize,
    /// Total number of chunks
    pub total_chunks: usize,
    /// Character offset where this chunk starts
    pub char_start: usize,
    /// Character offset where this chunk ends
    pub char_end: usize,
}

impl ChunkMetadata {
    /// Create new chunk metadata
    pub fn new(chunk_index: usize, total_chunks: usize, char_start: usize, char_end: usize) -> Self {
        Self {
            chunk_index,
            total_chunks,
            char_start,
            char_end,
        }
    }

    /// Convert to JSON value map
    pub fn to_json_map(&self) -> std::collections::HashMap<String, serde_json::Value> {
        let mut map = std::collections::HashMap::new();
        map.insert(
            "chunk_index".to_string(),
            serde_json::Value::Number(self.chunk_index.into()),
        );
        map.insert(
            "total_chunks".to_string(),
            serde_json::Value::Number(self.total_chunks.into()),
        );
        map.insert(
            "char_start".to_string(),
            serde_json::Value::Number(self.char_start.into()),
        );
        map.insert(
            "char_end".to_string(),
            serde_json::Value::Number(self.char_end.into()),
        );
        map
    }
}

/// A chunk of text extracted from a document
#[derive(Debug, Clone)]
pub struct Chunk {
    /// Chunk content
    pub content: String,
    /// Chunk metadata
    pub metadata: ChunkMetadata,
}

impl Chunk {
    /// Create a new chunk
    pub fn new(content: impl Into<String>, metadata: ChunkMetadata) -> Self {
        Self {
            content: content.into(),
            metadata,
        }
    }

    /// Get the chunk index
    pub fn index(&self) -> usize {
        self.metadata.chunk_index
    }

    /// Get the content length
    pub fn len(&self) -> usize {
        self.content.len()
    }

    /// Check if the chunk is empty
    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }
}

/// Trait for chunking strategies
pub trait ChunkingStrategy: Send + Sync + Debug {
    /// Split content into chunks
    fn chunk(&self, content: &str, config: &ChunkingConfig) -> Result<Vec<Chunk>, DomainError>;

    /// Get the strategy name
    fn name(&self) -> &'static str;
}

/// Helper functions for chunking
pub mod helpers {
    /// Find the nearest word boundary before a position
    pub fn find_word_boundary_before(text: &str, pos: usize) -> usize {
        if pos >= text.len() {
            return text.len();
        }

        let bytes = text.as_bytes();
        let mut boundary = pos;

        while boundary > 0 && !bytes[boundary - 1].is_ascii_whitespace() {
            boundary -= 1;
        }

        if boundary == 0 {
            pos
        } else {
            boundary
        }
    }

    /// Find the nearest word boundary after a position
    pub fn find_word_boundary_after(text: &str, pos: usize) -> usize {
        if pos >= text.len() {
            return text.len();
        }

        let bytes = text.as_bytes();
        let mut boundary = pos;

        while boundary < text.len() && !bytes[boundary].is_ascii_whitespace() {
            boundary += 1;
        }

        boundary
    }

    /// Trim whitespace from both ends and normalize internal whitespace
    pub fn normalize_whitespace(text: &str) -> String {
        text.split_whitespace().collect::<Vec<_>>().join(" ")
    }
}

#[cfg(test)]
pub mod mock {
    use super::*;
    use std::sync::Mutex;

    /// Mock chunking strategy for testing
    #[derive(Debug)]
    pub struct MockChunkingStrategy {
        name: &'static str,
        result: Mutex<Option<Result<Vec<Chunk>, String>>>,
    }

    impl MockChunkingStrategy {
        pub fn new() -> Self {
            Self {
                name: "mock",
                result: Mutex::new(None),
            }
        }

        pub fn with_name(mut self, name: &'static str) -> Self {
            self.name = name;
            self
        }

        pub fn with_result(self, chunks: Vec<Chunk>) -> Self {
            *self.result.lock().unwrap() = Some(Ok(chunks));
            self
        }

        pub fn with_error(self, error: impl Into<String>) -> Self {
            *self.result.lock().unwrap() = Some(Err(error.into()));
            self
        }
    }

    impl Default for MockChunkingStrategy {
        fn default() -> Self {
            Self::new()
        }
    }

    impl ChunkingStrategy for MockChunkingStrategy {
        fn chunk(&self, content: &str, config: &ChunkingConfig) -> Result<Vec<Chunk>, DomainError> {
            if let Some(result) = self.result.lock().unwrap().take() {
                return result.map_err(DomainError::validation);
            }

            config.validate()?;

            if content.is_empty() {
                return Ok(vec![]);
            }

            Ok(vec![Chunk::new(
                content,
                ChunkMetadata::new(0, 1, 0, content.len()),
            )])
        }

        fn name(&self) -> &'static str {
            self.name
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunking_config_default() {
        let config = ChunkingConfig::default();
        assert_eq!(config.chunk_size, 1000);
        assert_eq!(config.chunk_overlap, 200);
        assert_eq!(config.min_chunk_size, 50);
    }

    #[test]
    fn test_chunking_config_validation() {
        let config = ChunkingConfig::new(100, 50);
        assert!(config.validate().is_ok());

        let invalid = ChunkingConfig::new(0, 0);
        assert!(invalid.validate().is_err());

        let invalid = ChunkingConfig::new(100, 100);
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_chunk_metadata_to_json() {
        let meta = ChunkMetadata::new(0, 5, 0, 100);
        let map = meta.to_json_map();

        assert_eq!(
            map.get("chunk_index"),
            Some(&serde_json::Value::Number(0.into()))
        );
        assert_eq!(
            map.get("total_chunks"),
            Some(&serde_json::Value::Number(5.into()))
        );
    }

    #[test]
    fn test_find_word_boundary_before() {
        let text = "hello world test";
        assert_eq!(helpers::find_word_boundary_before(text, 8), 6);
        assert_eq!(helpers::find_word_boundary_before(text, 5), 5);
    }

    #[test]
    fn test_find_word_boundary_after() {
        let text = "hello world test";
        assert_eq!(helpers::find_word_boundary_after(text, 3), 5);
        assert_eq!(helpers::find_word_boundary_after(text, 6), 11);
    }

    #[test]
    fn test_normalize_whitespace() {
        let text = "  hello   world  \n\t test  ";
        assert_eq!(helpers::normalize_whitespace(text), "hello world test");
    }

    #[test]
    fn test_mock_chunking_strategy() {
        let strategy = mock::MockChunkingStrategy::new();
        let config = ChunkingConfig::default();

        let chunks = strategy.chunk("hello world", &config).unwrap();
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].content, "hello world");
    }

    #[test]
    fn test_mock_chunking_strategy_empty() {
        let strategy = mock::MockChunkingStrategy::new();
        let config = ChunkingConfig::default();

        let chunks = strategy.chunk("", &config).unwrap();
        assert!(chunks.is_empty());
    }
}
