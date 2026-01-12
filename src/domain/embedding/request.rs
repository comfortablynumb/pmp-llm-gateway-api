//! Embedding request types

use serde::{Deserialize, Serialize};

/// Input for embedding generation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum EmbeddingInput {
    /// Single text input
    Single(String),
    /// Batch of text inputs
    Batch(Vec<String>),
}

impl EmbeddingInput {
    /// Get all inputs as a vector
    pub fn as_vec(&self) -> Vec<&str> {
        match self {
            EmbeddingInput::Single(s) => vec![s.as_str()],
            EmbeddingInput::Batch(v) => v.iter().map(|s| s.as_str()).collect(),
        }
    }

    /// Get the number of inputs
    pub fn len(&self) -> usize {
        match self {
            EmbeddingInput::Single(_) => 1,
            EmbeddingInput::Batch(v) => v.len(),
        }
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        match self {
            EmbeddingInput::Single(s) => s.is_empty(),
            EmbeddingInput::Batch(v) => v.is_empty(),
        }
    }
}

/// Request to generate embeddings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingRequest {
    /// Model to use for embedding
    model: String,
    /// Input text(s) to embed
    input: EmbeddingInput,
    /// Optional encoding format
    #[serde(skip_serializing_if = "Option::is_none")]
    encoding_format: Option<String>,
    /// Optional dimensions (for models that support it)
    #[serde(skip_serializing_if = "Option::is_none")]
    dimensions: Option<usize>,
}

impl EmbeddingRequest {
    /// Create a new embedding request
    pub fn new(model: impl Into<String>, input: EmbeddingInput) -> Self {
        Self {
            model: model.into(),
            input,
            encoding_format: None,
            dimensions: None,
        }
    }

    /// Create a request for a single text
    pub fn single(model: impl Into<String>, text: impl Into<String>) -> Self {
        Self::new(model, EmbeddingInput::Single(text.into()))
    }

    /// Create a request for multiple texts
    pub fn batch(model: impl Into<String>, texts: Vec<String>) -> Self {
        Self::new(model, EmbeddingInput::Batch(texts))
    }

    /// Set the encoding format
    pub fn with_encoding_format(mut self, format: impl Into<String>) -> Self {
        self.encoding_format = Some(format.into());
        self
    }

    /// Set the output dimensions
    pub fn with_dimensions(mut self, dimensions: usize) -> Self {
        self.dimensions = Some(dimensions);
        self
    }

    /// Get the model
    pub fn model(&self) -> &str {
        &self.model
    }

    /// Get the input
    pub fn input(&self) -> &EmbeddingInput {
        &self.input
    }

    /// Get inputs as strings
    pub fn inputs(&self) -> Vec<&str> {
        self.input.as_vec()
    }

    /// Get encoding format
    pub fn encoding_format(&self) -> Option<&str> {
        self.encoding_format.as_deref()
    }

    /// Get dimensions
    pub fn dimensions(&self) -> Option<usize> {
        self.dimensions
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedding_input_single() {
        let input = EmbeddingInput::Single("hello".into());

        assert_eq!(input.len(), 1);
        assert!(!input.is_empty());
        assert_eq!(input.as_vec(), vec!["hello"]);
    }

    #[test]
    fn test_embedding_input_batch() {
        let input = EmbeddingInput::Batch(vec!["hello".into(), "world".into()]);

        assert_eq!(input.len(), 2);
        assert!(!input.is_empty());
        assert_eq!(input.as_vec(), vec!["hello", "world"]);
    }

    #[test]
    fn test_embedding_request_single() {
        let request = EmbeddingRequest::single("text-embedding-3-small", "test");

        assert_eq!(request.model(), "text-embedding-3-small");
        assert_eq!(request.inputs(), vec!["test"]);
    }

    #[test]
    fn test_embedding_request_batch() {
        let request =
            EmbeddingRequest::batch("text-embedding-3-small", vec!["a".into(), "b".into()]);

        assert_eq!(request.inputs(), vec!["a", "b"]);
    }

    #[test]
    fn test_embedding_request_with_options() {
        let request = EmbeddingRequest::single("text-embedding-3-small", "test")
            .with_encoding_format("float")
            .with_dimensions(256);

        assert_eq!(request.encoding_format(), Some("float"));
        assert_eq!(request.dimensions(), Some(256));
    }
}
