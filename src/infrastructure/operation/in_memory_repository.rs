//! In-memory operation repository implementation

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use tokio::sync::RwLock;

use crate::domain::error::DomainError;
use crate::domain::operation::{Operation, OperationId, OperationRepository, OperationStatus};

/// In-memory implementation of OperationRepository
#[derive(Debug)]
pub struct InMemoryOperationRepository {
    operations: Arc<RwLock<HashMap<String, Operation>>>,
}

impl InMemoryOperationRepository {
    /// Create a new empty repository
    pub fn new() -> Self {
        Self {
            operations: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for InMemoryOperationRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl OperationRepository for InMemoryOperationRepository {
    async fn get(&self, id: &OperationId) -> Result<Option<Operation>, DomainError> {
        let operations = self.operations.read().await;
        Ok(operations.get(id.as_str()).cloned())
    }

    async fn create(&self, operation: Operation) -> Result<Operation, DomainError> {
        let mut operations = self.operations.write().await;
        let id = operation.id().as_str().to_string();

        if operations.contains_key(&id) {
            return Err(DomainError::conflict(format!(
                "Operation '{}' already exists",
                operation.id().as_str()
            )));
        }

        operations.insert(id, operation.clone());
        Ok(operation)
    }

    async fn update(&self, operation: &Operation) -> Result<Operation, DomainError> {
        let mut operations = self.operations.write().await;
        let id = operation.id().as_str().to_string();

        if !operations.contains_key(&id) {
            return Err(DomainError::not_found(format!(
                "Operation '{}'",
                operation.id().as_str()
            )));
        }

        operations.insert(id, operation.clone());
        Ok(operation.clone())
    }

    async fn delete(&self, id: &OperationId) -> Result<bool, DomainError> {
        let mut operations = self.operations.write().await;
        Ok(operations.remove(id.as_str()).is_some())
    }

    async fn list_by_status(&self, status: OperationStatus) -> Result<Vec<Operation>, DomainError> {
        let operations = self.operations.read().await;
        let filtered: Vec<Operation> = operations
            .values()
            .filter(|op| op.status() == status)
            .cloned()
            .collect();
        Ok(filtered)
    }

    async fn list_by_ids(&self, ids: &[OperationId]) -> Result<Vec<Operation>, DomainError> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        let operations = self.operations.read().await;
        let results: Vec<Operation> = ids
            .iter()
            .filter_map(|id| operations.get(id.as_str()).cloned())
            .collect();
        Ok(results)
    }

    async fn delete_older_than(&self, before: DateTime<Utc>) -> Result<u64, DomainError> {
        let mut operations = self.operations.write().await;
        let ids_to_delete: Vec<String> = operations
            .iter()
            .filter(|(_, op)| op.created_at() < before)
            .map(|(id, _)| id.clone())
            .collect();

        let count = ids_to_delete.len() as u64;

        for id in ids_to_delete {
            operations.remove(&id);
        }

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::operation::repository::tests::{
        create_test_operation, test_repository_basic_crud, test_repository_delete_older_than,
        test_repository_list_by_ids, test_repository_list_by_status,
    };
    use crate::domain::operation::OperationType;

    #[tokio::test]
    async fn test_basic_crud() {
        let repo = InMemoryOperationRepository::new();
        test_repository_basic_crud(&repo).await;
    }

    #[tokio::test]
    async fn test_list_by_status() {
        let repo = InMemoryOperationRepository::new();
        test_repository_list_by_status(&repo).await;
    }

    #[tokio::test]
    async fn test_list_by_ids() {
        let repo = InMemoryOperationRepository::new();
        test_repository_list_by_ids(&repo).await;
    }

    #[tokio::test]
    async fn test_delete_older_than() {
        let repo = InMemoryOperationRepository::new();
        test_repository_delete_older_than(&repo).await;
    }

    #[tokio::test]
    async fn test_create_duplicate_fails() {
        let repo = InMemoryOperationRepository::new();
        let op = create_test_operation(OperationType::ChatCompletion);
        let op_clone = Operation::with_id(
            op.id().clone(),
            OperationType::WorkflowExecution,
            serde_json::json!({}),
        );

        repo.create(op).await.expect("first create should succeed");

        let result = repo.create(op_clone).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }

    #[tokio::test]
    async fn test_update_nonexistent_fails() {
        let repo = InMemoryOperationRepository::new();
        let op = create_test_operation(OperationType::ChatCompletion);

        let result = repo.update(&op).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Not found"));
    }

    #[tokio::test]
    async fn test_delete_nonexistent_returns_false() {
        let repo = InMemoryOperationRepository::new();
        let id = OperationId::generate();

        let deleted = repo.delete(&id).await.expect("delete should succeed");
        assert!(!deleted);
    }
}
