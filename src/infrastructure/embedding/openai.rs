//! OpenAI embedding provider implementation

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::HttpClientTrait;
use crate::domain::embedding::{
    Embedding, EmbeddingInput, EmbeddingProvider, EmbeddingRequest, EmbeddingResponse,
    EmbeddingUsage,
};
use crate::domain::DomainError;

const DEFAULT_OPENAI_BASE_URL: &str = "https://api.openai.com";

/// Known OpenAI embedding models and their dimensions
const EMBEDDING_MODELS: &[(&str, usize)] = &[
    ("text-embedding-3-small", 1536),
    ("text-embedding-3-large", 3072),
    ("text-embedding-ada-002", 1536),
];

/// OpenAI embedding provider
#[derive(Debug)]
pub struct OpenAiEmbeddingProvider<C: HttpClientTrait> {
    client: C,
    auth_header: String,
    base_url: String,
}

impl<C: HttpClientTrait> OpenAiEmbeddingProvider<C> {
    /// Create a new OpenAI embedding provider
    pub fn new(client: C, api_key: impl Into<String>) -> Self {
        Self::with_base_url(client, api_key, DEFAULT_OPENAI_BASE_URL)
    }

    /// Create a new provider with custom base URL
    pub fn with_base_url(
        client: C,
        api_key: impl Into<String>,
        base_url: impl Into<String>,
    ) -> Self {
        let api_key = api_key.into();
        let auth_header = format!("Bearer {}", api_key);
        let base_url = base_url.into().trim_end_matches('/').to_string();

        Self {
            client,
            auth_header,
            base_url,
        }
    }

    fn embeddings_url(&self) -> String {
        format!("{}/v1/embeddings", self.base_url)
    }

    fn headers(&self) -> Vec<(&str, &str)> {
        vec![
            ("Authorization", self.auth_header.as_str()),
            ("Content-Type", "application/json"),
        ]
    }

    fn build_request(&self, request: &EmbeddingRequest) -> serde_json::Value {
        let input = match request.input() {
            EmbeddingInput::Single(s) => serde_json::json!(s),
            EmbeddingInput::Batch(v) => serde_json::json!(v),
        };

        let mut body = serde_json::json!({
            "model": request.model(),
            "input": input,
        });

        if let Some(format) = request.encoding_format() {
            body["encoding_format"] = serde_json::json!(format);
        }

        if let Some(dims) = request.dimensions() {
            body["dimensions"] = serde_json::json!(dims);
        }

        body
    }

    fn parse_response(
        &self,
        json: serde_json::Value,
    ) -> Result<EmbeddingResponse, DomainError> {
        let response: OpenAiEmbeddingResponse = serde_json::from_value(json).map_err(|e| {
            DomainError::provider("openai", format!("Failed to parse embedding response: {}", e))
        })?;

        let embeddings: Vec<Embedding> = response
            .data
            .into_iter()
            .map(|d| Embedding::new(d.index, d.embedding))
            .collect();

        let usage = EmbeddingUsage::new(response.usage.prompt_tokens, response.usage.total_tokens);

        Ok(EmbeddingResponse::new(response.model, embeddings, usage))
    }
}

#[async_trait]
impl<C: HttpClientTrait> EmbeddingProvider for OpenAiEmbeddingProvider<C> {
    async fn embed(&self, request: EmbeddingRequest) -> Result<EmbeddingResponse, DomainError> {
        let url = self.embeddings_url();
        let body = self.build_request(&request);

        let response = self
            .client
            .post_json(&url, self.headers(), &body)
            .await?;

        self.parse_response(response)
    }

    fn provider_name(&self) -> &'static str {
        "openai"
    }

    fn default_model(&self) -> &'static str {
        "text-embedding-3-small"
    }

    fn dimensions(&self, model: &str) -> Option<usize> {
        EMBEDDING_MODELS
            .iter()
            .find(|(name, _)| *name == model)
            .map(|(_, dims)| *dims)
    }
}

// OpenAI API types for embeddings

