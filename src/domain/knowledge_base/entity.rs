//! Knowledge base entity and related types

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::validation::{validate_knowledge_base_id, KnowledgeBaseValidationError};
use super::MetadataFilter;

/// Knowledge base identifier - alphanumeric + hyphens, max 50 characters
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct KnowledgeBaseId(String);

impl KnowledgeBaseId {
    /// Create a new KnowledgeBaseId after validation
    pub fn new(id: impl Into<String>) -> Result<Self, KnowledgeBaseValidationError> {
        let id = id.into();
        validate_knowledge_base_id(&id)?;
        Ok(Self(id))
    }

    /// Get the inner string value
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for KnowledgeBaseId {
    type Error = KnowledgeBaseValidationError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<KnowledgeBaseId> for String {
    fn from(id: KnowledgeBaseId) -> Self {
        id.0
    }
}

impl std::fmt::Display for KnowledgeBaseId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Type of knowledge base backend
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeBaseType {
    /// PostgreSQL with pgvector extension
    Pgvector,
    /// AWS Bedrock Knowledge Base
    AwsKnowledgeBase,
    /// Pinecone vector database
    Pinecone,
    /// Weaviate vector database
    Weaviate,
    /// Qdrant vector database
    Qdrant,
}

impl std::fmt::Display for KnowledgeBaseType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pgvector => write!(f, "pgvector"),
            Self::AwsKnowledgeBase => write!(f, "aws_knowledge_base"),
            Self::Pinecone => write!(f, "pinecone"),
            Self::Weaviate => write!(f, "weaviate"),
            Self::Qdrant => write!(f, "qdrant"),
        }
    }
}

/// Configuration for embedding generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    /// Model used for generating embeddings (e.g., "text-embedding-3-small")
    pub model: String,
    /// Embedding dimensions
    pub dimensions: u32,
    /// Provider for the embedding model (optional, inferred from model if not set)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
}

impl EmbeddingConfig {
    /// Create a new embedding configuration
    pub fn new(model: impl Into<String>, dimensions: u32) -> Self {
        Self {
            model: model.into(),
            dimensions,
            provider: None,
        }
    }

    /// Set the provider
    pub fn with_provider(mut self, provider: impl Into<String>) -> Self {
        self.provider = Some(provider.into());
        self
    }
}

/// Knowledge base configuration options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeBaseConfig {
    /// Default number of results to return
    #[serde(default = "default_top_k")]
    pub default_top_k: u32,
    /// Default similarity threshold (0.0 - 1.0)
    #[serde(default = "default_similarity_threshold")]
    pub default_similarity_threshold: f32,
    /// Whether to include embeddings in search results
    #[serde(default)]
    pub include_embeddings: bool,
    /// Whether to include metadata in search results
    #[serde(default = "default_true")]
    pub include_metadata: bool,
    /// Maximum content length to return per result
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_content_length: Option<usize>,
}

fn default_top_k() -> u32 {
    10
}

fn default_similarity_threshold() -> f32 {
    0.7
}

fn default_true() -> bool {
    true
}

impl Default for KnowledgeBaseConfig {
    fn default() -> Self {
        Self {
            default_top_k: default_top_k(),
            default_similarity_threshold: default_similarity_threshold(),
            include_embeddings: false,
            include_metadata: true,
            max_content_length: None,
        }
    }
}

impl KnowledgeBaseConfig {
    /// Create a new configuration with defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Set default top_k
    pub fn with_default_top_k(mut self, top_k: u32) -> Self {
        self.default_top_k = top_k;
        self
    }

    /// Set default similarity threshold
    pub fn with_default_similarity_threshold(mut self, threshold: f32) -> Self {
        self.default_similarity_threshold = threshold;
        self
    }

    /// Enable including embeddings in results
    pub fn with_include_embeddings(mut self, include: bool) -> Self {
        self.include_embeddings = include;
        self
    }

    /// Set max content length
    pub fn with_max_content_length(mut self, length: usize) -> Self {
        self.max_content_length = Some(length);
        self
    }
}

