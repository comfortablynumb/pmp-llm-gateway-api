//! HTML document parser

use async_trait::async_trait;
use scraper::{Html, Selector};

use crate::domain::ingestion::{
    DocumentMetadata, DocumentParser, ParsedDocument, ParserInput,
};
use crate::domain::DomainError;

/// Parser for HTML files
#[derive(Debug, Clone, Default)]
pub struct HtmlParser;

impl HtmlParser {
    /// Create a new HTML parser
    pub fn new() -> Self {
        Self
    }

    fn extract_title(document: &Html) -> Option<String> {
        let title_selector = Selector::parse("title").ok()?;
        document
            .select(&title_selector)
            .next()
            .map(|el| el.text().collect::<String>().trim().to_string())
            .filter(|s| !s.is_empty())
    }

    fn extract_text(document: &Html) -> String {
        let body_selector = Selector::parse("body").ok();

        let root = if let Some(ref sel) = body_selector {
            document.select(sel).next()
        } else {
            None
        };

        let text = if let Some(body) = root {
            Self::extract_element_text(&body)
        } else {
            document.root_element().text().collect::<String>()
        };

        Self::normalize_text(&text)
    }

    fn extract_element_text(element: &scraper::ElementRef) -> String {
        let mut text = String::new();

        for node in element.children() {
            if let Some(el) = scraper::ElementRef::wrap(node) {
                let tag_name = el.value().name();

                if matches!(tag_name, "script" | "style" | "noscript" | "head") {
                    continue;
                }

                if matches!(
                    tag_name,
                    "p" | "div"
                        | "h1"
                        | "h2"
                        | "h3"
                        | "h4"
                        | "h5"
                        | "h6"
                        | "br"
                        | "li"
                        | "tr"
                        | "td"
                        | "th"
                ) {
                    if !text.is_empty() && !text.ends_with('\n') {
                        text.push('\n');
                    }
                }

                text.push_str(&Self::extract_element_text(&el));

                if matches!(tag_name, "p" | "div" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6") {
                    text.push('\n');
                }
            } else if let Some(txt) = node.value().as_text() {
                text.push_str(txt);
            }
        }

        text
    }

    fn normalize_text(text: &str) -> String {
        let lines: Vec<&str> = text
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty())
            .collect();

        lines.join("\n")
    }
}

#[async_trait]
impl DocumentParser for HtmlParser {
    fn supported_extensions(&self) -> &[&str] {
        &["html", "htm"]
    }

    fn supported_mime_types(&self) -> &[&str] {
        &["text/html"]
    }

    async fn parse(&self, input: ParserInput) -> Result<ParsedDocument, DomainError> {
        let raw_content = input.content.as_text()?;
        let document = Html::parse_document(&raw_content);

        let content = Self::extract_text(&document);
        let title = Self::extract_title(&document);

        let mut metadata = DocumentMetadata::new().with_mime_type("text/html");

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
    async fn test_parse_simple_html() {
        let parser = HtmlParser::new();
        let html = r#"
            <!DOCTYPE html>
            <html>
            <head><title>Test Page</title></head>
            <body>
                <h1>Hello World</h1>
                <p>This is a paragraph.</p>
            </body>
            </html>
        "#;
        let input = ParserInput::from_text(html);

        let result = parser.parse(input).await.unwrap();

        assert!(result.content.contains("Hello World"));
        assert!(result.content.contains("This is a paragraph"));
        assert_eq!(result.metadata.title, Some("Test Page".to_string()));
    }

    #[tokio::test]
    async fn test_parse_html_strips_scripts() {
        let parser = HtmlParser::new();
        let html = r#"
            <html>
            <body>
                <p>Visible text</p>
                <script>var x = 'hidden';</script>
                <p>More visible text</p>
            </body>
            </html>
        "#;
        let input = ParserInput::from_text(html);

        let result = parser.parse(input).await.unwrap();

        assert!(result.content.contains("Visible text"));
        assert!(result.content.contains("More visible text"));
        assert!(!result.content.contains("hidden"));
    }

    #[tokio::test]
    async fn test_parse_html_strips_styles() {
        let parser = HtmlParser::new();
        let html = r#"
            <html>
            <head><style>.hidden { display: none; }</style></head>
            <body><p>Content here</p></body>
            </html>
        "#;
        let input = ParserInput::from_text(html);

        let result = parser.parse(input).await.unwrap();

        assert!(result.content.contains("Content here"));
        assert!(!result.content.contains("display"));
    }

    #[tokio::test]
    async fn test_parse_html_with_lists() {
        let parser = HtmlParser::new();
        let html = r#"
            <html>
            <body>
                <ul>
                    <li>Item 1</li>
                    <li>Item 2</li>
                </ul>
            </body>
            </html>
        "#;
        let input = ParserInput::from_text(html);

        let result = parser.parse(input).await.unwrap();

        assert!(result.content.contains("Item 1"));
        assert!(result.content.contains("Item 2"));
    }

    #[tokio::test]
    async fn test_parse_html_without_title() {
        let parser = HtmlParser::new();
        let html = "<html><body><p>No title here</p></body></html>";
        let input = ParserInput::from_text(html);

        let result = parser.parse(input).await.unwrap();

        assert!(result.metadata.title.is_none());
    }

    #[tokio::test]
    async fn test_parse_html_with_nested_elements() {
        let parser = HtmlParser::new();
        let html = r#"
            <html>
            <body>
                <div>
                    <div>
                        <span>Deeply nested</span>
                    </div>
                </div>
            </body>
            </html>
        "#;
        let input = ParserInput::from_text(html);

        let result = parser.parse(input).await.unwrap();

        assert!(result.content.contains("Deeply nested"));
    }

    #[test]
    fn test_supported_extensions() {
        let parser = HtmlParser::new();
        assert!(parser.supports_file("page.html"));
        assert!(parser.supports_file("page.htm"));
        assert!(!parser.supports_file("file.txt"));
    }

    #[test]
    fn test_supported_mime_types() {
        let parser = HtmlParser::new();
        assert!(parser.supports_mime("text/html"));
        assert!(parser.supports_mime("text/html; charset=utf-8"));
        assert!(!parser.supports_mime("text/plain"));
    }
}