#[derive(Debug, Serialize, Deserialize)]
struct OpenAiEmbeddingResponse {
    model: String,
    data: Vec<OpenAiEmbeddingData>,
    usage: OpenAiEmbeddingUsage,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAiEmbeddingData {
    index: usize,
    embedding: Vec<f32>,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAiEmbeddingUsage {
    prompt_tokens: u32,
    total_tokens: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::llm::MockHttpClient;

    const TEST_URL: &str = "https://api.openai.com/v1/embeddings";

    fn create_mock_response(num_embeddings: usize, dimensions: usize) -> serde_json::Value {
        let data: Vec<serde_json::Value> = (0..num_embeddings)
            .map(|i| {
                let embedding: Vec<f32> = (0..dimensions).map(|j| (i + j) as f32 * 0.001).collect();
                serde_json::json!({
                    "index": i,
                    "embedding": embedding,
                    "object": "embedding"
                })
            })
            .collect();

        serde_json::json!({
            "model": "text-embedding-3-small",
            "data": data,
            "usage": {
                "prompt_tokens": 10,
                "total_tokens": 10
            }
        })
    }

    #[tokio::test]
    async fn test_embed_single_text() {
        let mock_response = create_mock_response(1, 1536);
        let client = MockHttpClient::new().with_response(TEST_URL, mock_response);
        let provider = OpenAiEmbeddingProvider::new(client, "test-api-key");

        let request = EmbeddingRequest::single("text-embedding-3-small", "Hello world");
        let response = provider.embed(request).await.unwrap();

        assert_eq!(response.model(), "text-embedding-3-small");
        assert_eq!(response.embeddings().len(), 1);
        assert_eq!(response.embeddings()[0].dimensions(), 1536);
        assert_eq!(response.usage().prompt_tokens(), 10);
    }

    #[tokio::test]
    async fn test_embed_batch() {
        let mock_response = create_mock_response(3, 1536);
        let client = MockHttpClient::new().with_response(TEST_URL, mock_response);
        let provider = OpenAiEmbeddingProvider::new(client, "test-api-key");

        let request = EmbeddingRequest::batch(
            "text-embedding-3-small",
            vec!["Hello".into(), "World".into(), "Test".into()],
        );
        let response = provider.embed(request).await.unwrap();

        assert_eq!(response.embeddings().len(), 3);

        for (i, emb) in response.embeddings().iter().enumerate() {
            assert_eq!(emb.index(), i);
        }
    }

    #[tokio::test]
    async fn test_embed_with_custom_dimensions() {
        let mock_response = create_mock_response(1, 256);
        let client = MockHttpClient::new().with_response(TEST_URL, mock_response);
        let provider = OpenAiEmbeddingProvider::new(client, "test-api-key");

        let request =
            EmbeddingRequest::single("text-embedding-3-small", "Hello").with_dimensions(256);
        let response = provider.embed(request).await.unwrap();

        assert_eq!(response.embeddings()[0].dimensions(), 256);
    }

    #[tokio::test]
    async fn test_embed_error() {
        let client = MockHttpClient::new().with_error(TEST_URL, "Rate limit exceeded");
        let provider = OpenAiEmbeddingProvider::new(client, "test-api-key");

        let request = EmbeddingRequest::single("text-embedding-3-small", "Hello");
        let result = provider.embed(request).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_custom_base_url() {
        let custom_url = "http://localhost:8080/v1/embeddings";
        let mock_response = create_mock_response(1, 1536);
        let client = MockHttpClient::new().with_response(custom_url, mock_response);
        let provider =
            OpenAiEmbeddingProvider::with_base_url(client, "test-key", "http://localhost:8080");

        let request = EmbeddingRequest::single("text-embedding-3-small", "Test");
        let response = provider.embed(request).await.unwrap();

        assert_eq!(response.embeddings().len(), 1);
    }

    #[test]
    fn test_provider_info() {
        let client = MockHttpClient::new();
        let provider = OpenAiEmbeddingProvider::new(client, "test-key");

        assert_eq!(provider.provider_name(), "openai");
        assert_eq!(provider.default_model(), "text-embedding-3-small");
        assert_eq!(provider.dimensions("text-embedding-3-small"), Some(1536));
        assert_eq!(provider.dimensions("text-embedding-3-large"), Some(3072));
        assert_eq!(provider.dimensions("unknown-model"), None);
    }
}
