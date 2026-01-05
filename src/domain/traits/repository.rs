use async_trait::async_trait;
use std::fmt::Debug;

use crate::domain::DomainError;

/// Generic repository trait for CRUD operations
#[async_trait]
pub trait Repository<T, ID>: Send + Sync + Debug
where
    T: Send + Sync,
    ID: Send + Sync,
{
    async fn find_by_id(&self, id: &ID) -> Result<Option<T>, DomainError>;

    async fn find_all(&self) -> Result<Vec<T>, DomainError>;

    async fn save(&self, entity: T) -> Result<T, DomainError>;

    async fn delete(&self, id: &ID) -> Result<bool, DomainError>;

    async fn exists(&self, id: &ID) -> Result<bool, DomainError> {
        Ok(self.find_by_id(id).await?.is_some())
    }
}
