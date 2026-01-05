//! Operation service for managing async operations

use std::fmt::Debug;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::Utc;
use serde_json::Value;
use tracing::{debug, info, instrument, warn};

use crate::domain::error::DomainError;
use crate::domain::operation::{Operation, OperationId, OperationRepository, OperationType};

/// Operation service configuration
#[derive(Debug, Clone)]
pub struct OperationServiceConfig {
    /// How long to keep completed operations before cleanup
    pub retention_duration: Duration,
}

impl Default for OperationServiceConfig {
    fn default() -> Self {
        Self {
            retention_duration: Duration::from_secs(3600), // 1 hour
        }
    }
}

impl OperationServiceConfig {
    /// Create config with custom retention duration
    pub fn with_retention(retention_duration: Duration) -> Self {
        Self { retention_duration }
    }
}

/// Trait for operation service (for dynamic dispatch in AppState)
#[async_trait]
pub trait OperationServiceTrait: Send + Sync + Debug {
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

/// Operation service implementation
#[derive(Debug)]
pub struct OperationService<R: OperationRepository> {
    repository: Arc<R>,
    config: OperationServiceConfig,
}

impl<R: OperationRepository> OperationService<R> {
    /// Create a new operation service
    pub fn new(repository: Arc<R>) -> Self {
        Self {
            repository,
            config: OperationServiceConfig::default(),
        }
    }

    /// Create with custom configuration
    pub fn with_config(repository: Arc<R>, config: OperationServiceConfig) -> Self {
        Self { repository, config }
    }

    /// Parse an operation ID string
    fn parse_id(&self, id: &str) -> Result<OperationId, DomainError> {
        OperationId::new(id).map_err(|e| DomainError::invalid_id(e.to_string()))
    }

    /// Get operation and return error if not found
    async fn get_required(&self, id: &str) -> Result<Operation, DomainError> {
        let op_id = self.parse_id(id)?;

        self.repository
            .get(&op_id)
            .await?
            .ok_or_else(|| DomainError::not_found(format!("Operation '{}'", id)))
    }
}

#[async_trait]
impl<R: OperationRepository> OperationServiceTrait for OperationService<R> {
    #[instrument(skip(self, input, metadata), fields(op_type = %op_type))]
    async fn create_pending(
        &self,
        op_type: OperationType,
        input: Value,
        metadata: Value,
    ) -> Result<Operation, DomainError> {
        let operation = Operation::new(op_type, input, metadata);
        let op_id = operation.id().to_string();

        let created = self.repository.create(operation).await?;
        info!(operation_id = %op_id, "Created pending operation");

        Ok(created)
    }

    #[instrument(skip(self))]
    async fn get(&self, id: &str) -> Result<Option<Operation>, DomainError> {
        let op_id = self.parse_id(id)?;
        self.repository.get(&op_id).await
    }

    #[instrument(skip(self))]
    async fn get_batch(&self, ids: &[String]) -> Result<Vec<Operation>, DomainError> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        let op_ids: Result<Vec<OperationId>, _> =
            ids.iter().map(|id| self.parse_id(id)).collect();
        let op_ids = op_ids?;

        self.repository.list_by_ids(&op_ids).await
    }

    #[instrument(skip(self))]
    async fn mark_running(&self, id: &str) -> Result<Operation, DomainError> {
        let mut operation = self.get_required(id).await?;

        operation
            .mark_running()
            .map_err(|e| DomainError::validation(e.to_string()))?;

        let updated = self.repository.update(&operation).await?;
        debug!(operation_id = %id, "Marked operation as running");

        Ok(updated)
    }

    #[instrument(skip(self, result))]
    async fn mark_completed(&self, id: &str, result: Value) -> Result<Operation, DomainError> {
        let mut operation = self.get_required(id).await?;

        operation
            .mark_completed(result)
            .map_err(|e| DomainError::validation(e.to_string()))?;

        let updated = self.repository.update(&operation).await?;
        info!(operation_id = %id, "Marked operation as completed");

        Ok(updated)
    }

    #[instrument(skip(self))]
    async fn mark_failed(&self, id: &str, error: String) -> Result<Operation, DomainError> {
        let mut operation = self.get_required(id).await?;

        operation
            .mark_failed(&error)
            .map_err(|e| DomainError::validation(e.to_string()))?;

        let updated = self.repository.update(&operation).await?;
        warn!(operation_id = %id, error = %error, "Marked operation as failed");

        Ok(updated)
    }

    #[instrument(skip(self))]
    async fn cancel(&self, id: &str) -> Result<Operation, DomainError> {
        let mut operation = self.get_required(id).await?;

        operation
            .mark_cancelled()
            .map_err(|e| DomainError::validation(e.to_string()))?;

        let updated = self.repository.update(&operation).await?;
        info!(operation_id = %id, "Cancelled operation");

        Ok(updated)
    }

