//! Model service - CRUD operations for model configuration

use std::sync::Arc;

use crate::domain::storage::Storage;
use crate::domain::{
    validate_model_config, CredentialType, DomainError, Model, ModelConfig, ModelId,
    ModelValidationError,
};

/// Request to create a new model
#[derive(Debug, Clone)]
pub struct CreateModelRequest {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub provider: CredentialType,
    pub provider_model: String,
    pub credential_id: String,
    pub config: Option<ModelConfig>,
    pub enabled: bool,
}

/// Request to update an existing model
#[derive(Debug, Clone)]
pub struct UpdateModelRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub provider_model: Option<String>,
    pub credential_id: Option<String>,
    pub config: Option<ModelConfig>,
    pub enabled: Option<bool>,
}

/// Model service for CRUD operations
#[derive(Debug)]
pub struct ModelService<S: Storage<Model>> {
    storage: Arc<S>,
}

impl<S: Storage<Model>> ModelService<S> {
    /// Create a new ModelService with the given storage
    pub fn new(storage: Arc<S>) -> Self {
        Self { storage }
    }

    /// Get a model by ID
    pub async fn get(&self, id: &str) -> Result<Option<Model>, DomainError> {
        let model_id = self.parse_model_id(id)?;
        self.storage.get(&model_id).await
    }

    /// Get a model by ID, returning an error if not found
    pub async fn get_required(&self, id: &str) -> Result<Model, DomainError> {
        self.get(id)
            .await?
            .ok_or_else(|| DomainError::not_found(format!("Model '{}' not found", id)))
    }

    /// List all models
    pub async fn list(&self) -> Result<Vec<Model>, DomainError> {
        self.storage.list().await
    }

    /// List all enabled models
    pub async fn list_enabled(&self) -> Result<Vec<Model>, DomainError> {
        let models = self.storage.list().await?;
        Ok(models.into_iter().filter(|m| m.is_enabled()).collect())
    }

    /// Create a new model
    pub async fn create(&self, request: CreateModelRequest) -> Result<Model, DomainError> {
        let model_id = self.parse_model_id(&request.id)?;

        // Check for duplicate
        if self.storage.exists(&model_id).await? {
            return Err(DomainError::conflict(format!(
                "Model with ID '{}' already exists",
                request.id
            )));
        }

        // Validate config if provided
        if let Some(ref config) = request.config {
            self.validate_config(config)?;
        }

        // Build the model
        let mut model = Model::new(
            model_id,
            request.name,
            request.provider,
            request.provider_model,
            request.credential_id,
        );

        if let Some(description) = request.description {
            model = model.with_description(description);
        }

        if let Some(config) = request.config {
            model = model.with_config(config);
        }

        model = model.with_enabled(request.enabled);

        self.storage.create(model).await
    }

    /// Update an existing model
    pub async fn update(
        &self,
        id: &str,
        request: UpdateModelRequest,
    ) -> Result<Model, DomainError> {
        let model_id = self.parse_model_id(id)?;

        // Get existing model
        let mut model = self
            .storage
            .get(&model_id)
            .await?
            .ok_or_else(|| DomainError::not_found(format!("Model '{}' not found", id)))?;

        // Apply updates
        if let Some(name) = request.name {
            model.set_name(name);
        }

        if let Some(description) = request.description {
            model.set_description(Some(description));
        }

        if let Some(provider_model) = request.provider_model {
            model.set_provider_model(provider_model);
        }

        if let Some(credential_id) = request.credential_id {
            model.set_credential_id(credential_id);
        }

        if let Some(config) = request.config {
            self.validate_config(&config)?;
            model.set_config(config);
        }

        if let Some(enabled) = request.enabled {
            model.set_enabled(enabled);
        }

        self.storage.update(model).await
    }

    /// Delete a model by ID
    pub async fn delete(&self, id: &str) -> Result<bool, DomainError> {
        let model_id = self.parse_model_id(id)?;
        self.storage.delete(&model_id).await
    }

    /// Enable a model
    pub async fn enable(&self, id: &str) -> Result<Model, DomainError> {
        self.update(id, UpdateModelRequest {
            name: None,
            description: None,
            provider_model: None,
            credential_id: None,
            config: None,
            enabled: Some(true),
        })
        .await
    }

    /// Disable a model
    pub async fn disable(&self, id: &str) -> Result<Model, DomainError> {
        self.update(id, UpdateModelRequest {
            name: None,
            description: None,
            provider_model: None,
            credential_id: None,
            config: None,
            enabled: Some(false),
        })
        .await
    }

    /// Parse and validate a model ID string
    fn parse_model_id(&self, id: &str) -> Result<ModelId, DomainError> {
        ModelId::new(id).map_err(|e| self.validation_error_to_domain(e))
    }

    /// Validate a model configuration
    fn validate_config(&self, config: &ModelConfig) -> Result<(), DomainError> {
        validate_model_config(config).map_err(|e| self.validation_error_to_domain(e))
    }

