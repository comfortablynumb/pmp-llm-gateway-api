//! Knowledge Base Provider plugin trait
//!
//! Defines the interface for plugins that provide knowledge base capabilities.

use super::entity::Plugin;
use super::error::PluginError;
use crate::domain::credentials::CredentialType;
use crate::domain::knowledge_base::{KnowledgeBaseId, KnowledgeBaseProvider, KnowledgeBaseType};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

/// Configuration for creating a knowledge base provider instance
#[derive(Debug, Clone)]
pub struct KnowledgeBaseProviderConfig {
    /// The knowledge base ID
    pub knowledge_base_id: KnowledgeBaseId,

    /// The knowledge base type
    pub knowledge_base_type: KnowledgeBaseType,

    /// The credential type for authentication
    pub credential_type: CredentialType,

    /// Credential ID for looking up credentials
    pub credential_id: String,

    /// Connection string or endpoint
    pub connection_string: Option<String>,

    /// Additional parameters
    pub additional_params: HashMap<String, String>,
}

impl KnowledgeBaseProviderConfig {
    pub fn new(
        knowledge_base_id: KnowledgeBaseId,
        knowledge_base_type: KnowledgeBaseType,
        credential_type: CredentialType,
        credential_id: impl Into<String>,
    ) -> Self {
        Self {
            knowledge_base_id,
            knowledge_base_type,
            credential_type,
            credential_id: credential_id.into(),
            connection_string: None,
            additional_params: HashMap::new(),
        }
    }

    pub fn with_connection_string(mut self, connection_string: impl Into<String>) -> Self {
        self.connection_string = Some(connection_string.into());
        self
    }

    pub fn with_param(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.additional_params.insert(key.into(), value.into());
        self
    }

    /// Get a parameter value
    pub fn get_param(&self, key: &str) -> Option<&String> {
        self.additional_params.get(key)
    }
}

/// Trait for plugins that provide knowledge base capabilities
#[async_trait]
pub trait KnowledgeBaseProviderPlugin: Plugin {
    /// Get the knowledge base types this plugin supports
    fn supported_knowledge_base_types(&self) -> Vec<KnowledgeBaseType>;

    /// Check if this plugin supports the given knowledge base type
    fn supports_knowledge_base_type(&self, kb_type: &KnowledgeBaseType) -> bool {
        self.supported_knowledge_base_types().contains(kb_type)
    }

    /// Get the credential types this plugin supports
    fn supported_credential_types(&self) -> Vec<CredentialType>;

    /// Check if this plugin supports the given credential type
    fn supports_credential_type(&self, credential_type: &CredentialType) -> bool {
        self.supported_credential_types().contains(credential_type)
    }

    /// Create a knowledge base provider instance with the given configuration
    ///
    /// # Arguments
    /// * `config` - Configuration containing knowledge base settings
    ///
    /// # Returns
    /// * `Ok(Arc<dyn KnowledgeBaseProvider>)` - The created provider instance
    /// * `Err(PluginError)` - If provider creation fails
    async fn create_knowledge_base_provider(
        &self,
        config: KnowledgeBaseProviderConfig,
    ) -> Result<Arc<dyn KnowledgeBaseProvider>, PluginError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_knowledge_base_config_builder() {
        let kb_id = KnowledgeBaseId::try_from("test-kb".to_string()).unwrap();

        let config = KnowledgeBaseProviderConfig::new(
            kb_id.clone(),
            KnowledgeBaseType::Pgvector,
            CredentialType::Pgvector,
            "pgvector-default",
        )
        .with_connection_string("postgresql://localhost:5432/vectors")
        .with_param("table_name", "embeddings");

        assert_eq!(config.knowledge_base_id, kb_id);
        assert!(matches!(
            config.knowledge_base_type,
            KnowledgeBaseType::Pgvector
        ));
        assert_eq!(
            config.connection_string,
            Some("postgresql://localhost:5432/vectors".to_string())
        );
        assert_eq!(config.get_param("table_name"), Some(&"embeddings".to_string()));
    }
}
