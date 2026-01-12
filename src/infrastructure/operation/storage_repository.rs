//! Storage-backed operation repository implementation

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::sync::Arc;

use crate::domain::operation::{Operation, OperationId, OperationRepository, OperationStatus};
use crate::domain::storage::Storage;
use crate::domain::DomainError;

/// Storage-backed implementation of OperationRepository
#[derive(Debug)]
pub struct StorageOperationRepository {
    storage: Arc<dyn Storage<Operation>>,
}

impl StorageOperationRepository {
    /// Create a new storage-backed repository
    pub fn new(storage: Arc<dyn Storage<Operation>>) -> Self {
        Self { storage }
    }
}

#[async_trait]
impl OperationRepository for StorageOperationRepository {
    async fn get(&self, id: &OperationId) -> Result<Option<Operation>, DomainError> {
        self.storage.get(id).await
    }

    async fn create(&self, operation: Operation) -> Result<Operation, DomainError> {
        if self.storage.exists(operation.id()).await? {
            return Err(DomainError::conflict(format!(
                "Operation '{}' already exists",
                operation.id()
            )));
        }

        self.storage.create(operation).await
    }

    async fn update(&self, operation: &Operation) -> Result<Operation, DomainError> {
        if !self.storage.exists(operation.id()).await? {
            return Err(DomainError::not_found(format!(
                "Operation '{}' not found",
                operation.id()
            )));
        }

        self.storage.update(operation.clone()).await
    }

    async fn delete(&self, id: &OperationId) -> Result<bool, DomainError> {
        self.storage.delete(id).await
    }

    async fn list_by_status(&self, status: OperationStatus) -> Result<Vec<Operation>, DomainError> {
        let all = self.storage.list().await?;
        Ok(all.into_iter().filter(|op| op.status() == status).collect())
    }

    async fn list_by_ids(&self, ids: &[OperationId]) -> Result<Vec<Operation>, DomainError> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        let all = self.storage.list().await?;
        Ok(all.into_iter().filter(|op| ids.contains(op.id())).collect())
    }

    async fn delete_older_than(&self, before: DateTime<Utc>) -> Result<u64, DomainError> {
        let all = self.storage.list().await?;
        let mut deleted = 0u64;

        for op in all {
            if op.created_at() < before {
                if self.storage.delete(op.id()).await? {
                    deleted += 1;
                }
            }
        }

        Ok(deleted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::operation::repository::tests;
    use crate::infrastructure::storage::InMemoryStorage;

    fn create_repo() -> StorageOperationRepository {
        let storage = Arc::new(InMemoryStorage::<Operation>::new());
        StorageOperationRepository::new(storage)
    }

    #[tokio::test]
    async fn test_basic_crud() {
        let repo = create_repo();
        tests::test_repository_basic_crud(&repo).await;
    }

    #[tokio::test]
    async fn test_list_by_status() {
        let repo = create_repo();
        tests::test_repository_list_by_status(&repo).await;
    }

    #[tokio::test]
    async fn test_list_by_ids() {
        let repo = create_repo();
        tests::test_repository_list_by_ids(&repo).await;
    }

    #[tokio::test]
    async fn test_delete_older_than() {
        let repo = create_repo();
        tests::test_repository_delete_older_than(&repo).await;
    }
}
