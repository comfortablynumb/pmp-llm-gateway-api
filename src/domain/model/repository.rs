//! Model repository trait

use async_trait::async_trait;

use super::{Model, ModelId};
use crate::domain::DomainError;

/// Repository trait for Model persistence
#[async_trait]
pub trait ModelRepository: Send + Sync + std::fmt::Debug {
    /// Get a model by ID
    async fn get(&self, id: &ModelId) -> Result<Option<Model>, DomainError>;

    /// Get all models
    async fn list(&self) -> Result<Vec<Model>, DomainError>;

    /// Get all enabled models
    async fn list_enabled(&self) -> Result<Vec<Model>, DomainError>;

    /// Create a new model
    async fn create(&self, model: Model) -> Result<Model, DomainError>;

    /// Update an existing model
    async fn update(&self, model: Model) -> Result<Model, DomainError>;

    /// Delete a model by ID
    async fn delete(&self, id: &ModelId) -> Result<bool, DomainError>;

    /// Check if a model exists
    async fn exists(&self, id: &ModelId) -> Result<bool, DomainError>;
}

/// In-memory implementation of ModelRepository
pub mod in_memory {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    /// In-memory implementation of ModelRepository for testing and development
    #[derive(Debug, Default)]
    pub struct InMemoryModelRepository {
        models: Mutex<HashMap<String, Model>>,
    }

    impl InMemoryModelRepository {
        pub fn new() -> Self {
            Self::default()
        }

        pub fn with_model(self, model: Model) -> Self {
            self.models
                .lock()
                .unwrap()
                .insert(model.id().to_string(), model);
            self
        }

        pub fn with_models(self, models: Vec<Model>) -> Self {
            let mut map = self.models.lock().unwrap();

            for model in models {
                map.insert(model.id().to_string(), model);
            }
            drop(map);
            self
        }
    }

    #[async_trait]
    impl ModelRepository for InMemoryModelRepository {
        async fn get(&self, id: &ModelId) -> Result<Option<Model>, DomainError> {
            Ok(self.models.lock().unwrap().get(id.as_str()).cloned())
        }

        async fn list(&self) -> Result<Vec<Model>, DomainError> {
            Ok(self.models.lock().unwrap().values().cloned().collect())
        }

        async fn list_enabled(&self) -> Result<Vec<Model>, DomainError> {
            Ok(self
                .models
                .lock()
                .unwrap()
                .values()
                .filter(|m| m.is_enabled())
                .cloned()
                .collect())
        }

        async fn create(&self, model: Model) -> Result<Model, DomainError> {
            let id = model.id().to_string();

            if self.models.lock().unwrap().contains_key(&id) {
                return Err(DomainError::conflict(format!(
                    "Model with ID '{}' already exists",
                    id
                )));
            }

            self.models.lock().unwrap().insert(id, model.clone());
            Ok(model)
        }

        async fn update(&self, model: Model) -> Result<Model, DomainError> {
            let id = model.id().to_string();

            if !self.models.lock().unwrap().contains_key(&id) {
                return Err(DomainError::not_found(format!("Model '{}' not found", id)));
            }

            self.models.lock().unwrap().insert(id, model.clone());
            Ok(model)
        }

        async fn delete(&self, id: &ModelId) -> Result<bool, DomainError> {
            Ok(self.models.lock().unwrap().remove(id.as_str()).is_some())
        }

        async fn exists(&self, id: &ModelId) -> Result<bool, DomainError> {
            Ok(self.models.lock().unwrap().contains_key(id.as_str()))
        }
    }
}

#[cfg(test)]
pub mod mock {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    /// Mock implementation of ModelRepository for testing
    #[derive(Debug, Default)]
    pub struct MockModelRepository {
        models: Mutex<HashMap<String, Model>>,
        error: Mutex<Option<String>>,
    }

    impl MockModelRepository {
        pub fn new() -> Self {
            Self::default()
        }