    /// Convert ModelValidationError to DomainError
    fn validation_error_to_domain(&self, error: ModelValidationError) -> DomainError {
        DomainError::validation(error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::storage::mock::MockStorage;

    fn create_service() -> ModelService<MockStorage<Model>> {
        ModelService::new(Arc::new(MockStorage::new()))
    }

    fn create_request(id: &str) -> CreateModelRequest {
        CreateModelRequest {
            id: id.to_string(),
            name: format!("Test Model {}", id),
            description: Some("A test model".to_string()),
            provider: CredentialType::OpenAi,
            provider_model: "gpt-4o".to_string(),
            credential_id: "openai-cred".to_string(),
            config: Some(ModelConfig::new().with_temperature(0.7)),
            enabled: true,
        }
    }

    #[tokio::test]
    async fn test_create_model() {
        let service = create_service();
        let request = create_request("test-model");

        let model = service.create(request).await.unwrap();

        assert_eq!(model.id().as_str(), "test-model");
        assert_eq!(model.name(), "Test Model test-model");
        assert_eq!(model.provider(), &CredentialType::OpenAi);
        assert_eq!(model.provider_model(), "gpt-4o");
        assert!(model.is_enabled());
    }

    #[tokio::test]
    async fn test_create_duplicate_model() {
        let service = create_service();
        let request = create_request("duplicate");

        service.create(request.clone()).await.unwrap();
        let result = service.create(request).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_create_model_invalid_id() {
        let service = create_service();
        let request = CreateModelRequest {
            id: "invalid_id!".to_string(),
            name: "Test".to_string(),
            description: None,
            provider: CredentialType::OpenAi,
            provider_model: "gpt-4".to_string(),
            credential_id: "openai-cred".to_string(),
            config: None,
            enabled: true,
        };

        let result = service.create(request).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_create_model_invalid_config() {
        let service = create_service();
        let request = CreateModelRequest {
            id: "valid-id".to_string(),
            name: "Test".to_string(),
            description: None,
            provider: CredentialType::OpenAi,
            provider_model: "gpt-4".to_string(),
            credential_id: "openai-cred".to_string(),
            config: Some(ModelConfig::new().with_temperature(5.0)), // Invalid temp
            enabled: true,
        };

        let result = service.create(request).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_model() {
        let service = create_service();
        let request = create_request("get-test");

        service.create(request).await.unwrap();

        let model = service.get("get-test").await.unwrap();
        assert!(model.is_some());
        assert_eq!(model.unwrap().id().as_str(), "get-test");
    }

    #[tokio::test]
    async fn test_get_model_not_found() {
        let service = create_service();

        let model = service.get("not-exists").await.unwrap();
        assert!(model.is_none());
    }

    #[tokio::test]
    async fn test_get_required_not_found() {
        let service = create_service();

        let result = service.get_required("not-exists").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_update_model() {
        let service = create_service();
        service.create(create_request("update-test")).await.unwrap();

        let update = UpdateModelRequest {
            name: Some("Updated Name".to_string()),
            description: None,
            provider_model: Some("gpt-4-turbo".to_string()),
            credential_id: None,
            config: Some(ModelConfig::new().with_temperature(0.5)),
            enabled: None,
        };

        let updated = service.update("update-test", update).await.unwrap();

        assert_eq!(updated.name(), "Updated Name");
        assert_eq!(updated.provider_model(), "gpt-4-turbo");
        assert_eq!(updated.config().temperature, Some(0.5));
        assert_eq!(updated.version(), 2); // Config change increments version
    }

    #[tokio::test]
    async fn test_update_model_not_found() {
        let service = create_service();

        let update = UpdateModelRequest {
            name: Some("Updated".to_string()),
            description: None,
            provider_model: None,
            credential_id: None,
            config: None,
            enabled: None,
        };

        let result = service.update("not-exists", update).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_delete_model() {
        let service = create_service();
        service.create(create_request("delete-test")).await.unwrap();

        let deleted = service.delete("delete-test").await.unwrap();
        assert!(deleted);

        let model = service.get("delete-test").await.unwrap();
        assert!(model.is_none());
    }

    #[tokio::test]
    async fn test_delete_model_not_found() {
        let service = create_service();

        let deleted = service.delete("not-exists").await.unwrap();
        assert!(!deleted);
    }

    #[tokio::test]
    async fn test_enable_disable_model() {
        let service = create_service();
        service.create(create_request("toggle-test")).await.unwrap();

        let disabled = service.disable("toggle-test").await.unwrap();
        assert!(!disabled.is_enabled());

        let enabled = service.enable("toggle-test").await.unwrap();
        assert!(enabled.is_enabled());
    }

    #[tokio::test]
    async fn test_list_models() {
        let service = create_service();
        service.create(create_request("list-1")).await.unwrap();
        service.create(create_request("list-2")).await.unwrap();

        let models = service.list().await.unwrap();
        assert_eq!(models.len(), 2);
    }

    #[tokio::test]
    async fn test_list_enabled_models() {
        let service = create_service();
        service.create(create_request("enabled-1")).await.unwrap();
        service.create(create_request("enabled-2")).await.unwrap();

        service.disable("enabled-1").await.unwrap();

        let enabled = service.list_enabled().await.unwrap();
        assert_eq!(enabled.len(), 1);
        assert_eq!(enabled[0].id().as_str(), "enabled-2");
    }
}
