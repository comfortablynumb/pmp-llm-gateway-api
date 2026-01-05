//! Paragraph-based chunking strategy

use crate::domain::ingestion::{Chunk, ChunkingConfig, ChunkingStrategy, ChunkMetadata};
use crate::domain::DomainError;

/// Chunking strategy that splits text by paragraphs
#[derive(Debug, Clone, Default)]
pub struct ParagraphChunker;

impl ParagraphChunker {
    /// Create a new paragraph chunker
    pub fn new() -> Self {
        Self
    }

    fn split_paragraphs(text: &str) -> Vec<&str> {
        text.split("\n\n")
            .map(|p| p.trim())
            .filter(|p| !p.is_empty())
            .collect()
    }
}

impl ChunkingStrategy for ParagraphChunker {
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

        let paragraphs = Self::split_paragraphs(content);

        if paragraphs.is_empty() {
            return Ok(vec![Chunk::new(
                content,
                ChunkMetadata::new(0, 1, 0, content.len()),
            )]);
        }

        let mut chunks = Vec::new();
        let mut current_chunk = String::new();
        let mut chunk_start = 0;
        let mut current_pos = 0;

        for paragraph in paragraphs {
            if current_chunk.is_empty() {
                current_chunk.push_str(paragraph);
                chunk_start = current_pos;
            } else if current_chunk.len() + 2 + paragraph.len() <= config.chunk_size {
                current_chunk.push_str("\n\n");
                current_chunk.push_str(paragraph);
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
                    current_chunk = format!("{}\n\n{}", overlap, paragraph);
                    chunk_start = current_pos.saturating_sub(overlap_len);
                } else {
                    current_chunk = paragraph.to_string();
                    chunk_start = current_pos;
                }
            }

            current_pos += paragraph.len() + 2;
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
        "paragraph"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_content() {
        let chunker = ParagraphChunker::new();
        let config = ChunkingConfig::default();

        let chunks = chunker.chunk("", &config).unwrap();
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_single_paragraph() {
        let chunker = ParagraphChunker::new();
        let config = ChunkingConfig::new(1000, 0);

        let chunks = chunker.chunk("This is a single paragraph.", &config).unwrap();

        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].content, "This is a single paragraph.");
    }

    #[test]
    fn test_multiple_paragraphs() {
        let chunker = ParagraphChunker::new();
        let config = ChunkingConfig::new(50, 0).with_min_chunk_size(5);

        let content = "First paragraph here.\n\nSecond paragraph here.\n\nThird paragraph here.";
        let chunks = chunker.chunk(content, &config).unwrap();

        assert!(chunks.len() >= 1);

        for chunk in &chunks {
            assert!(!chunk.content.is_empty());
        }
    }

    #[test]
    fn test_paragraphs_combined_to_chunk_size() {
        let chunker = ParagraphChunker::new();
        let config = ChunkingConfig::new(200, 0).with_min_chunk_size(5);

        let content = "Short para.\n\nAnother short.\n\nOne more.\n\nAnd another.";
        let chunks = chunker.chunk(content, &config).unwrap();

        assert!(!chunks.is_empty());
    }

    #[test]
    fn test_chunk_metadata() {
        let chunker = ParagraphChunker::new();
        let config = ChunkingConfig::new(30, 0).with_min_chunk_size(5);

        let content = "First paragraph.\n\nSecond paragraph.\n\nThird paragraph.";
        let chunks = chunker.chunk(content, &config).unwrap();

        for (i, chunk) in chunks.iter().enumerate() {
            assert_eq!(chunk.metadata.chunk_index, i);
            assert_eq!(chunk.metadata.total_chunks, chunks.len());
        }
    }

    #[test]
    fn test_name() {
        let chunker = ParagraphChunker::new();
        assert_eq!(chunker.name(), "paragraph");
    }

    #[test]
    fn test_with_overlap() {
        let chunker = ParagraphChunker::new();
        let config = ChunkingConfig::new(50, 10).with_min_chunk_size(5);

        let content = "First paragraph is long.\n\nSecond paragraph is also long.";
        let chunks = chunker.chunk(content, &config).unwrap();

        assert!(chunks.len() >= 1);
    }

    #[test]
    fn test_multiple_newlines() {
        let chunker = ParagraphChunker::new();
        let config = ChunkingConfig::new(1000, 0);

        let content = "Para one.\n\n\n\nPara two.\n\n\n\n\nPara three.";
        let chunks = chunker.chunk(content, &config).unwrap();

        assert_eq!(chunks.len(), 1);
        assert!(chunks[0].content.contains("Para one"));
        assert!(chunks[0].content.contains("Para two"));
        assert!(chunks[0].content.contains("Para three"));
    }
}
