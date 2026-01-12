//! Embedding provider implementations

mod openai;

pub use openai::OpenAiEmbeddingProvider;

// Re-export HTTP client for use by embedding providers
pub use super::llm::{HttpClient, HttpClientTrait};
