//! Sentence-based chunking strategy

use unicode_segmentation::UnicodeSegmentation;

use crate::domain::ingestion::{Chunk, ChunkingConfig, ChunkingStrategy, ChunkMetadata};
use crate::domain::DomainError;

/// Chunking strategy that splits text by sentences
#[derive(Debug, Clone, Default)]
pub struct SentenceChunker;

impl SentenceChunker {
    /// Create a new sentence chunker
    pub fn new() -> Self {
        Self
    }

    fn split_sentences(text: &str) -> Vec<&str> {
        text.unicode_sentences().collect()
    }
}

impl ChunkingStrategy for SentenceChunker {
    fn chunk(&self, content: &str, config: &ChunkingConfig) -> Result<Vec<Chunk>, DomainError> {
        config.validate()?;

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

        let sentences = Self::split_sentences(content);

        if sentences.is_empty() {
            return Ok(vec![Chunk::new(
                content,
                ChunkMetadata::new(0, 1, 0, content.len()),
            )]);
        }

        let mut chunks = Vec::new();
        let mut current_chunk = String::new();
        let mut chunk_start = 0;
        let mut current_pos = 0;

        for sentence in sentences {
            let sentence = sentence.trim();

            if sentence.is_empty() {
                continue;
            }

            if current_chunk.is_empty() {
                current_chunk.push_str(sentence);
                chunk_start = current_pos;
            } else if current_chunk.len() + 1 + sentence.len() <= config.chunk_size {
                current_chunk.push(' ');
                current_chunk.push_str(sentence);
            } else {
                if current_chunk.len() >= config.min_chunk_size {
                    let chunk_end = chunk_start + current_chunk.len();
                    chunks.push(Chunk::new(
                        current_chunk.clone(),
                        ChunkMetadata::new(chunks.len(), 0, chunk_start, chunk_end),
                    ));
                }

                if config.chunk_overlap > 0 && !current_chunk.is_empty() {
                    let overlap_start = current_chunk.len().saturating_sub(config.chunk_overlap);
                    let overlap = current_chunk[overlap_start..].to_string();
                    let overlap_len = overlap.len();
                    current_chunk = format!("{} {}", overlap, sentence);
                    chunk_start = current_pos.saturating_sub(overlap_len);
                } else {
                    current_chunk = sentence.to_string();
                    chunk_start = current_pos;
                }
            }

            current_pos += sentence.len() + 1;
        }

        if !current_chunk.is_empty() && current_chunk.len() >= config.min_chunk_size {
            let chunk_end = chunk_start + current_chunk.len();
            chunks.push(Chunk::new(
                current_chunk,
                ChunkMetadata::new(chunks.len(), 0, chunk_start, chunk_end),
            ));
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
        "sentence"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_content() {
        let chunker = SentenceChunker::new();
        let config = ChunkingConfig::default();

        let chunks = chunker.chunk("", &config).unwrap();
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_single_sentence() {
        let chunker = SentenceChunker::new();
        let config = ChunkingConfig::new(1000, 0);

        let chunks = chunker.chunk("This is a single sentence.", &config).unwrap();

        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].content, "This is a single sentence.");
    }

    #[test]
    fn test_multiple_sentences_small_chunks() {
        let chunker = SentenceChunker::new();
        let config = ChunkingConfig::new(50, 0).with_min_chunk_size(5);

        let content = "First sentence here. Second sentence here. Third sentence here.";
        let chunks = chunker.chunk(content, &config).unwrap();

        assert!(chunks.len() > 1);

        for chunk in &chunks {
            assert!(!chunk.content.is_empty());
        }
    }

    #[test]
    fn test_sentences_combined_to_chunk_size() {
        let chunker = SentenceChunker::new();
        let config = ChunkingConfig::new(100, 0).with_min_chunk_size(5);

        let content = "Short. Another short. One more. And another one here.";
        let chunks = chunker.chunk(content, &config).unwrap();

        for chunk in &chunks {
            assert!(chunk.content.len() <= config.chunk_size + 50);
        }
    }

    #[test]
    fn test_chunk_metadata() {
        let chunker = SentenceChunker::new();
        let config = ChunkingConfig::new(30, 0).with_min_chunk_size(5);

        let content = "First sentence. Second sentence. Third sentence.";
        let chunks = chunker.chunk(content, &config).unwrap();

        for (i, chunk) in chunks.iter().enumerate() {
            assert_eq!(chunk.metadata.chunk_index, i);
            assert_eq!(chunk.metadata.total_chunks, chunks.len());
        }
    }

    #[test]
    fn test_name() {
        let chunker = SentenceChunker::new();
        assert_eq!(chunker.name(), "sentence");
    }

    #[test]
    fn test_with_overlap() {
        let chunker = SentenceChunker::new();
        let config = ChunkingConfig::new(50, 10).with_min_chunk_size(5);

        let content = "First sentence here is long. Second sentence here is also long.";
        let chunks = chunker.chunk(content, &config).unwrap();

        assert!(chunks.len() >= 1);
    }

    #[test]
    fn test_unicode_sentences() {
        let chunker = SentenceChunker::new();
        let config = ChunkingConfig::new(100, 0).with_min_chunk_size(5);

        let content = "Hello world! Привет мир! 你好世界!";
        let chunks = chunker.chunk(content, &config).unwrap();

        assert!(!chunks.is_empty());
        let combined: String = chunks.iter().map(|c| c.content.as_str()).collect();
        assert!(combined.contains("Hello"));
        assert!(combined.contains("Привет"));
        assert!(combined.contains("你好"));
    }
}
