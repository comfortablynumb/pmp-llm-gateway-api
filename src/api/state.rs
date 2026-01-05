//! Application state for shared services

use std::sync::Arc;

use serde_json::Value;

use crate::domain::api_key::{ApiKeyPermissions, ApiKeyRepository};
use crate::domain::llm::LlmProvider;
use crate::domain::model::ModelRepository;
use crate::domain::operation::OperationRepository;
use crate::domain::prompt::PromptRepository;
use crate::domain::workflow::WorkflowRepository;
use crate::domain::{
    ApiKey, DomainError, Model, Operation, OperationType, Prompt, Workflow, WorkflowExecutor,
    WorkflowResult,
};
use crate::infrastructure::api_key::ApiKeyService;
use crate::infrastructure::services::{
    CreateModelRequest, CreatePromptRequest, CreateWorkflowRequest, ModelService, OperationService,
    PromptService, UpdateModelRequest, UpdatePromptRequest, UpdateWorkflowRequest, WorkflowService,
};

/// Application state containing shared services using dynamic dispatch
#[derive(Clone)]
pub struct AppState {
    pub model_service: Arc<dyn ModelServiceTrait>,
    pub prompt_service: Arc<dyn PromptServiceTrait>,
    pub api_key_service: Arc<dyn ApiKeyServiceTrait>,
    pub workflow_service: Arc<dyn WorkflowServiceTrait>,
    pub operation_service: Arc<dyn OperationServiceTrait>,
    pub llm_provider: Arc<dyn LlmProvider>,
}

/// Trait for model service operations
#[async_trait::async_trait]
pub trait ModelServiceTrait: Send + Sync {
    async fn get(&self, id: &str) -> Result<Option<Model>, DomainError>;
    async fn list(&self) -> Result<Vec<Model>, DomainError>;
    async fn create(&self, request: CreateModelRequest) -> Result<Model, DomainError>;
    async fn update(&self, id: &str, request: UpdateModelRequest) -> Result<Model, DomainError>;
    async fn delete(&self, id: &str) -> Result<bool, DomainError>;
}

/// Trait for prompt service operations
#[async_trait::async_trait]
pub trait PromptServiceTrait: Send + Sync {
    async fn get(&self, id: &str) -> Result<Option<Prompt>, DomainError>;
    async fn list(&self) -> Result<Vec<Prompt>, DomainError>;
    async fn create(&self, request: CreatePromptRequest) -> Result<Prompt, DomainError>;
    async fn update(&self, id: &str, request: UpdatePromptRequest) -> Result<Prompt, DomainError>;
    async fn delete(&self, id: &str) -> Result<bool, DomainError>;
    async fn render(
        &self,
        id: &str,
        variables: &std::collections::HashMap<String, String>,
    ) -> Result<String, DomainError>;
}

/// Trait for workflow service operations
#[async_trait::async_trait]
pub trait WorkflowServiceTrait: Send + Sync {
    async fn get(&self, id: &str) -> Result<Option<Workflow>, DomainError>;
    async fn list(&self) -> Result<Vec<Workflow>, DomainError>;
    async fn create(&self, request: CreateWorkflowRequest) -> Result<Workflow, DomainError>;
    async fn update(&self, id: &str, request: UpdateWorkflowRequest) -> Result<Workflow, DomainError>;
    async fn delete(&self, id: &str) -> Result<bool, DomainError>;
    async fn execute(&self, id: &str, input: Value) -> Result<WorkflowResult, DomainError>;
}

/// Trait for API key service operations
#[async_trait::async_trait]
pub trait ApiKeyServiceTrait: Send + Sync {
    async fn validate(&self, key: &str) -> Result<Option<ApiKey>, DomainError>;
    async fn get(&self, id: &str) -> Result<Option<ApiKey>, DomainError>;
    async fn list(&self) -> Result<Vec<ApiKey>, DomainError>;
    async fn create(
        &self,
        name: &str,
        permissions: ApiKeyPermissions,
    ) -> Result<(ApiKey, String), DomainError>;
    async fn update_permissions(
        &self,
        id: &str,
        permissions: ApiKeyPermissions,
    ) -> Result<(), DomainError>;
    async fn delete(&self, id: &str) -> Result<(), DomainError>;
    async fn suspend(&self, id: &str) -> Result<(), DomainError>;
    async fn activate(&self, id: &str) -> Result<(), DomainError>;
    async fn revoke(&self, id: &str) -> Result<(), DomainError>;
}

