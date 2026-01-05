//! Chain repository trait

use async_trait::async_trait;

use super::{ChainId, ModelChain};
use crate::domain::DomainError;

/// Repository trait for ModelChain persistence
#[async_trait]
pub trait ChainRepository: Send + Sync + std::fmt::Debug {
    /// Get a chain by ID
    async fn get(&self, id: &ChainId) -> Result<Option<ModelChain>, DomainError>;

    /// Get all chains
    async fn list(&self) -> Result<Vec<ModelChain>, DomainError>;

    /// Get all enabled chains
    async fn list_enabled(&self) -> Result<Vec<ModelChain>, DomainError>;

    /// Create a new chain
    async fn create(&self, chain: ModelChain) -> Result<ModelChain, DomainError>;

    /// Update an existing chain
    async fn update(&self, chain: ModelChain) -> Result<ModelChain, DomainError>;

    /// Delete a chain by ID
    async fn delete(&self, id: &ChainId) -> Result<bool, DomainError>;

    /// Check if a chain exists
    async fn exists(&self, id: &ChainId) -> Result<bool, DomainError>;
}

#[cfg(test)]
pub mod mock {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    /// Mock implementation of ChainRepository for testing
    #[derive(Debug, Default)]
    pub struct MockChainRepository {
        chains: Mutex<HashMap<String, ModelChain>>,
        error: Mutex<Option<String>>,
    }

    impl MockChainRepository {
        pub fn new() -> Self {
            Self::default()
        }

        pub fn with_chain(self, chain: ModelChain) -> Self {
            self.chains
                .lock()
                .unwrap()
                .insert(chain.id().to_string(), chain);
            self
        }

        pub fn with_error(self, error: impl Into<String>) -> Self {
            *self.error.lock().unwrap() = Some(error.into());
            self
        }

        fn check_error(&self) -> Result<(), DomainError> {
            if let Some(err) = self.error.lock().unwrap().as_ref() {
                return Err(DomainError::internal(err.clone()));
            }
            Ok(())
        }
    }

    #[async_trait]
    impl ChainRepository for MockChainRepository {
        async fn get(&self, id: &ChainId) -> Result<Option<ModelChain>, DomainError> {
            self.check_error()?;
            Ok(self.chains.lock().unwrap().get(id.as_str()).cloned())
        }

        async fn list(&self) -> Result<Vec<ModelChain>, DomainError> {
            self.check_error()?;
            Ok(self.chains.lock().unwrap().values().cloned().collect())
        }

        async fn list_enabled(&self) -> Result<Vec<ModelChain>, DomainError> {
            self.check_error()?;
            Ok(self
                .chains
                .lock()
                .unwrap()
                .values()
                .filter(|c| c.is_enabled())
                .cloned()
                .collect())
        }

        async fn create(&self, chain: ModelChain) -> Result<ModelChain, DomainError> {
            self.check_error()?;
            let id = chain.id().to_string();

            if self.chains.lock().unwrap().contains_key(&id) {
                return Err(DomainError::conflict(format!(
                    "Chain with ID '{}' already exists",
                    id
                )));
            }

            self.chains.lock().unwrap().insert(id, chain.clone());
            Ok(chain)
        }

        async fn update(&self, chain: ModelChain) -> Result<ModelChain, DomainError> {
            self.check_error()?;
            let id = chain.id().to_string();

            if !self.chains.lock().unwrap().contains_key(&id) {
                return Err(DomainError::not_found(format!("Chain '{}' not found", id)));
            }

            self.chains.lock().unwrap().insert(id, chain.clone());
            Ok(chain)
        }

        async fn delete(&self, id: &ChainId) -> Result<bool, DomainError> {
            self.check_error()?;
            Ok(self.chains.lock().unwrap().remove(id.as_str()).is_some())
        }

        async fn exists(&self, id: &ChainId) -> Result<bool, DomainError> {
            self.check_error()?;
            Ok(self.chains.lock().unwrap().contains_key(id.as_str()))
        }
    }
}
