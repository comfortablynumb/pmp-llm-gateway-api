//! Knowledge Base service - CRUD operations for knowledge base configuration

use std::sync::Arc;

use crate::domain::storage::Storage;
use crate::domain::{
    DomainError, EmbeddingConfig, KnowledgeBase, KnowledgeBaseConfig, KnowledgeBaseId,
    KnowledgeBaseType, KnowledgeBaseValidationError,
};

/// Request to create a new knowledge base
#[derive(Debug, Clone)]
pub struct CreateKnowledgeBaseRequest {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub kb_type: KnowledgeBaseType,
    pub embedding_model: String,
    pub embedding_dimensions: u32,
    pub credential_id: String,
    pub config: Option<KnowledgeBaseConfig>,
    pub enabled: bool,
}

/// Request to update an existing knowledge base
#[derive(Debug, Clone)]
pub struct UpdateKnowledgeBaseRequest {
    pub name: Option<String>,
    pub description: Option<Option<String>>,
    pub config: Option<KnowledgeBaseConfig>,
    pub enabled: Option<bool>,
}

/// Knowledge Base service for CRUD operations
pub struct KnowledgeBaseService {
    storage: Arc<dyn Storage<KnowledgeBase>>,
}

impl std::fmt::Debug for KnowledgeBaseService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KnowledgeBaseService").finish()
    }
}

impl KnowledgeBaseService {
    /// Create a new KnowledgeBaseService with the given storage
    pub fn new(storage: Arc<dyn Storage<KnowledgeBase>>) -> Self {
        Self { storage }
    }

    /// Get a knowledge base by ID
    pub async fn get(&self, id: &str) -> Result<Option<KnowledgeBase>, DomainError> {
        let kb_id = self.parse_kb_id(id)?;
        self.storage.get(&kb_id).await
    }

    /// Get a knowledge base by ID, returning an error if not found
    pub async fn get_required(&self, id: &str) -> Result<KnowledgeBase, DomainError> {
        self.get(id)
            .await?
            .ok_or_else(|| DomainError::not_found(format!("Knowledge base '{}' not found", id)))
    }

    /// List all knowledge bases
    pub async fn list(&self) -> Result<Vec<KnowledgeBase>, DomainError> {
        self.storage.list().await
    }

    /// Create a new knowledge base
    pub async fn create(
        &self,
        request: CreateKnowledgeBaseRequest,
    ) -> Result<KnowledgeBase, DomainError> {
        let kb_id = self.parse_kb_id(&request.id)?;

        // Check for duplicate
        if self.storage.exists(&kb_id).await? {
            return Err(DomainError::conflict(format!(
                "Knowledge base with ID '{}' already exists",
                request.id
            )));
        }

        // Create embedding config
        let embedding = EmbeddingConfig::new(request.embedding_model, request.embedding_dimensions);

        // Build the knowledge base
        let mut kb = KnowledgeBase::new(kb_id, request.name, request.kb_type, embedding);

        if let Some(description) = request.description {
            kb = kb.with_description(description);
        }

        if let Some(config) = request.config {
            kb = kb.with_config(config);
        }

        // Store credential_id in connection_config
        let mut connection_config = std::collections::HashMap::new();
        connection_config.insert("credential_id".to_string(), request.credential_id);
        kb = kb.with_connection_config(connection_config);
        kb = kb.with_enabled(request.enabled);

        self.storage.save(kb.clone()).await?;
        Ok(kb)
    }

    /// Update an existing knowledge base
    pub async fn update(
        &self,
        id: &str,
        request: UpdateKnowledgeBaseRequest,
    ) -> Result<KnowledgeBase, DomainError> {
        let kb_id = self.parse_kb_id(id)?;

        // Get existing knowledge base
        let mut kb = self
            .storage
            .get(&kb_id)
            .await?
            .ok_or_else(|| DomainError::not_found(format!("Knowledge base '{}' not found", id)))?;

        // Apply updates
        if let Some(name) = request.name {
            kb.set_name(name);
        }

        if let Some(description) = request.description {
            kb.set_description(description);
        }

        if let Some(config) = request.config {
            kb.set_config(config);
        }

        if let Some(enabled) = request.enabled {
            kb.set_enabled(enabled);
        }

        self.storage.save(kb.clone()).await?;
        Ok(kb)
    }