/// Trait for operation service (async operations)
#[async_trait::async_trait]
pub trait OperationServiceTrait: Send + Sync {
    /// Create a new pending operation
    async fn create_pending(
        &self,
        op_type: OperationType,
        input: Value,
        metadata: Value,
    ) -> Result<Operation, DomainError>;
    /// Get an operation by ID
    async fn get(&self, id: &str) -> Result<Option<Operation>, DomainError>;
    /// Get multiple operations by IDs
    async fn get_batch(&self, ids: &[String]) -> Result<Vec<Operation>, DomainError>;
    /// Mark an operation as running
    async fn mark_running(&self, id: &str) -> Result<Operation, DomainError>;
    /// Mark an operation as completed with result
    async fn mark_completed(&self, id: &str, result: Value) -> Result<Operation, DomainError>;
    /// Mark an operation as failed with error message
    async fn mark_failed(&self, id: &str, error: String) -> Result<Operation, DomainError>;
    /// Cancel an operation
    async fn cancel(&self, id: &str) -> Result<Operation, DomainError>;
    /// Clean up old completed operations
    async fn cleanup_old(&self) -> Result<u64, DomainError>;
}

// Implement traits for the actual services

#[async_trait::async_trait]
impl<R: ModelRepository + 'static> ModelServiceTrait for ModelService<R> {
    async fn get(&self, id: &str) -> Result<Option<Model>, DomainError> {
        ModelService::get(self, id).await
    }

    async fn list(&self) -> Result<Vec<Model>, DomainError> {
        ModelService::list(self).await
    }

    async fn create(&self, request: CreateModelRequest) -> Result<Model, DomainError> {
        ModelService::create(self, request).await
    }

    async fn update(&self, id: &str, request: UpdateModelRequest) -> Result<Model, DomainError> {
        ModelService::update(self, id, request).await
    }

    async fn delete(&self, id: &str) -> Result<bool, DomainError> {
        ModelService::delete(self, id).await
    }
}

#[async_trait::async_trait]
impl<R: PromptRepository + 'static> PromptServiceTrait for PromptService<R> {
    async fn get(&self, id: &str) -> Result<Option<Prompt>, DomainError> {
        PromptService::get(self, id).await
    }

    async fn list(&self) -> Result<Vec<Prompt>, DomainError> {
        PromptService::list(self).await
    }

    async fn create(&self, request: CreatePromptRequest) -> Result<Prompt, DomainError> {
        PromptService::create(self, request).await
    }

    async fn update(&self, id: &str, request: UpdatePromptRequest) -> Result<Prompt, DomainError> {
        PromptService::update(self, id, request).await
    }

    async fn delete(&self, id: &str) -> Result<bool, DomainError> {
        PromptService::delete(self, id).await
    }

    async fn render(
        &self,
        id: &str,
        variables: &std::collections::HashMap<String, String>,
    ) -> Result<String, DomainError> {
        PromptService::render_by_id(self, id, variables.clone()).await
    }
}

