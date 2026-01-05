//! Recursive chunking strategy

use unicode_segmentation::UnicodeSegmentation;

use crate::domain::ingestion::{Chunk, ChunkingConfig, ChunkingStrategy, ChunkMetadata};
use crate::domain::DomainError;

/// Chunking strategy that recursively splits text hierarchically
///
/// Splitting order: headers -> paragraphs -> sentences -> words -> characters
#[derive(Debug, Clone, Default)]
pub struct RecursiveChunker;

impl RecursiveChunker {
    /// Create a new recursive chunker
    pub fn new() -> Self {
        Self
    }

    fn split_by_headers(text: &str) -> Vec<&str> {
        let mut parts = Vec::new();
        let mut current_start = 0;

        for (i, line) in text.lines().enumerate() {
            if line.starts_with('#') && i > 0 {
                let pos = text[current_start..]
                    .find(line)
                    .map(|p| current_start + p)
                    .unwrap_or(current_start);

                if pos > current_start {
                    let part = &text[current_start..pos];

                    if !part.trim().is_empty() {
                        parts.push(part.trim());
                    }
                }
                current_start = pos;
            }
        }

        if current_start < text.len() {
            let part = &text[current_start..];

            if !part.trim().is_empty() {
                parts.push(part.trim());
            }
        }

        if parts.is_empty() && !text.trim().is_empty() {
            parts.push(text.trim());
        }

        parts
    }

    fn split_by_paragraphs(text: &str) -> Vec<&str> {
        text.split("\n\n")
            .map(|p| p.trim())
            .filter(|p| !p.is_empty())
            .collect()
    }

    fn split_by_sentences(text: &str) -> Vec<&str> {
        text.unicode_sentences().collect()
    }

    fn recursive_chunk(
        text: &str,
        config: &ChunkingConfig,
        level: usize,
    ) -> Vec<String> {
        if text.len() <= config.chunk_size {
            return vec![text.to_string()];
        }

        let parts = match level {
            0 => Self::split_by_headers(text),
            1 => Self::split_by_paragraphs(text),
            2 => Self::split_by_sentences(text),
            _ => return Self::split_by_size(text, config),
        };

        if parts.len() <= 1 {
            return Self::recursive_chunk(text, config, level + 1);
        }

        let mut result = Vec::new();
        let mut current = String::new();

        for part in parts {
            let separator = match level {
                0 => "\n\n",
                1 => "\n\n",
                _ => " ",
            };

            if current.is_empty() {
                current = part.to_string();
            } else if current.len() + separator.len() + part.len() <= config.chunk_size {
                current.push_str(separator);
                current.push_str(part);
            } else {
                if current.len() > config.chunk_size {
                    result.extend(Self::recursive_chunk(&current, config, level + 1));
                } else {
                    result.push(current);
                }
                current = part.to_string();
            }
        }

        if !current.is_empty() {
            if current.len() > config.chunk_size {
                result.extend(Self::recursive_chunk(&current, config, level + 1));
            } else {
                result.push(current);
            }
        }

        result
    }

    fn split_by_size(text: &str, config: &ChunkingConfig) -> Vec<String> {
        let mut result = Vec::new();
        let mut current = String::new();

        for word in text.split_whitespace() {
            if current.is_empty() {
                current = word.to_string();
            } else if current.len() + 1 + word.len() <= config.chunk_size {
                current.push(' ');
                current.push_str(word);
            } else {
                result.push(current);
                current = word.to_string();
            }
        }

        if !current.is_empty() {
            result.push(current);
        }

        if result.is_empty() && !text.is_empty() {
            let mut pos = 0;
            while pos < text.len() {
                let end = (pos + config.chunk_size).min(text.len());
                result.push(text[pos..end].to_string());
                pos = end;
            }
        }

        result
    }
}

impl ChunkingStrategy for RecursiveChunker {
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

        let chunk_texts = Self::recursive_chunk(content, config, 0);

