//! Markdown document parser

use async_trait::async_trait;
use pulldown_cmark::{Event, Parser, Tag};

use crate::domain::ingestion::{
    DocumentMetadata, DocumentParser, ParsedDocument, ParserInput,
};
use crate::domain::DomainError;

/// Parser for Markdown files
#[derive(Debug, Clone, Default)]
pub struct MarkdownParser;

impl MarkdownParser {
    /// Create a new Markdown parser
    pub fn new() -> Self {
        Self
    }

    fn extract_text_and_title(markdown: &str) -> (String, Option<String>) {
        let parser = Parser::new(markdown);
        let mut text = String::new();
        let mut title: Option<String> = None;
        let mut in_heading = false;
        let mut heading_level: i32 = 0;
        let mut current_heading = String::new();

        for event in parser {
            match event {
                Event::Start(Tag::Heading(level, ..)) => {
                    in_heading = true;
                    heading_level = level as i32;
                    current_heading.clear();
                }
                Event::End(Tag::Heading(..)) => {
                    if heading_level == 1 && title.is_none() {
                        title = Some(current_heading.trim().to_string());
                    }

                    if !text.is_empty() {
                        text.push('\n');
                    }
                    text.push_str(&current_heading);
                    text.push('\n');
                    in_heading = false;
                    current_heading.clear();
                }
                Event::Text(t) | Event::Code(t) => {
                    if in_heading {
                        current_heading.push_str(&t);
                    } else {
                        text.push_str(&t);
                    }
                }
                Event::SoftBreak | Event::HardBreak => {
                    if in_heading {
                        current_heading.push(' ');
                    } else {
                        text.push(' ');
                    }
                }
                Event::Start(Tag::Paragraph) => {
                    if !text.is_empty() && !text.ends_with('\n') {
                        text.push('\n');
                    }
                }
                Event::End(Tag::Paragraph) => {
                    text.push('\n');
                }
                Event::Start(Tag::Item) => {
                    if !text.is_empty() && !text.ends_with('\n') {
                        text.push('\n');
                    }
                    text.push_str("• ");
                }
                Event::End(Tag::Item) => {
                    if !text.ends_with('\n') {
                        text.push('\n');
                    }
                }
                Event::Start(Tag::CodeBlock(_)) => {
                    if !text.is_empty() && !text.ends_with('\n') {
                        text.push('\n');
                    }
                }
                Event::End(Tag::CodeBlock(_)) => {
                    text.push('\n');
                }
                _ => {}
            }
        }

        let text = text
            .lines()
            .map(|l| l.trim())
            .collect::<Vec<_>>()
            .join("\n")
            .trim()
            .to_string();

        (text, title)
    }
}

#[async_trait]
impl DocumentParser for MarkdownParser {
    fn supported_extensions(&self) -> &[&str] {
        &["md", "markdown"]
    }

    fn supported_mime_types(&self) -> &[&str] {
        &["text/markdown", "text/x-markdown"]
    }

    async fn parse(&self, input: ParserInput) -> Result<ParsedDocument, DomainError> {
        let raw_content = input.content.as_text()?;
        let (content, title) = Self::extract_text_and_title(&raw_content);

        let mut metadata = DocumentMetadata::new().with_mime_type("text/markdown");

        if let Some(t) = title {
            metadata = metadata.with_title(t);
        }

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

    #[tokio::test]
    async fn test_parse_simple_markdown() {
        let parser = MarkdownParser::new();
        let input = ParserInput::from_text("# Hello World\n\nThis is a paragraph.");

        let result = parser.parse(input).await.unwrap();

        assert!(result.content.contains("Hello World"));
        assert!(result.content.contains("This is a paragraph"));
        assert_eq!(result.metadata.title, Some("Hello World".to_string()));
    }

    #[tokio::test]
    async fn test_parse_markdown_with_formatting() {
        let parser = MarkdownParser::new();
        let input = ParserInput::from_text("**bold** and *italic* text");

        let result = parser.parse(input).await.unwrap();

        assert!(result.content.contains("bold"));
        assert!(result.content.contains("italic"));
    }

    #[tokio::test]
    async fn test_parse_markdown_with_code() {
        let parser = MarkdownParser::new();
        let input = ParserInput::from_text("Some `inline code` here.\n\n```rust\nlet x = 1;\n```");

        let result = parser.parse(input).await.unwrap();

        assert!(result.content.contains("inline code"));
        assert!(result.content.contains("let x = 1"));
    }

    #[tokio::test]
    async fn test_parse_markdown_with_lists() {
        let parser = MarkdownParser::new();
        let input = ParserInput::from_text("- Item 1\n- Item 2\n- Item 3");

        let result = parser.parse(input).await.unwrap();

        assert!(result.content.contains("Item 1"));
        assert!(result.content.contains("Item 2"));
        assert!(result.content.contains("Item 3"));
    }

    #[tokio::test]
    async fn test_parse_markdown_without_h1() {
        let parser = MarkdownParser::new();
        let input = ParserInput::from_text("## Secondary Heading\n\nNo H1 here.");

        let result = parser.parse(input).await.unwrap();

        assert!(result.metadata.title.is_none());
    }

    #[tokio::test]
    async fn test_supported_extensions() {
        let parser = MarkdownParser::new();
        assert!(parser.supports_file("readme.md"));
        assert!(parser.supports_file("doc.markdown"));
        assert!(!parser.supports_file("file.txt"));
    }

    #[tokio::test]
    async fn test_supported_mime_types() {
        let parser = MarkdownParser::new();
        assert!(parser.supports_mime("text/markdown"));
        assert!(parser.supports_mime("text/x-markdown"));
        assert!(!parser.supports_mime("text/plain"));
    }
}
