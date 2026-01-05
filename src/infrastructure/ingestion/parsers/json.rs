//! JSON document parser

use async_trait::async_trait;

use crate::domain::ingestion::{
    DocumentMetadata, DocumentParser, ParsedDocument, ParserInput,
};
use crate::domain::DomainError;

/// Parser for JSON files (serializes entire content as text)
#[derive(Debug, Clone, Default)]
pub struct JsonParser;

impl JsonParser {
    /// Create a new JSON parser
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl DocumentParser for JsonParser {
    fn supported_extensions(&self) -> &[&str] {
        &["json"]
    }

    fn supported_mime_types(&self) -> &[&str] {
        &["application/json"]
    }

    async fn parse(&self, input: ParserInput) -> Result<ParsedDocument, DomainError> {
        let raw_content = input.content.as_text()?;

        let json_value: serde_json::Value = serde_json::from_str(&raw_content)
            .map_err(|e| DomainError::validation(format!("Invalid JSON: {}", e)))?;

        let content = serde_json::to_string_pretty(&json_value)
            .map_err(|e| DomainError::validation(format!("Failed to serialize JSON: {}", e)))?;

        let mut metadata = DocumentMetadata::new().with_mime_type("application/json");

        if let Some(ref filename) = input.filename {
            metadata = metadata.with_source(filename.clone());
        }

        metadata = metadata.with_custom(
            "json_structure".to_string(),
            serde_json::Value::String(Self::describe_structure(&json_value)),
        );

        for (key, value) in input.metadata {
            metadata = metadata.with_custom(key, value);
        }

        Ok(ParsedDocument::new(content, metadata))
    }
}

impl JsonParser {
    fn describe_structure(value: &serde_json::Value) -> String {
        match value {
            serde_json::Value::Null => "null".to_string(),
            serde_json::Value::Bool(_) => "boolean".to_string(),
            serde_json::Value::Number(_) => "number".to_string(),
            serde_json::Value::String(_) => "string".to_string(),
            serde_json::Value::Array(arr) => {
                if arr.is_empty() {
                    "array[]".to_string()
                } else {
                    format!("array[{}]", arr.len())
                }
            }
            serde_json::Value::Object(obj) => {
                let keys: Vec<&str> = obj.keys().map(|k| k.as_str()).take(5).collect();
                if keys.len() < obj.len() {
                    format!("object{{{},...}}", keys.join(", "))
                } else {
                    format!("object{{{}}}", keys.join(", "))
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_parse_simple_json() {
        let parser = JsonParser::new();
        let json = r#"{"name": "test", "value": 42}"#;
        let input = ParserInput::from_text(json);

        let result = parser.parse(input).await.unwrap();

        assert!(result.content.contains("name"));
        assert!(result.content.contains("test"));
        assert!(result.content.contains("42"));
    }

    #[tokio::test]
    async fn test_parse_json_array() {
        let parser = JsonParser::new();
        let json = r#"[1, 2, 3, 4, 5]"#;
        let input = ParserInput::from_text(json);

        let result = parser.parse(input).await.unwrap();

        assert!(result.content.contains("1"));
        assert!(result.content.contains("5"));
    }

    #[tokio::test]
    async fn test_parse_nested_json() {
        let parser = JsonParser::new();
        let json = r#"{"outer": {"inner": {"deep": "value"}}}"#;
        let input = ParserInput::from_text(json);

        let result = parser.parse(input).await.unwrap();

        assert!(result.content.contains("outer"));
        assert!(result.content.contains("inner"));
        assert!(result.content.contains("deep"));
        assert!(result.content.contains("value"));
    }

    #[tokio::test]
    async fn test_parse_invalid_json() {
        let parser = JsonParser::new();
        let json = r#"{invalid json}"#;
        let input = ParserInput::from_text(json);

        let result = parser.parse(input).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_json_structure_metadata() {
        let parser = JsonParser::new();
        let json = r#"{"a": 1, "b": 2}"#;
        let input = ParserInput::from_text(json);

        let result = parser.parse(input).await.unwrap();

        assert!(result.metadata.custom.contains_key("json_structure"));
    }

    #[tokio::test]
    async fn test_parse_json_with_filename() {
        let parser = JsonParser::new();
        let json = r#"{"test": true}"#;
        let input = ParserInput::from_text(json).with_filename("data.json");

        let result = parser.parse(input).await.unwrap();

        assert_eq!(result.metadata.source, Some("data.json".to_string()));
        assert_eq!(result.metadata.mime_type, Some("application/json".to_string()));
    }

    #[test]
    fn test_supported_extensions() {
        let parser = JsonParser::new();
        assert!(parser.supports_file("data.json"));
        assert!(!parser.supports_file("data.txt"));
    }

    #[test]
    fn test_supported_mime_types() {
        let parser = JsonParser::new();
        assert!(parser.supports_mime("application/json"));
        assert!(parser.supports_mime("application/json; charset=utf-8"));
        assert!(!parser.supports_mime("text/plain"));
    }

    #[test]
    fn test_describe_structure() {
        assert_eq!(
            JsonParser::describe_structure(&serde_json::Value::Null),
            "null"
        );
        assert_eq!(
            JsonParser::describe_structure(&serde_json::Value::Bool(true)),
            "boolean"
        );
        assert_eq!(
            JsonParser::describe_structure(&serde_json::json!(42)),
            "number"
        );
        assert_eq!(
            JsonParser::describe_structure(&serde_json::json!("test")),
            "string"
        );
        assert_eq!(
            JsonParser::describe_structure(&serde_json::json!([1, 2, 3])),
            "array[3]"
        );
    }
}