        let chunks: Vec<Chunk> = chunk_texts
            .into_iter()
            .filter(|c| c.len() >= config.min_chunk_size)
            .enumerate()
            .map(|(i, text)| {
                let start = content.find(&text).unwrap_or(0);
                Chunk::new(text.clone(), ChunkMetadata::new(i, 0, start, start + text.len()))
            })
            .collect();

        let total = chunks.len();
        let mut chunks = chunks;
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
        "recursive"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_content() {
        let chunker = RecursiveChunker::new();
        let config = ChunkingConfig::default();

        let chunks = chunker.chunk("", &config).unwrap();
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_small_content() {
        let chunker = RecursiveChunker::new();
        let config = ChunkingConfig::new(1000, 0);

        let chunks = chunker.chunk("Small content", &config).unwrap();

        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].content, "Small content");
    }

    #[test]
    fn test_split_by_headers() {
        let chunker = RecursiveChunker::new();
        let config = ChunkingConfig::new(50, 0).with_min_chunk_size(5);

        let content = "# Header 1\n\nContent under header 1.\n\n# Header 2\n\nContent under header 2.";
        let chunks = chunker.chunk(content, &config).unwrap();

        assert!(chunks.len() >= 2);
    }

    #[test]
    fn test_split_by_paragraphs() {
        let chunker = RecursiveChunker::new();
        let config = ChunkingConfig::new(30, 0).with_min_chunk_size(5);

        let content = "First paragraph here.\n\nSecond paragraph here.\n\nThird paragraph here.";
        let chunks = chunker.chunk(content, &config).unwrap();

        assert!(chunks.len() >= 2);
    }

    #[test]
    fn test_split_by_sentences() {
        let chunker = RecursiveChunker::new();
        let config = ChunkingConfig::new(25, 0).with_min_chunk_size(5);

        let content = "First sentence here. Second sentence here. Third sentence here.";
        let chunks = chunker.chunk(content, &config).unwrap();

        assert!(chunks.len() >= 2);
    }

    #[test]
    fn test_chunk_metadata() {
        let chunker = RecursiveChunker::new();
        let config = ChunkingConfig::new(50, 0).with_min_chunk_size(5);

        let content = "# Section 1\n\nParagraph 1.\n\n# Section 2\n\nParagraph 2.";
        let chunks = chunker.chunk(content, &config).unwrap();

        for (i, chunk) in chunks.iter().enumerate() {
            assert_eq!(chunk.metadata.chunk_index, i);
            assert_eq!(chunk.metadata.total_chunks, chunks.len());
        }
    }

    #[test]
    fn test_name() {
        let chunker = RecursiveChunker::new();
        assert_eq!(chunker.name(), "recursive");
    }

    #[test]
    fn test_long_paragraph() {
        let chunker = RecursiveChunker::new();
        let config = ChunkingConfig::new(50, 0).with_min_chunk_size(5);

        let content = "This is a very long paragraph that should be split into multiple chunks because it exceeds the maximum chunk size limit.";
        let chunks = chunker.chunk(content, &config).unwrap();

        assert!(chunks.len() >= 2);

        for chunk in &chunks {
            assert!(chunk.content.len() <= config.chunk_size + 20);
        }
    }

    #[test]
    fn test_mixed_content() {
        let chunker = RecursiveChunker::new();
        let config = ChunkingConfig::new(100, 0).with_min_chunk_size(10);

        let content = r#"# Introduction

This is the introduction paragraph.

# Main Content

This is the main content. It has multiple sentences. Each sentence adds to the content.

## Subsection

More detailed content here.

# Conclusion

Final thoughts."#;

        let chunks = chunker.chunk(content, &config).unwrap();

        assert!(chunks.len() >= 2);

        let combined: String = chunks.iter().map(|c| c.content.as_str()).collect::<Vec<_>>().join(" ");
        assert!(combined.contains("Introduction"));
        assert!(combined.contains("Conclusion"));
    }
}