    /// Delete a knowledge base by ID
    pub async fn delete(&self, id: &str) -> Result<bool, DomainError> {
        let kb_id = self.parse_kb_id(id)?;
        self.storage.delete(&kb_id).await
    }

    /// Check if a knowledge base exists
    pub async fn exists(&self, id: &str) -> Result<bool, DomainError> {
        let kb_id = self.parse_kb_id(id)?;
        self.storage.exists(&kb_id).await
    }

    /// Parse and validate a knowledge base ID string
    fn parse_kb_id(&self, id: &str) -> Result<KnowledgeBaseId, DomainError> {
        KnowledgeBaseId::new(id).map_err(|e| self.validation_error_to_domain(e))
    }

    /// Convert KnowledgeBaseValidationError to DomainError
    fn validation_error_to_domain(&self, error: KnowledgeBaseValidationError) -> DomainError {
        DomainError::validation(error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::storage::mock::MockStorage;

    fn create_service() -> KnowledgeBaseService {
        KnowledgeBaseService::new(Arc::new(MockStorage::new()))
    }

    fn create_request(id: &str) -> CreateKnowledgeBaseRequest {
        CreateKnowledgeBaseRequest {
            id: id.to_string(),
            name: format!("Test KB {}", id),
            description: Some("A test knowledge base".to_string()),
            kb_type: KnowledgeBaseType::Pgvector,
            embedding_model: "text-embedding-3-small".to_string(),
            embedding_dimensions: 1536,
            credential_id: "pgvector-cred".to_string(),
            config: Some(
                KnowledgeBaseConfig::new()
                    .with_default_top_k(10)
                    .with_default_similarity_threshold(0.7),
            ),
            enabled: true,
        }
    }

    #[tokio::test]
    async fn test_create_knowledge_base() {
        let service = create_service();
        let request = create_request("test-kb");

        let kb = service.create(request).await.unwrap();

        assert_eq!(kb.id().as_str(), "test-kb");
        assert_eq!(kb.name(), "Test KB test-kb");
        assert_eq!(kb.kb_type(), &KnowledgeBaseType::Pgvector);
        assert_eq!(kb.embedding().model, "text-embedding-3-small");
        assert_eq!(kb.embedding().dimensions, 1536);
        assert!(kb.is_enabled());
    }

    #[tokio::test]
    async fn test_create_duplicate_knowledge_base() {
        let service = create_service();
        let request = create_request("duplicate");

        service.create(request.clone()).await.unwrap();
        let result = service.create(request).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_knowledge_base() {
        let service = create_service();
        let request = create_request("get-test");

        service.create(request).await.unwrap();

        let kb = service.get("get-test").await.unwrap();
        assert!(kb.is_some());
        assert_eq!(kb.unwrap().id().as_str(), "get-test");
    }

    #[tokio::test]
    async fn test_get_knowledge_base_not_found() {
        let service = create_service();

        let kb = service.get("not-exists").await.unwrap();
        assert!(kb.is_none());
    }

    #[tokio::test]
    async fn test_update_knowledge_base() {
        let service = create_service();
        service.create(create_request("update-test")).await.unwrap();

        let update = UpdateKnowledgeBaseRequest {
            name: Some("Updated Name".to_string()),
            description: None,
            config: Some(
                KnowledgeBaseConfig::new()
                    .with_default_top_k(5)
                    .with_default_similarity_threshold(0.8),
            ),
            enabled: None,
        };

        let updated = service.update("update-test", update).await.unwrap();

        assert_eq!(updated.name(), "Updated Name");
        assert_eq!(updated.config().default_top_k, 5);
    }

    #[tokio::test]
    async fn test_delete_knowledge_base() {
        let service = create_service();
        service.create(create_request("delete-test")).await.unwrap();

        let deleted = service.delete("delete-test").await.unwrap();
        assert!(deleted);

        let kb = service.get("delete-test").await.unwrap();
        assert!(kb.is_none());
    }

    #[tokio::test]
    async fn test_list_knowledge_bases() {
        let service = create_service();
        service.create(create_request("list-1")).await.unwrap();
        service.create(create_request("list-2")).await.unwrap();

        let kbs = service.list().await.unwrap();
        assert_eq!(kbs.len(), 2);
    }
}
