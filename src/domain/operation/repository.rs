//! Operation repository trait

use std::fmt::Debug;

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use super::{Operation, OperationId, OperationStatus};
use crate::domain::error::DomainError;

/// Repository trait for async operations
#[async_trait]
pub trait OperationRepository: Send + Sync + Debug {
    /// Get an operation by ID
    async fn get(&self, id: &OperationId) -> Result<Option<Operation>, DomainError>;

    /// Create a new operation
    async fn create(&self, operation: Operation) -> Result<Operation, DomainError>;

    /// Update an existing operation
    async fn update(&self, operation: &Operation) -> Result<Operation, DomainError>;

    /// Delete an operation by ID
    async fn delete(&self, id: &OperationId) -> Result<bool, DomainError>;

    /// List operations by status
    async fn list_by_status(&self, status: OperationStatus) -> Result<Vec<Operation>, DomainError>;

    /// List operations by multiple IDs
    async fn list_by_ids(&self, ids: &[OperationId]) -> Result<Vec<Operation>, DomainError>;

    /// Delete operations older than a given timestamp
    async fn delete_older_than(&self, before: DateTime<Utc>) -> Result<u64, DomainError>;
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use serde_json::json;

    use crate::domain::operation::OperationType;

    /// Helper to create a test operation
    pub fn create_test_operation(op_type: OperationType) -> Operation {
        Operation::new(op_type, json!({"test": true}), json!({"source": "test"}))
    }

    /// Test suite for OperationRepository implementations
    pub async fn test_repository_basic_crud<R: OperationRepository>(repo: &R) {
        // Create
        let op = create_test_operation(OperationType::ChatCompletion);
        let op_id = op.id().clone();

        let created = repo.create(op).await.expect("create should succeed");
        assert_eq!(created.id(), &op_id);
        assert_eq!(created.status(), OperationStatus::Pending);

        // Get
        let fetched = repo.get(&op_id).await.expect("get should succeed");
        assert!(fetched.is_some());
        let fetched = fetched.unwrap();
        assert_eq!(fetched.id(), &op_id);

        // Update
        let mut updated_op = fetched.clone();
        updated_op.mark_running().expect("mark_running should succeed");
        let updated = repo.update(&updated_op).await.expect("update should succeed");
        assert_eq!(updated.status(), OperationStatus::Running);

        // Verify update persisted
        let fetched_updated = repo.get(&op_id).await.expect("get should succeed").unwrap();
        assert_eq!(fetched_updated.status(), OperationStatus::Running);

        // Delete
        let deleted = repo.delete(&op_id).await.expect("delete should succeed");
        assert!(deleted);

        // Verify deletion
        let after_delete = repo.get(&op_id).await.expect("get should succeed");
        assert!(after_delete.is_none());
    }

    /// Test list by status functionality
    pub async fn test_repository_list_by_status<R: OperationRepository>(repo: &R) {
        // Create multiple operations
        let op1 = create_test_operation(OperationType::ChatCompletion);
        let op2 = create_test_operation(OperationType::WorkflowExecution);
        let op3 = create_test_operation(OperationType::ChatCompletion);

        let op1_id = op1.id().clone();

        repo.create(op1).await.expect("create should succeed");
        repo.create(op2).await.expect("create should succeed");
        repo.create(op3).await.expect("create should succeed");

        // All should be pending
        let pending = repo
            .list_by_status(OperationStatus::Pending)
            .await
            .expect("list should succeed");
        assert!(pending.len() >= 3);

        // Mark one as running
        let mut fetched = repo.get(&op1_id).await.unwrap().unwrap();
        fetched.mark_running().unwrap();
        repo.update(&fetched).await.unwrap();

        // Check running list
        let running = repo
            .list_by_status(OperationStatus::Running)
            .await
            .expect("list should succeed");
        assert!(running.iter().any(|op| op.id() == &op1_id));
    }

    /// Test list by IDs functionality
    pub async fn test_repository_list_by_ids<R: OperationRepository>(repo: &R) {
        let op1 = create_test_operation(OperationType::ChatCompletion);
        let op2 = create_test_operation(OperationType::WorkflowExecution);
        let op3 = create_test_operation(OperationType::ChatCompletion);

        let id1 = op1.id().clone();
        let id2 = op2.id().clone();
        let id3 = op3.id().clone();

        repo.create(op1).await.unwrap();
        repo.create(op2).await.unwrap();
        repo.create(op3).await.unwrap();

        // Fetch specific IDs
        let results = repo
            .list_by_ids(&[id1.clone(), id3.clone()])
            .await
            .expect("list should succeed");
        assert_eq!(results.len(), 2);
        assert!(results.iter().any(|op| op.id() == &id1));
        assert!(results.iter().any(|op| op.id() == &id3));
        assert!(!results.iter().any(|op| op.id() == &id2));

        // Empty list should return empty
        let empty = repo.list_by_ids(&[]).await.expect("list should succeed");
        assert!(empty.is_empty());
    }

    /// Test delete older than functionality
    pub async fn test_repository_delete_older_than<R: OperationRepository>(repo: &R) {
        let op1 = create_test_operation(OperationType::ChatCompletion);
        let op2 = create_test_operation(OperationType::WorkflowExecution);

        repo.create(op1).await.unwrap();
        repo.create(op2).await.unwrap();

        // Delete operations older than now (should delete none since they were just created)
        let deleted = repo
            .delete_older_than(Utc::now() - chrono::Duration::hours(1))
            .await
            .expect("delete should succeed");
        assert_eq!(deleted, 0);

        // Delete operations older than future (should delete all)
        let deleted = repo
            .delete_older_than(Utc::now() + chrono::Duration::hours(1))
            .await
            .expect("delete should succeed");
        assert!(deleted >= 2);
    }
}
