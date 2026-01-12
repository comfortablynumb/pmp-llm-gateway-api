//! External API service for managing external API configurations

use std::collections::HashMap;
use std::sync::Arc;

use crate::domain::external_api::{ExternalApi, ExternalApiId};
use crate::domain::storage::Storage;
use crate::domain::DomainError;

/// Request to create a new external API
#[derive(Debug, Clone)]
pub struct CreateExternalApiRequest {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub base_url: String,
    pub base_headers: HashMap<String, String>,
}

/// Request to update an external API
#[derive(Debug, Clone)]
pub struct UpdateExternalApiRequest {
    pub name: Option<String>,
    pub description: Option<Option<String>>,
    pub base_url: Option<String>,
    pub base_headers: Option<HashMap<String, String>>,
    pub enabled: Option<bool>,
}

/// Trait for external API service operations (used by workflow executor)
#[async_trait::async_trait]
pub trait ExternalApiServiceTrait: Send + Sync + std::fmt::Debug {
    /// Get an external API by ID
    async fn get(&self, id: &str) -> Result<Option<ExternalApi>, DomainError>;
    /// List all external APIs
    async fn list(&self) -> Result<Vec<ExternalApi>, DomainError>;
}

/// Service for managing external API configurations
#[derive(Debug)]
pub struct ExternalApiService<S: Storage<ExternalApi>> {
    storage: Arc<S>,
}

impl<S: Storage<ExternalApi>> ExternalApiService<S> {
    /// Create a new external API service
    pub fn new(storage: Arc<S>) -> Self {
        Self { storage }
    }

    /// Create a new external API
    pub async fn create(
        &self,
        request: CreateExternalApiRequest,
    ) -> Result<ExternalApi, DomainError> {
        let id = ExternalApiId::new(&request.id)?;

        // Check if already exists
        if self.storage.get(&id).await?.is_some() {
            return Err(DomainError::conflict(format!(
                "External API '{}' already exists",
                request.id
            )));
        }

        let api = ExternalApi::builder(id, &request.name, &request.base_url)?
            .base_headers(request.base_headers)
            .build();

        let mut api = api;

        if let Some(desc) = request.description {
            api.set_description(Some(desc));
        }

        self.storage.create(api).await
    }

    /// Get an external API by ID
    pub async fn get(&self, id: &str) -> Result<Option<ExternalApi>, DomainError> {
        let api_id = ExternalApiId::new(id)?;
        self.storage.get(&api_id).await
    }

    /// List all external APIs
    pub async fn list(&self) -> Result<Vec<ExternalApi>, DomainError> {
        self.storage.list().await
    }

    /// Update an external API
    pub async fn update(
        &self,
        id: &str,
        request: UpdateExternalApiRequest,
    ) -> Result<ExternalApi, DomainError> {
        let api_id = ExternalApiId::new(id)?;

        let mut api = self
            .storage
            .get(&api_id)
            .await?
            .ok_or_else(|| DomainError::not_found(format!("External API '{}' not found", id)))?;

        api.update(
            request.name,
            request.description,
            request.base_url,
            request.base_headers,
            request.enabled,
        )?;

        self.storage.update(api).await
    }

    /// Delete an external API
    pub async fn delete(&self, id: &str) -> Result<(), DomainError> {
        let api_id = ExternalApiId::new(id)?;
        self.storage.delete(&api_id).await?;
        Ok(())
    }

    /// Check if an external API exists
    pub async fn exists(&self, id: &str) -> Result<bool, DomainError> {
        let api_id = ExternalApiId::new(id)?;
        self.storage.exists(&api_id).await
    }
}

#[async_trait::async_trait]
impl<S: Storage<ExternalApi> + 'static> ExternalApiServiceTrait for ExternalApiService<S> {
    async fn get(&self, id: &str) -> Result<Option<ExternalApi>, DomainError> {
        let api_id = ExternalApiId::new(id)?;
        self.storage.get(&api_id).await
    }

    async fn list(&self) -> Result<Vec<ExternalApi>, DomainError> {
        self.storage.list().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::storage::InMemoryStorage;

    fn create_service() -> ExternalApiService<InMemoryStorage<ExternalApi>> {
        ExternalApiService::new(Arc::new(InMemoryStorage::new()))
    }

    #[tokio::test]
    async fn test_create_external_api() {
        let service = create_service();

        let request = CreateExternalApiRequest {
            id: "test-api".to_string(),
            name: "Test API".to_string(),
            description: Some("A test API".to_string()),
            base_url: "https://api.example.com".to_string(),
            base_headers: HashMap::new(),
        };

        let api = service.create(request).await.unwrap();
        assert_eq!(api.id().as_str(), "test-api");
        assert_eq!(api.name(), "Test API");
        assert_eq!(api.base_url(), "https://api.example.com");
    }

    #[tokio::test]
    async fn test_create_duplicate_fails() {
        let service = create_service();

        let request = CreateExternalApiRequest {
            id: "test-api".to_string(),
            name: "Test API".to_string(),
            description: None,
            base_url: "https://api.example.com".to_string(),
            base_headers: HashMap::new(),
        };

        service.create(request.clone()).await.unwrap();
        let result = service.create(request).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_update_external_api() {
        let service = create_service();

        let request = CreateExternalApiRequest {
            id: "test-api".to_string(),
            name: "Test API".to_string(),
            description: None,
            base_url: "https://api.example.com".to_string(),
            base_headers: HashMap::new(),
        };
        service.create(request).await.unwrap();

        let update = UpdateExternalApiRequest {
            name: Some("Updated API".to_string()),
            description: Some(Some("Updated description".to_string())),
            base_url: Some("https://new-api.example.com".to_string()),
            base_headers: None,
            enabled: Some(false),
        };

        let api = service.update("test-api", update).await.unwrap();
        assert_eq!(api.name(), "Updated API");
        assert_eq!(api.description(), Some("Updated description"));
        assert_eq!(api.base_url(), "https://new-api.example.com");
        assert!(!api.is_enabled());
    }

    #[tokio::test]
    async fn test_delete_external_api() {
        let service = create_service();

        let request = CreateExternalApiRequest {
            id: "test-api".to_string(),
            name: "Test API".to_string(),
            description: None,
            base_url: "https://api.example.com".to_string(),
            base_headers: HashMap::new(),
        };
        service.create(request).await.unwrap();

        service.delete("test-api").await.unwrap();

        let api = service.get("test-api").await.unwrap();
        assert!(api.is_none());
    }
}