    #[instrument(skip(self))]
    async fn cleanup_old(&self) -> Result<u64, DomainError> {
        let cutoff = Utc::now() - chrono::Duration::from_std(self.config.retention_duration)
            .unwrap_or_else(|_| chrono::Duration::hours(1));

        let deleted = self.repository.delete_older_than(cutoff).await?;

        if deleted > 0 {
            info!(deleted_count = deleted, "Cleaned up old operations");
        }

        Ok(deleted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::operation::OperationStatus;
    use crate::infrastructure::operation::InMemoryOperationRepository;
    use serde_json::json;

    fn create_test_service() -> OperationService<InMemoryOperationRepository> {
        let repo = Arc::new(InMemoryOperationRepository::new());
        OperationService::new(repo)
    }

    #[tokio::test]
    async fn test_create_pending() {
        let service = create_test_service();

        let operation = service
            .create_pending(
                OperationType::ChatCompletion,
                json!({"model": "gpt-4"}),
                json!({"request_id": "123"}),
            )
            .await
            .expect("create should succeed");

        assert!(operation.id().as_str().starts_with("op-"));
        assert_eq!(operation.status(), OperationStatus::Pending);
        assert_eq!(operation.operation_type(), OperationType::ChatCompletion);
    }

    #[tokio::test]
    async fn test_get_operation() {
        let service = create_test_service();

        let created = service
            .create_pending(OperationType::ChatCompletion, json!({}), json!({}))
            .await
            .unwrap();

        let fetched = service
            .get(created.id().as_str())
            .await
            .expect("get should succeed")
            .expect("operation should exist");

        assert_eq!(fetched.id(), created.id());
    }

    #[tokio::test]
    async fn test_get_nonexistent() {
        let service = create_test_service();

        let result = service
            .get("op-12345678-1234-1234-1234-123456789abc")
            .await
            .expect("get should succeed");

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_get_invalid_id() {
        let service = create_test_service();

        let result = service.get("invalid-id").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_batch() {
        let service = create_test_service();

        let op1 = service
            .create_pending(OperationType::ChatCompletion, json!({}), json!({}))
            .await
            .unwrap();
        let op2 = service
            .create_pending(OperationType::WorkflowExecution, json!({}), json!({}))
            .await
            .unwrap();

        let ids = vec![op1.id().to_string(), op2.id().to_string()];
        let results = service.get_batch(&ids).await.expect("get_batch should succeed");

        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn test_operation_lifecycle() {
        let service = create_test_service();

        // Create
        let created = service
            .create_pending(OperationType::ChatCompletion, json!({}), json!({}))
            .await
            .unwrap();
        let id = created.id().as_str();

        // Mark running
        let running = service.mark_running(id).await.expect("mark_running should succeed");
        assert_eq!(running.status(), OperationStatus::Running);
        assert!(running.started_at().is_some());

        // Mark completed
        let completed = service
            .mark_completed(id, json!({"response": "hello"}))
            .await
            .expect("mark_completed should succeed");
        assert_eq!(completed.status(), OperationStatus::Completed);
        assert!(completed.completed_at().is_some());
        assert_eq!(completed.result(), Some(&json!({"response": "hello"})));
    }

    #[tokio::test]
    async fn test_mark_failed() {
        let service = create_test_service();

        let created = service
            .create_pending(OperationType::ChatCompletion, json!({}), json!({}))
            .await
            .unwrap();
        let id = created.id().as_str();

        service.mark_running(id).await.unwrap();

        let failed = service
            .mark_failed(id, "Something went wrong".to_string())
            .await
            .expect("mark_failed should succeed");

        assert_eq!(failed.status(), OperationStatus::Failed);
        assert_eq!(failed.error(), Some("Something went wrong"));
    }

    #[tokio::test]
    async fn test_cancel_pending() {
        let service = create_test_service();

        let created = service
            .create_pending(OperationType::ChatCompletion, json!({}), json!({}))
            .await
            .unwrap();

        let cancelled = service
            .cancel(created.id().as_str())
            .await
            .expect("cancel should succeed");

        assert_eq!(cancelled.status(), OperationStatus::Cancelled);
    }

    #[tokio::test]
    async fn test_cancel_running() {
        let service = create_test_service();

        let created = service
            .create_pending(OperationType::ChatCompletion, json!({}), json!({}))
            .await
            .unwrap();
        let id = created.id().as_str();

        service.mark_running(id).await.unwrap();

        let cancelled = service.cancel(id).await.expect("cancel should succeed");
        assert_eq!(cancelled.status(), OperationStatus::Cancelled);
    }

    #[tokio::test]
    async fn test_cannot_cancel_completed() {
        let service = create_test_service();

        let created = service
            .create_pending(OperationType::ChatCompletion, json!({}), json!({}))
            .await
            .unwrap();
        let id = created.id().as_str();

        service.mark_running(id).await.unwrap();
        service.mark_completed(id, json!({})).await.unwrap();

        let result = service.cancel(id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_invalid_state_transition() {
        let service = create_test_service();

        let created = service
            .create_pending(OperationType::ChatCompletion, json!({}), json!({}))
            .await
            .unwrap();
        let id = created.id().as_str();

        // Cannot complete from pending
        let result = service.mark_completed(id, json!({})).await;
        assert!(result.is_err());

        // Cannot fail from pending
        let result = service.mark_failed(id, "error".to_string()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_cleanup_old() {
        let service = create_test_service();

        // Create some operations
        service
            .create_pending(OperationType::ChatCompletion, json!({}), json!({}))
            .await
            .unwrap();
        service
            .create_pending(OperationType::WorkflowExecution, json!({}), json!({}))
            .await
            .unwrap();

        // Cleanup (should delete none since retention is 1 hour and ops are fresh)
        let deleted = service.cleanup_old().await.expect("cleanup should succeed");
        assert_eq!(deleted, 0);
    }
}
