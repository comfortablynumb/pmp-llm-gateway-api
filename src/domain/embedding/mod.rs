//! Embedding provider domain models and traits

mod provider;
mod request;
mod response;

pub use provider::EmbeddingProvider;
pub use request::{EmbeddingInput, EmbeddingRequest};
pub use response::{cosine_similarity, Embedding, EmbeddingResponse, EmbeddingUsage};

#[cfg(test)]
pub use provider::mock::MockEmbeddingProvider;