/// Knowledge base entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeBase {
    /// Unique identifier
    id: KnowledgeBaseId,
    /// Display name
    name: String,
    /// Description
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    /// Type of knowledge base backend
    kb_type: KnowledgeBaseType,
    /// Embedding configuration
    embedding: EmbeddingConfig,
    /// Knowledge base configuration
    config: KnowledgeBaseConfig,
    /// Provider-specific connection details (stored securely)
    #[serde(skip_serializing_if = "Option::is_none")]
    connection_config: Option<HashMap<String, String>>,
    /// Whether the knowledge base is enabled
    enabled: bool,
    /// Creation timestamp
    created_at: DateTime<Utc>,
    /// Last update timestamp
    updated_at: DateTime<Utc>,
}

impl KnowledgeBase {
    /// Create a new knowledge base
    pub fn new(
        id: KnowledgeBaseId,
        name: impl Into<String>,
        kb_type: KnowledgeBaseType,
        embedding: EmbeddingConfig,
    ) -> Self {
        let now = Utc::now();

        Self {
            id,
            name: name.into(),
            description: None,
            kb_type,
            embedding,
            config: KnowledgeBaseConfig::default(),
            connection_config: None,
            enabled: true,
            created_at: now,
            updated_at: now,
        }
    }

    /// Set description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set configuration
    pub fn with_config(mut self, config: KnowledgeBaseConfig) -> Self {
        self.config = config;
        self
    }

    /// Set connection config
    pub fn with_connection_config(mut self, config: HashMap<String, String>) -> Self {
        self.connection_config = Some(config);
        self
    }

    /// Set enabled state
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    // Getters

