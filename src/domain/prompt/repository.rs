//! Prompt repository trait

use async_trait::async_trait;

use super::{Prompt, PromptId};
use crate::domain::DomainError;

/// Repository trait for Prompt persistence
#[async_trait]
pub trait PromptRepository: Send + Sync + std::fmt::Debug {
    /// Get a prompt by ID
    async fn get(&self, id: &PromptId) -> Result<Option<Prompt>, DomainError>;

    /// Get all prompts
    async fn list(&self) -> Result<Vec<Prompt>, DomainError>;

    /// Get all enabled prompts
    async fn list_enabled(&self) -> Result<Vec<Prompt>, DomainError>;

    /// Get prompts by tag
    async fn list_by_tag(&self, tag: &str) -> Result<Vec<Prompt>, DomainError>;

    /// Create a new prompt
    async fn create(&self, prompt: Prompt) -> Result<Prompt, DomainError>;

    /// Update an existing prompt
    async fn update(&self, prompt: Prompt) -> Result<Prompt, DomainError>;

    /// Delete a prompt by ID
    async fn delete(&self, id: &PromptId) -> Result<bool, DomainError>;

    /// Check if a prompt exists
    async fn exists(&self, id: &PromptId) -> Result<bool, DomainError>;
}

/// In-memory implementation of PromptRepository
pub mod in_memory {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    /// In-memory implementation of PromptRepository for testing and development
    #[derive(Debug, Default)]
    pub struct InMemoryPromptRepository {
        prompts: Mutex<HashMap<String, Prompt>>,
    }

    impl InMemoryPromptRepository {
        pub fn new() -> Self {
            Self::default()
        }

        pub fn with_prompt(self, prompt: Prompt) -> Self {
            self.prompts
                .lock()
                .unwrap()
                .insert(prompt.id().to_string(), prompt);
            self
        }

        pub fn with_prompts(self, prompts: Vec<Prompt>) -> Self {
            let mut map = self.prompts.lock().unwrap();

            for prompt in prompts {
                map.insert(prompt.id().to_string(), prompt);
            }
            drop(map);
            self
        }
    }

    #[async_trait]
    impl PromptRepository for InMemoryPromptRepository {
        async fn get(&self, id: &PromptId) -> Result<Option<Prompt>, DomainError> {
            Ok(self.prompts.lock().unwrap().get(id.as_str()).cloned())
        }

        async fn list(&self) -> Result<Vec<Prompt>, DomainError> {
            Ok(self.prompts.lock().unwrap().values().cloned().collect())
        }

        async fn list_enabled(&self) -> Result<Vec<Prompt>, DomainError> {
            Ok(self
                .prompts
                .lock()
                .unwrap()
                .values()
                .filter(|p| p.is_enabled())
                .cloned()
                .collect())
        }

        async fn list_by_tag(&self, tag: &str) -> Result<Vec<Prompt>, DomainError> {
            Ok(self
                .prompts
                .lock()
                .unwrap()
                .values()
                .filter(|p| p.tags().contains(&tag.to_string()))
                .cloned()
                .collect())
        }

        async fn create(&self, prompt: Prompt) -> Result<Prompt, DomainError> {
            let id = prompt.id().to_string();

            if self.prompts.lock().unwrap().contains_key(&id) {
                return Err(DomainError::conflict(format!(
                    "Prompt with ID '{}' already exists",
                    id
                )));
            }

            self.prompts.lock().unwrap().insert(id, prompt.clone());
            Ok(prompt)
        }

        async fn update(&self, prompt: Prompt) -> Result<Prompt, DomainError> {
            let id = prompt.id().to_string();

            if !self.prompts.lock().unwrap().contains_key(&id) {
                return Err(DomainError::not_found(format!(
                    "Prompt '{}' not found",
                    id
                )));
            }

            self.prompts.lock().unwrap().insert(id, prompt.clone());
            Ok(prompt)
        }

        async fn delete(&self, id: &PromptId) -> Result<bool, DomainError> {
            Ok(self.prompts.lock().unwrap().remove(id.as_str()).is_some())
        }

        async fn exists(&self, id: &PromptId) -> Result<bool, DomainError> {
            Ok(self.prompts.lock().unwrap().contains_key(id.as_str()))
        }
    }
}

#[cfg(test)]
pub mod mock {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    /// Mock implementation of PromptRepository for testing
    #[derive(Debug, Default)]
    pub struct MockPromptRepository {
        prompts: Mutex<HashMap<String, Prompt>>,
        error: Mutex<Option<String>>,
    }

