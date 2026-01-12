//! Embedding response types

use serde::{Deserialize, Serialize};

/// A single embedding vector
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Embedding {
    /// Index of this embedding in the batch
    index: usize,
    /// The embedding vector
    embedding: Vec<f32>,
}

impl Embedding {
    /// Create a new embedding
    pub fn new(index: usize, embedding: Vec<f32>) -> Self {
        Self { index, embedding }
    }

    /// Get the index
    pub fn index(&self) -> usize {
        self.index
    }

    /// Get the embedding vector
    pub fn vector(&self) -> &[f32] {
        &self.embedding
    }

    /// Get the embedding dimensions
    pub fn dimensions(&self) -> usize {
        self.embedding.len()
    }

    /// Consume and return the vector
    pub fn into_vector(self) -> Vec<f32> {
        self.embedding
    }

    /// Calculate cosine similarity with another embedding
    pub fn cosine_similarity(&self, other: &Embedding) -> f32 {
        cosine_similarity(&self.embedding, &other.embedding)
    }

    /// Calculate cosine similarity with a raw vector
    pub fn cosine_similarity_vec(&self, other: &[f32]) -> f32 {
        cosine_similarity(&self.embedding, other)
    }
}

/// Calculate cosine similarity between two vectors
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    dot_product / (norm_a * norm_b)
}

/// Usage statistics for embedding request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingUsage {
    /// Number of prompt tokens
    prompt_tokens: u32,
    /// Total tokens used
    total_tokens: u32,
}

impl EmbeddingUsage {
    /// Create new usage stats
    pub fn new(prompt_tokens: u32, total_tokens: u32) -> Self {
        Self {
            prompt_tokens,
            total_tokens,
        }
    }

    /// Get prompt tokens
    pub fn prompt_tokens(&self) -> u32 {
        self.prompt_tokens
    }

    /// Get total tokens
    pub fn total_tokens(&self) -> u32 {
        self.total_tokens
    }
}

/// Response from an embedding request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingResponse {
    /// Model used
    model: String,
    /// Generated embeddings
    data: Vec<Embedding>,
    /// Usage statistics
    usage: EmbeddingUsage,
}

impl EmbeddingResponse {
    /// Create a new embedding response
    pub fn new(model: String, data: Vec<Embedding>, usage: EmbeddingUsage) -> Self {
        Self { model, data, usage }
    }

    /// Get the model used
    pub fn model(&self) -> &str {
        &self.model
    }

    /// Get all embeddings
    pub fn embeddings(&self) -> &[Embedding] {
        &self.data
    }

    /// Get the first embedding (for single input requests)
    pub fn first(&self) -> Option<&Embedding> {
        self.data.first()
    }

    /// Get usage statistics
    pub fn usage(&self) -> &EmbeddingUsage {
        &self.usage
    }

    /// Get embedding at index
    pub fn get(&self, index: usize) -> Option<&Embedding> {
        self.data.get(index)
    }

    /// Consume and return embeddings
    pub fn into_embeddings(self) -> Vec<Embedding> {
        self.data
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedding_creation() {
        let emb = Embedding::new(0, vec![0.1, 0.2, 0.3]);

        assert_eq!(emb.index(), 0);
        assert_eq!(emb.dimensions(), 3);
        assert_eq!(emb.vector(), &[0.1, 0.2, 0.3]);
    }

    #[test]
    fn test_cosine_similarity_identical() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];

        let similarity = cosine_similarity(&a, &b);

        assert!((similarity - 1.0).abs() < 0.0001);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];

        let similarity = cosine_similarity(&a, &b);

        assert!(similarity.abs() < 0.0001);
    }

    #[test]
    fn test_cosine_similarity_opposite() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![-1.0, 0.0, 0.0];

        let similarity = cosine_similarity(&a, &b);

        assert!((similarity + 1.0).abs() < 0.0001);
    }

    #[test]
    fn test_cosine_similarity_similar() {
        let a = vec![1.0, 1.0, 0.0];
        let b = vec![1.0, 0.9, 0.1];

        let similarity = cosine_similarity(&a, &b);

        assert!(similarity > 0.9);
    }

    #[test]
    fn test_embedding_cosine_similarity() {
        let emb1 = Embedding::new(0, vec![1.0, 0.0]);
        let emb2 = Embedding::new(1, vec![0.707, 0.707]);

        let similarity = emb1.cosine_similarity(&emb2);

        assert!(similarity > 0.7 && similarity < 0.72);
    }

    #[test]
    fn test_embedding_response() {
        let embeddings = vec![
            Embedding::new(0, vec![0.1, 0.2]),
            Embedding::new(1, vec![0.3, 0.4]),
        ];
        let usage = EmbeddingUsage::new(10, 10);
        let response = EmbeddingResponse::new("test-model".into(), embeddings, usage);

        assert_eq!(response.model(), "test-model");
        assert_eq!(response.embeddings().len(), 2);
        assert_eq!(response.usage().total_tokens(), 10);
    }

    #[test]
    fn test_cosine_similarity_empty() {
        let empty: Vec<f32> = vec![];
        let non_empty = vec![1.0, 2.0];

        assert_eq!(cosine_similarity(&empty, &non_empty), 0.0);
    }

    #[test]
    fn test_cosine_similarity_different_lengths() {
        let a = vec![1.0, 2.0];
        let b = vec![1.0, 2.0, 3.0];

        assert_eq!(cosine_similarity(&a, &b), 0.0);
    }
}