        pub fn with_model(self, model: Model) -> Self {
            self.models
                .lock()
                .unwrap()
                .insert(model.id().to_string(), model);
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
    impl ModelRepository for MockModelRepository {
        async fn get(&self, id: &ModelId) -> Result<Option<Model>, DomainError> {
            self.check_error()?;
            Ok(self.models.lock().unwrap().get(id.as_str()).cloned())
        }

        async fn list(&self) -> Result<Vec<Model>, DomainError> {
            self.check_error()?;
            Ok(self.models.lock().unwrap().values().cloned().collect())
        }

        async fn list_enabled(&self) -> Result<Vec<Model>, DomainError> {
            self.check_error()?;
            Ok(self
                .models
                .lock()
                .unwrap()
                .values()
                .filter(|m| m.is_enabled())
                .cloned()
                .collect())
        }

        async fn create(&self, model: Model) -> Result<Model, DomainError> {
            self.check_error()?;
            let id = model.id().to_string();

            if self.models.lock().unwrap().contains_key(&id) {
                return Err(DomainError::conflict(format!(
                    "Model with ID '{}' already exists",
                    id
                )));
            }

            self.models.lock().unwrap().insert(id, model.clone());
            Ok(model)
        }

        async fn update(&self, model: Model) -> Result<Model, DomainError> {
            self.check_error()?;
            let id = model.id().to_string();

            if !self.models.lock().unwrap().contains_key(&id) {
                return Err(DomainError::not_found(format!("Model '{}' not found", id)));
            }

            self.models.lock().unwrap().insert(id, model.clone());
            Ok(model)
        }

        async fn delete(&self, id: &ModelId) -> Result<bool, DomainError> {
            self.check_error()?;
            Ok(self.models.lock().unwrap().remove(id.as_str()).is_some())
        }

        async fn exists(&self, id: &ModelId) -> Result<bool, DomainError> {
            self.check_error()?;
            Ok(self.models.lock().unwrap().contains_key(id.as_str()))
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::domain::CredentialType;

        fn create_test_model(id: &str) -> Model {
            Model::new(
                ModelId::new(id).unwrap(),
                format!("Test Model {}", id),
                CredentialType::OpenAi,
                "gpt-4",
            )
        }

        #[tokio::test]
        async fn test_mock_repository_crud() {
            let repo = MockModelRepository::new();

            // Create
            let model = create_test_model("test-1");
            let created = repo.create(model.clone()).await.unwrap();
            assert_eq!(created.id().as_str(), "test-1");

            // Get
            let id = ModelId::new("test-1").unwrap();
            let fetched = repo.get(&id).await.unwrap();
            assert!(fetched.is_some());
            assert_eq!(fetched.unwrap().name(), "Test Model test-1");

            // List
            let all = repo.list().await.unwrap();
            assert_eq!(all.len(), 1);

            // Update
            let mut updated_model = create_test_model("test-1");
            updated_model.set_name("Updated Name");
            let updated = repo.update(updated_model).await.unwrap();
            assert_eq!(updated.name(), "Updated Name");

            // Delete
            let deleted = repo.delete(&id).await.unwrap();
            assert!(deleted);

            // Verify deleted
            let not_found = repo.get(&id).await.unwrap();
            assert!(not_found.is_none());
        }

        #[tokio::test]
        async fn test_mock_repository_exists() {
            let model = create_test_model("exists-test");
            let repo = MockModelRepository::new().with_model(model);

            let id = ModelId::new("exists-test").unwrap();
            assert!(repo.exists(&id).await.unwrap());

            let missing_id = ModelId::new("missing").unwrap();
            assert!(!repo.exists(&missing_id).await.unwrap());
        }

        #[tokio::test]
        async fn test_mock_repository_duplicate_create() {
            let model = create_test_model("duplicate");
            let repo = MockModelRepository::new().with_model(model.clone());

            let result = repo.create(model).await;
            assert!(result.is_err());
        }

        #[tokio::test]
        async fn test_mock_repository_update_not_found() {
            let repo = MockModelRepository::new();
            let model = create_test_model("not-exists");

            let result = repo.update(model).await;
            assert!(result.is_err());
        }
    }
}