    impl MockPromptRepository {
        pub fn new() -> Self {
            Self::default()
        }

        pub fn with_prompt(self, prompt: Prompt) -> Self {
            self.prompts
                .lock()
                .unwrap()
                .insert(prompt.id().to_string(), prompt);
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
    impl PromptRepository for MockPromptRepository {
        async fn get(&self, id: &PromptId) -> Result<Option<Prompt>, DomainError> {
            self.check_error()?;
            Ok(self.prompts.lock().unwrap().get(id.as_str()).cloned())
        }

        async fn list(&self) -> Result<Vec<Prompt>, DomainError> {
            self.check_error()?;
            Ok(self.prompts.lock().unwrap().values().cloned().collect())
        }

        async fn list_enabled(&self) -> Result<Vec<Prompt>, DomainError> {
            self.check_error()?;
            Ok(self
                .prompts
                .lock()
                .unwrap()
                .values()
                .filter(|p| p.is_enabled())
                .cloned()
                .collect())
        }

        async fn list_by_tag(&self, tag: &str) -> Result<Vec<Prompt>, DomainError> {
            self.check_error()?;
            Ok(self
                .prompts
                .lock()
                .unwrap()
                .values()
                .filter(|p| p.tags().contains(&tag.to_string()))
                .cloned()
                .collect())
        }

        async fn create(&self, prompt: Prompt) -> Result<Prompt, DomainError> {
            self.check_error()?;
            let id = prompt.id().to_string();

            if self.prompts.lock().unwrap().contains_key(&id) {
                return Err(DomainError::conflict(format!(
                    "Prompt with ID '{}' already exists",
                    id
                )));
            }

            self.prompts.lock().unwrap().insert(id, prompt.clone());
            Ok(prompt)
        }

        async fn update(&self, prompt: Prompt) -> Result<Prompt, DomainError> {
            self.check_error()?;
            let id = prompt.id().to_string();

            if !self.prompts.lock().unwrap().contains_key(&id) {
                return Err(DomainError::not_found(format!(
                    "Prompt '{}' not found",
                    id
                )));
            }

            self.prompts.lock().unwrap().insert(id, prompt.clone());
            Ok(prompt)
        }

        async fn delete(&self, id: &PromptId) -> Result<bool, DomainError> {
            self.check_error()?;
            Ok(self.prompts.lock().unwrap().remove(id.as_str()).is_some())
        }

        async fn exists(&self, id: &PromptId) -> Result<bool, DomainError> {
            self.check_error()?;
            Ok(self.prompts.lock().unwrap().contains_key(id.as_str()))
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn create_test_prompt(id: &str) -> Prompt {
            Prompt::new(
                PromptId::new(id).unwrap(),
                format!("Test Prompt {}", id),
                "You are a helpful assistant.",
            )
        }

        #[tokio::test]
        async fn test_mock_repository_crud() {
            let repo = MockPromptRepository::new();

            // Create
            let prompt = create_test_prompt("test-1");
            let created = repo.create(prompt.clone()).await.unwrap();
            assert_eq!(created.id().as_str(), "test-1");

            // Get
            let id = PromptId::new("test-1").unwrap();
            let fetched = repo.get(&id).await.unwrap();
            assert!(fetched.is_some());
            assert_eq!(fetched.unwrap().name(), "Test Prompt test-1");

            // List
            let all = repo.list().await.unwrap();
            assert_eq!(all.len(), 1);

            // Update
            let mut updated_prompt = create_test_prompt("test-1");
            updated_prompt.set_name("Updated Name");
            let updated = repo.update(updated_prompt).await.unwrap();
            assert_eq!(updated.name(), "Updated Name");

            // Delete
            let deleted = repo.delete(&id).await.unwrap();
            assert!(deleted);

            // Verify deleted
            let not_found = repo.get(&id).await.unwrap();
            assert!(not_found.is_none());
        }

        #[tokio::test]
        async fn test_mock_repository_list_by_tag() {
            let prompt1 = create_test_prompt("prompt-1").with_tag("system");
            let prompt2 = create_test_prompt("prompt-2").with_tag("user");
            let prompt3 = create_test_prompt("prompt-3").with_tag("system");

            let repo = MockPromptRepository::new()
                .with_prompt(prompt1)
                .with_prompt(prompt2)
                .with_prompt(prompt3);

            let system_prompts = repo.list_by_tag("system").await.unwrap();
            assert_eq!(system_prompts.len(), 2);

            let user_prompts = repo.list_by_tag("user").await.unwrap();
            assert_eq!(user_prompts.len(), 1);
        }
    }
}
