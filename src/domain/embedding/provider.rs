//! Embedding provider trait definition

use async_trait::async_trait;
use std::fmt::Debug;

use super::{EmbeddingRequest, EmbeddingResponse};
use crate::domain::DomainError;

/// Trait for embedding providers (OpenAI, Cohere, etc.)
#[async_trait]
pub trait EmbeddingProvider: Send + Sync + Debug {
    /// Generate embeddings for the given input
    async fn embed(&self, request: EmbeddingRequest) -> Result<EmbeddingResponse, DomainError>;

    /// Get the provider name
    fn provider_name(&self) -> &'static str;

    /// Get the default model for this provider
    fn default_model(&self) -> &'static str;

    /// Get the embedding dimensions for a model
    fn dimensions(&self, model: &str) -> Option<usize>;
}

#[cfg(test)]
pub mod mock {
    use super::*;
    use crate::domain::embedding::{Embedding, EmbeddingUsage};

    #[derive(Debug)]
    pub struct MockEmbeddingProvider {
        name: &'static str,
        dimensions: usize,
        error: Option<String>,
    }

    impl MockEmbeddingProvider {
        pub fn new(name: &'static str, dimensions: usize) -> Self {
            Self {
                name,
                dimensions,
                error: None,
            }
        }

        pub fn with_error(mut self, error: impl Into<String>) -> Self {
            self.error = Some(error.into());
            self
        }
    }

    #[async_trait]
    impl EmbeddingProvider for MockEmbeddingProvider {
        async fn embed(&self, request: EmbeddingRequest) -> Result<EmbeddingResponse, DomainError> {
            if let Some(ref error) = self.error {
                return Err(DomainError::provider(self.name, error));
            }

            let inputs = request.inputs();
            let embeddings: Vec<Embedding> = inputs
                .iter()
                .enumerate()
                .map(|(idx, text)| {
                    // Generate deterministic mock embedding based on text hash
                    let hash = text.bytes().fold(0u64, |acc, b| acc.wrapping_add(b as u64));
                    let vector: Vec<f32> = (0..self.dimensions)
                        .map(|i| ((hash.wrapping_add(i as u64) % 1000) as f32 / 1000.0) - 0.5)
                        .collect();

                    Embedding::new(idx, vector)
                })
                .collect();

            let total_tokens = inputs.iter().map(|t| t.len() / 4).sum::<usize>() as u32;

            Ok(EmbeddingResponse::new(
                request.model().to_string(),
                embeddings,
                EmbeddingUsage::new(total_tokens, total_tokens),
            ))
        }

        fn provider_name(&self) -> &'static str {
            self.name
        }

        fn default_model(&self) -> &'static str {
            "mock-embedding"
        }

        fn dimensions(&self, _model: &str) -> Option<usize> {
            Some(self.dimensions)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::domain::embedding::EmbeddingInput;

        #[tokio::test]
        async fn test_mock_provider_single_input() {
            let provider = MockEmbeddingProvider::new("test", 128);
            let request =
                EmbeddingRequest::new("mock-embedding", EmbeddingInput::Single("Hello".into()));

            let response = provider.embed(request).await.unwrap();

            assert_eq!(response.embeddings().len(), 1);
            assert_eq!(response.embeddings()[0].vector().len(), 128);
        }

        #[tokio::test]
        async fn test_mock_provider_batch_input() {
            let provider = MockEmbeddingProvider::new("test", 256);
            let request = EmbeddingRequest::new(
                "mock-embedding",
                EmbeddingInput::Batch(vec!["Hello".into(), "World".into()]),
            );

            let response = provider.embed(request).await.unwrap();

            assert_eq!(response.embeddings().len(), 2);
            assert_eq!(response.embeddings()[0].vector().len(), 256);
            assert_eq!(response.embeddings()[1].vector().len(), 256);
        }

        #[tokio::test]
        async fn test_mock_provider_error() {
            let provider = MockEmbeddingProvider::new("test", 128).with_error("API error");
            let request =
                EmbeddingRequest::new("mock-embedding", EmbeddingInput::Single("Hello".into()));

            let result = provider.embed(request).await;

            assert!(result.is_err());
        }

        #[tokio::test]
        async fn test_deterministic_embeddings() {
            let provider = MockEmbeddingProvider::new("test", 128);
            let request1 =
                EmbeddingRequest::new("mock-embedding", EmbeddingInput::Single("Hello".into()));
            let request2 =
                EmbeddingRequest::new("mock-embedding", EmbeddingInput::Single("Hello".into()));

            let response1 = provider.embed(request1).await.unwrap();
            let response2 = provider.embed(request2).await.unwrap();

            assert_eq!(
                response1.embeddings()[0].vector(),
                response2.embeddings()[0].vector()
            );
        }
    }
}