    pub fn id(&self) -> &KnowledgeBaseId {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    pub fn kb_type(&self) -> &KnowledgeBaseType {
        &self.kb_type
    }

    pub fn embedding(&self) -> &EmbeddingConfig {
        &self.embedding
    }

    pub fn config(&self) -> &KnowledgeBaseConfig {
        &self.config
    }

    pub fn connection_config(&self) -> Option<&HashMap<String, String>> {
        self.connection_config.as_ref()
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    // Mutators

    /// Update the name
    pub fn set_name(&mut self, name: impl Into<String>) {
        self.name = name.into();
        self.touch();
    }

    /// Update the description
    pub fn set_description(&mut self, description: Option<String>) {
        self.description = description;
        self.touch();
    }

    /// Update the configuration
    pub fn set_config(&mut self, config: KnowledgeBaseConfig) {
        self.config = config;
        self.touch();
    }

    /// Enable or disable
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        self.touch();
    }

    fn touch(&mut self) {
        self.updated_at = Utc::now();
    }
}

/// A single search result from a knowledge base
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// Unique identifier of the document/chunk
    pub id: String,
    /// Content text
    pub content: String,
    /// Similarity score (0.0 - 1.0, higher is more similar)
    pub score: f32,
    /// Document metadata
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, serde_json::Value>,
    /// Embedding vector (if requested)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding: Option<Vec<f32>>,
    /// Source document reference
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

impl SearchResult {
    /// Create a new search result
    pub fn new(id: impl Into<String>, content: impl Into<String>, score: f32) -> Self {
        Self {
            id: id.into(),
            content: content.into(),
            score,
            metadata: HashMap::new(),
            embedding: None,
            source: None,
        }
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    /// Set all metadata
    pub fn with_all_metadata(mut self, metadata: HashMap<String, serde_json::Value>) -> Self {
        self.metadata = metadata;
        self
    }

    /// Set embedding
    pub fn with_embedding(mut self, embedding: Vec<f32>) -> Self {
        self.embedding = Some(embedding);
        self
    }

    /// Set source
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }
}

/// Search query parameters
#[derive(Debug, Clone, Default)]
pub struct SearchQuery {
    /// Query text
    pub query: String,
    /// Number of results to return
    pub top_k: Option<u32>,
    /// Minimum similarity threshold
    pub similarity_threshold: Option<f32>,
    /// Metadata filters
    pub filter: Option<MetadataFilter>,
    /// Whether to include embeddings
    pub include_embeddings: bool,
}

impl SearchQuery {
    /// Create a new search query
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            ..Default::default()
        }
    }

    /// Set top_k
    pub fn with_top_k(mut self, top_k: u32) -> Self {
        self.top_k = Some(top_k);
        self
    }

    /// Set similarity threshold
    pub fn with_similarity_threshold(mut self, threshold: f32) -> Self {
        self.similarity_threshold = Some(threshold);
        self
    }

    /// Set metadata filter
    pub fn with_filter(mut self, filter: MetadataFilter) -> Self {
        self.filter = Some(filter);
        self
    }

    /// Include embeddings in results
    pub fn with_include_embeddings(mut self, include: bool) -> Self {
        self.include_embeddings = include;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_knowledge_base_id_valid() {
        let id = KnowledgeBaseId::new("my-kb-1").unwrap();
        assert_eq!(id.as_str(), "my-kb-1");
    }

    #[test]
    fn test_knowledge_base_id_invalid() {
        assert!(KnowledgeBaseId::new("").is_err());
        assert!(KnowledgeBaseId::new("my_kb").is_err());
        assert!(KnowledgeBaseId::new("-kb").is_err());
    }

    #[test]
    fn test_knowledge_base_creation() {
        let id = KnowledgeBaseId::new("product-docs").unwrap();
        let embedding = EmbeddingConfig::new("text-embedding-3-small", 1536);

        let kb = KnowledgeBase::new(id, "Product Documentation", KnowledgeBaseType::Pgvector, embedding)
            .with_description("Documentation for products")
            .with_config(
                KnowledgeBaseConfig::new()
                    .with_default_top_k(5)
                    .with_default_similarity_threshold(0.8),
            );

        assert_eq!(kb.name(), "Product Documentation");
        assert_eq!(kb.description(), Some("Documentation for products"));
        assert_eq!(kb.kb_type(), &KnowledgeBaseType::Pgvector);
        assert_eq!(kb.embedding().model, "text-embedding-3-small");
        assert_eq!(kb.embedding().dimensions, 1536);
        assert_eq!(kb.config().default_top_k, 5);
        assert!(kb.is_enabled());
    }

    #[test]
    fn test_knowledge_base_type_display() {
        assert_eq!(KnowledgeBaseType::Pgvector.to_string(), "pgvector");
        assert_eq!(
            KnowledgeBaseType::AwsKnowledgeBase.to_string(),
            "aws_knowledge_base"
        );
    }

    #[test]
    fn test_search_result() {
        let result = SearchResult::new("doc-1", "This is the content", 0.95)
            .with_metadata("category", serde_json::json!("technical"))
            .with_source("manual.pdf");

        assert_eq!(result.id, "doc-1");
        assert_eq!(result.content, "This is the content");
        assert_eq!(result.score, 0.95);
        assert_eq!(
            result.metadata.get("category"),
            Some(&serde_json::json!("technical"))
        );
        assert_eq!(result.source, Some("manual.pdf".to_string()));
    }

    #[test]
    fn test_search_query() {
        let query = SearchQuery::new("How do I reset my password?")
            .with_top_k(5)
            .with_similarity_threshold(0.8)
            .with_include_embeddings(false);

        assert_eq!(query.query, "How do I reset my password?");
        assert_eq!(query.top_k, Some(5));
        assert_eq!(query.similarity_threshold, Some(0.8));
        assert!(!query.include_embeddings);
    }

    #[test]
    fn test_embedding_config() {
        let config = EmbeddingConfig::new("text-embedding-3-large", 3072).with_provider("openai");

        assert_eq!(config.model, "text-embedding-3-large");
        assert_eq!(config.dimensions, 3072);
        assert_eq!(config.provider, Some("openai".to_string()));
    }
}