#[async_trait::async_trait]
impl<R: ApiKeyRepository + 'static> ApiKeyServiceTrait for ApiKeyService<R> {
    async fn validate(&self, key: &str) -> Result<Option<ApiKey>, DomainError> {
        ApiKeyService::validate(self, key).await
    }

    async fn get(&self, id: &str) -> Result<Option<ApiKey>, DomainError> {
        let key_id = crate::domain::api_key::ApiKeyId::new(id)
            .map_err(|e| DomainError::validation(e.to_string()))?;
        ApiKeyService::get(self, &key_id).await
    }

    async fn list(&self) -> Result<Vec<ApiKey>, DomainError> {
        ApiKeyService::list(self, None).await
    }

    async fn create(
        &self,
        name: &str,
        permissions: ApiKeyPermissions,
    ) -> Result<(ApiKey, String), DomainError> {
        // Generate a new API key ID using UUID
        let uuid = uuid::Uuid::new_v4().to_string();
        let id = crate::domain::api_key::ApiKeyId::new(&uuid)
            .map_err(|e| DomainError::validation(e.to_string()))?;
        let result = ApiKeyService::create(self, id, name, permissions, None).await?;
        Ok((result.api_key, result.secret))
    }

    async fn update_permissions(
        &self,
        id: &str,
        permissions: ApiKeyPermissions,
    ) -> Result<(), DomainError> {
        let key_id = crate::domain::api_key::ApiKeyId::new(id)
            .map_err(|e| DomainError::validation(e.to_string()))?;
        ApiKeyService::update_permissions(self, &key_id, permissions).await?;
        Ok(())
    }

    async fn delete(&self, id: &str) -> Result<(), DomainError> {
        let key_id = crate::domain::api_key::ApiKeyId::new(id)
            .map_err(|e| DomainError::validation(e.to_string()))?;
        ApiKeyService::delete(self, &key_id).await?;
        Ok(())
    }

    async fn suspend(&self, id: &str) -> Result<(), DomainError> {
        let key_id = crate::domain::api_key::ApiKeyId::new(id)
            .map_err(|e| DomainError::validation(e.to_string()))?;
        ApiKeyService::suspend(self, &key_id).await?;
        Ok(())
    }

    async fn activate(&self, id: &str) -> Result<(), DomainError> {
        let key_id = crate::domain::api_key::ApiKeyId::new(id)
            .map_err(|e| DomainError::validation(e.to_string()))?;
        ApiKeyService::activate(self, &key_id).await?;
        Ok(())
    }

    async fn revoke(&self, id: &str) -> Result<(), DomainError> {
        let key_id = crate::domain::api_key::ApiKeyId::new(id)
            .map_err(|e| DomainError::validation(e.to_string()))?;
        ApiKeyService::revoke(self, &key_id).await?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl<R: WorkflowRepository + 'static, E: WorkflowExecutor + 'static> WorkflowServiceTrait
    for WorkflowService<R, E>
{
    async fn get(&self, id: &str) -> Result<Option<Workflow>, DomainError> {
        WorkflowService::get(self, id).await
    }

    async fn list(&self) -> Result<Vec<Workflow>, DomainError> {
        WorkflowService::list(self).await
    }

    async fn create(&self, request: CreateWorkflowRequest) -> Result<Workflow, DomainError> {
        WorkflowService::create(self, request).await
    }

    async fn update(&self, id: &str, request: UpdateWorkflowRequest) -> Result<Workflow, DomainError> {
        WorkflowService::update(self, id, request).await
    }

    async fn delete(&self, id: &str) -> Result<bool, DomainError> {
        WorkflowService::delete(self, id).await
    }

    async fn execute(&self, id: &str, input: Value) -> Result<WorkflowResult, DomainError> {
        WorkflowService::execute(self, id, input).await
    }
}

#[async_trait::async_trait]
impl<R: OperationRepository + 'static> OperationServiceTrait for OperationService<R> {
    async fn create_pending(
        &self,
        op_type: OperationType,
        input: Value,
        metadata: Value,
    ) -> Result<Operation, DomainError> {
        OperationService::create_pending(self, op_type, input, metadata).await
    }

    async fn get(&self, id: &str) -> Result<Option<Operation>, DomainError> {
        OperationService::get(self, id).await
    }

    async fn get_batch(&self, ids: &[String]) -> Result<Vec<Operation>, DomainError> {
        OperationService::get_batch(self, ids).await
    }

    async fn mark_running(&self, id: &str) -> Result<Operation, DomainError> {
        OperationService::mark_running(self, id).await
    }

    async fn mark_completed(&self, id: &str, result: Value) -> Result<Operation, DomainError> {
        OperationService::mark_completed(self, id, result).await
    }

    async fn mark_failed(&self, id: &str, error: String) -> Result<Operation, DomainError> {
        OperationService::mark_failed(self, id, error).await
    }

    async fn cancel(&self, id: &str) -> Result<Operation, DomainError> {
        OperationService::cancel(self, id).await
    }

    async fn cleanup_old(&self) -> Result<u64, DomainError> {
        OperationService::cleanup_old(self).await
    }
}

impl AppState {
    /// Create new application state with provided services
    pub fn new(
        model_service: Arc<dyn ModelServiceTrait>,
        prompt_service: Arc<dyn PromptServiceTrait>,
        api_key_service: Arc<dyn ApiKeyServiceTrait>,
        workflow_service: Arc<dyn WorkflowServiceTrait>,
        operation_service: Arc<dyn OperationServiceTrait>,
        llm_provider: Arc<dyn LlmProvider>,
    ) -> Self {
        Self {
            model_service,
            prompt_service,
            api_key_service,
            workflow_service,
            operation_service,
            llm_provider,
        }
    }
}
