//! Plugin extension types
//!
//! Defines the types of extensions that plugins can provide.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Types of extensions that a plugin can provide
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtensionType {
    /// LLM provider for chat completions
    LlmProvider,

    /// Embedding provider for vector generation
    EmbeddingProvider,

    /// Knowledge base provider for RAG
    KnowledgeBaseProvider,

    /// Credential provider for secrets management
    CredentialProvider,

    /// Storage backend for persistence
    StorageBackend,
}

impl ExtensionType {
    /// Get all extension types as a slice
    pub fn all() -> &'static [ExtensionType] {
        &[
            ExtensionType::LlmProvider,
            ExtensionType::EmbeddingProvider,
            ExtensionType::KnowledgeBaseProvider,
            ExtensionType::CredentialProvider,
            ExtensionType::StorageBackend,
        ]
    }

    /// Get the string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            ExtensionType::LlmProvider => "llm_provider",
            ExtensionType::EmbeddingProvider => "embedding_provider",
            ExtensionType::KnowledgeBaseProvider => "knowledge_base_provider",
            ExtensionType::CredentialProvider => "credential_provider",
            ExtensionType::StorageBackend => "storage_backend",
        }
    }
}

impl fmt::Display for ExtensionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl TryFrom<&str> for ExtensionType {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "llm_provider" => Ok(ExtensionType::LlmProvider),
            "embedding_provider" => Ok(ExtensionType::EmbeddingProvider),
            "knowledge_base_provider" => Ok(ExtensionType::KnowledgeBaseProvider),
            "credential_provider" => Ok(ExtensionType::CredentialProvider),
            "storage_backend" => Ok(ExtensionType::StorageBackend),
            _ => Err(format!("Unknown extension type: {}", value)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extension_type_as_str() {
        assert_eq!(ExtensionType::LlmProvider.as_str(), "llm_provider");
        assert_eq!(
            ExtensionType::EmbeddingProvider.as_str(),
            "embedding_provider"
        );
        assert_eq!(
            ExtensionType::KnowledgeBaseProvider.as_str(),
            "knowledge_base_provider"
        );
        assert_eq!(
            ExtensionType::CredentialProvider.as_str(),
            "credential_provider"
        );
        assert_eq!(ExtensionType::StorageBackend.as_str(), "storage_backend");
    }

    #[test]
    fn test_extension_type_display() {
        assert_eq!(format!("{}", ExtensionType::LlmProvider), "llm_provider");
    }

    #[test]
    fn test_extension_type_try_from() {
        assert_eq!(
            ExtensionType::try_from("llm_provider").unwrap(),
            ExtensionType::LlmProvider
        );
        assert_eq!(
            ExtensionType::try_from("embedding_provider").unwrap(),
            ExtensionType::EmbeddingProvider
        );
        assert!(ExtensionType::try_from("unknown").is_err());
    }

    #[test]
    fn test_extension_type_all() {
        let all = ExtensionType::all();
        assert_eq!(all.len(), 5);
        assert!(all.contains(&ExtensionType::LlmProvider));
        assert!(all.contains(&ExtensionType::StorageBackend));
    }

    #[test]
    fn test_extension_type_serialization() {
        let ext = ExtensionType::LlmProvider;
        let json = serde_json::to_string(&ext).unwrap();
        assert_eq!(json, "\"llm_provider\"");

        let deserialized: ExtensionType = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, ExtensionType::LlmProvider);
    }
}
