//! Fixed-size chunking strategy

use crate::domain::ingestion::{
    chunker::helpers, Chunk, ChunkingConfig, ChunkingStrategy, ChunkMetadata,
};
use crate::domain::DomainError;

/// Chunking strategy that splits text into fixed-size chunks
#[derive(Debug, Clone, Default)]
pub struct FixedSizeChunker {
    /// Whether to respect word boundaries
    respect_word_boundaries: bool,
}

impl FixedSizeChunker {
    /// Create a new fixed-size chunker
    pub fn new() -> Self {
        Self {
            respect_word_boundaries: true,
        }
    }

    /// Set whether to respect word boundaries
    pub fn with_word_boundaries(mut self, respect: bool) -> Self {
        self.respect_word_boundaries = respect;
        self
    }

    fn find_chunk_end(&self, content: &str, start: usize, target_end: usize) -> usize {
        if !self.respect_word_boundaries || target_end >= content.len() {
            return target_end.min(content.len());
        }

        let boundary = helpers::find_word_boundary_before(content, target_end);

        if boundary <= start {
            helpers::find_word_boundary_after(content, target_end)
        } else {
            boundary
        }
    }
}

impl ChunkingStrategy for FixedSizeChunker {
    fn chunk(&self, content: &str, config: &ChunkingConfig) -> Result<Vec<Chunk>, DomainError> {
        config.validate()?;

        if content.is_empty() {
            return Ok(vec![]);
        }

        let content = content.trim();

        if content.is_empty() {
            return Ok(vec![]);
        }

        if content.len() <= config.chunk_size {
            return Ok(vec![Chunk::new(
                content,
                ChunkMetadata::new(0, 1, 0, content.len()),
            )]);
        }

        let mut chunks = Vec::new();
        let mut start = 0;
        let step = config.chunk_size - config.chunk_overlap;

        while start < content.len() {
            let target_end = (start + config.chunk_size).min(content.len());
            let end = self.find_chunk_end(content, start, target_end);

            let chunk_content = content[start..end].trim();

            if !chunk_content.is_empty() && chunk_content.len() >= config.min_chunk_size {
                chunks.push(Chunk::new(
                    chunk_content,
                    ChunkMetadata::new(chunks.len(), 0, start, end),
                ));
            }

            if end >= content.len() {
                break;
            }

            start += step;

            if start >= end {
                start = end;
            }
        }

        let total = chunks.len();
        for chunk in &mut chunks {
            chunk.metadata.total_chunks = total;
        }

        if chunks.is_empty() && !content.is_empty() {
            chunks.push(Chunk::new(
                content,
                ChunkMetadata::new(0, 1, 0, content.len()),
            ));
        }

        Ok(chunks)
    }

    fn name(&self) -> &'static str {
        "fixed_size"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_content() {
        let chunker = FixedSizeChunker::new();
        let config = ChunkingConfig::default();

        let chunks = chunker.chunk("", &config).unwrap();
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_whitespace_only() {
        let chunker = FixedSizeChunker::new();
        let config = ChunkingConfig::default();

        let chunks = chunker.chunk("   \n\t  ", &config).unwrap();
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_small_content() {
        let chunker = FixedSizeChunker::new();
        let config = ChunkingConfig::new(1000, 200);

        let chunks = chunker.chunk("Hello, World!", &config).unwrap();

        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].content, "Hello, World!");
        assert_eq!(chunks[0].metadata.chunk_index, 0);
        assert_eq!(chunks[0].metadata.total_chunks, 1);
    }

    #[test]
    fn test_multiple_chunks() {
        let chunker = FixedSizeChunker::new();
        let config = ChunkingConfig::new(50, 10).with_min_chunk_size(5);

        let content = "The quick brown fox jumps over the lazy dog. ".repeat(5);
        let chunks = chunker.chunk(&content, &config).unwrap();

        assert!(chunks.len() > 1);

        for (i, chunk) in chunks.iter().enumerate() {
            assert_eq!(chunk.metadata.chunk_index, i);
            assert_eq!(chunk.metadata.total_chunks, chunks.len());
            assert!(!chunk.content.is_empty());
        }
    }

    #[test]
    fn test_respects_word_boundaries() {
        let chunker = FixedSizeChunker::new().with_word_boundaries(true);
        let config = ChunkingConfig::new(10, 0).with_min_chunk_size(1);

        let content = "hello world test";
        let chunks = chunker.chunk(content, &config).unwrap();

        for chunk in &chunks {
            assert!(
                !chunk.content.starts_with(' '),
                "Chunk should not start with space: '{}'",
                chunk.content
            );
            assert!(
                !chunk.content.ends_with(' '),
                "Chunk should not end with space: '{}'",
                chunk.content
            );
        }
    }

    #[test]
    fn test_without_word_boundaries() {
        let chunker = FixedSizeChunker::new().with_word_boundaries(false);
        let config = ChunkingConfig::new(5, 0).with_min_chunk_size(1);

        let content = "abcdefghij";
        let chunks = chunker.chunk(content, &config).unwrap();

        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].content, "abcde");
        assert_eq!(chunks[1].content, "fghij");
    }

    #[test]
    fn test_overlap() {
        let chunker = FixedSizeChunker::new().with_word_boundaries(false);
        let config = ChunkingConfig::new(6, 2).with_min_chunk_size(1);

        let content = "abcdefghijklmnop";
        let chunks = chunker.chunk(content, &config).unwrap();

        assert!(chunks.len() >= 3);
    }

    #[test]
    fn test_chunk_metadata() {
        let chunker = FixedSizeChunker::new();
        let config = ChunkingConfig::new(20, 5).with_min_chunk_size(5);

        let content = "This is a test content that should be split into multiple chunks.";
        let chunks = chunker.chunk(content, &config).unwrap();

        for chunk in &chunks {
            assert!(chunk.metadata.char_start < chunk.metadata.char_end);
            assert!(chunk.metadata.char_end <= content.len());
        }
    }

    #[test]
    fn test_min_chunk_size() {
        let chunker = FixedSizeChunker::new();
        let config = ChunkingConfig::new(10, 0).with_min_chunk_size(5);

        let content = "Hello World Test Content Here";
        let chunks = chunker.chunk(content, &config).unwrap();

        for chunk in &chunks {
            assert!(
                chunk.content.len() >= config.min_chunk_size,
                "Chunk '{}' is smaller than min size",
                chunk.content
            );
        }
    }

    #[test]
    fn test_invalid_config() {
        let chunker = FixedSizeChunker::new();
        let config = ChunkingConfig::new(0, 0);

        let result = chunker.chunk("content", &config);
        assert!(result.is_err());
    }

    #[test]
    fn test_name() {
        let chunker = FixedSizeChunker::new();
        assert_eq!(chunker.name(), "fixed_size");
    }
}
