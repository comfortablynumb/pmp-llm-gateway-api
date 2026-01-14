//! Workflow repository trait

use async_trait::async_trait;

use super::entity::{Workflow, WorkflowId};
use crate::domain::DomainError;

/// Repository trait for workflow persistence
#[async_trait]
pub trait WorkflowRepository: Send + Sync + std::fmt::Debug {
    /// Get a workflow by ID
    async fn get(&self, id: &WorkflowId) -> Result<Option<Workflow>, DomainError>;

    /// List all workflows
    async fn list(&self) -> Result<Vec<Workflow>, DomainError>;

    /// List only enabled workflows
    async fn list_enabled(&self) -> Result<Vec<Workflow>, DomainError>;

    /// Create a new workflow
    async fn create(&self, workflow: Workflow) -> Result<Workflow, DomainError>;

    /// Update an existing workflow
    async fn update(&self, workflow: Workflow) -> Result<Workflow, DomainError>;

    /// Delete a workflow by ID
    async fn delete(&self, id: &WorkflowId) -> Result<bool, DomainError>;

    /// Check if a workflow exists
    async fn exists(&self, id: &WorkflowId) -> Result<bool, DomainError>;
}

#[cfg(test)]
pub mod mock {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    /// Mock workflow repository for testing
    #[derive(Debug, Default)]
    pub struct MockWorkflowRepository {
        workflows: Mutex<HashMap<String, Workflow>>,
        should_fail: Mutex<Option<String>>,
    }

    impl MockWorkflowRepository {
        pub fn new() -> Self {
            Self::default()
        }

        pub fn with_workflow(self, workflow: Workflow) -> Self {
            self.workflows
                .lock()
                .unwrap()
                .insert(workflow.id().as_str().to_string(), workflow);
            self
        }

        pub fn with_error(self, error: impl Into<String>) -> Self {
            *self.should_fail.lock().unwrap() = Some(error.into());
            self
        }

        fn check_error(&self) -> Result<(), DomainError> {
            if let Some(ref msg) = *self.should_fail.lock().unwrap() {
                return Err(DomainError::internal(msg.clone()));
            }
            Ok(())
        }
    }

    #[async_trait]
    impl WorkflowRepository for MockWorkflowRepository {
        async fn get(&self, id: &WorkflowId) -> Result<Option<Workflow>, DomainError> {
            self.check_error()?;
            let workflows = self.workflows.lock().unwrap();
            Ok(workflows.get(id.as_str()).cloned())
        }

        async fn list(&self) -> Result<Vec<Workflow>, DomainError> {
            self.check_error()?;
            let workflows = self.workflows.lock().unwrap();
            Ok(workflows.values().cloned().collect())
        }

        async fn list_enabled(&self) -> Result<Vec<Workflow>, DomainError> {
            self.check_error()?;
            let workflows = self.workflows.lock().unwrap();
            Ok(workflows
                .values()
                .filter(|w| w.is_enabled())
                .cloned()
                .collect())
        }

        async fn create(&self, workflow: Workflow) -> Result<Workflow, DomainError> {
            self.check_error()?;
            let mut workflows = self.workflows.lock().unwrap();

            if workflows.contains_key(workflow.id().as_str()) {
                return Err(DomainError::conflict(format!(
                    "Workflow '{}' already exists",
                    workflow.id()
                )));
            }

            workflows.insert(workflow.id().as_str().to_string(), workflow.clone());
            Ok(workflow)
        }

        async fn update(&self, workflow: Workflow) -> Result<Workflow, DomainError> {
            self.check_error()?;
            let mut workflows = self.workflows.lock().unwrap();

            if !workflows.contains_key(workflow.id().as_str()) {
                return Err(DomainError::not_found(format!(
                    "Workflow '{}' not found",
                    workflow.id()
                )));
            }

            workflows.insert(workflow.id().as_str().to_string(), workflow.clone());
            Ok(workflow)
        }

        async fn delete(&self, id: &WorkflowId) -> Result<bool, DomainError> {
            self.check_error()?;
            let mut workflows = self.workflows.lock().unwrap();
            Ok(workflows.remove(id.as_str()).is_some())
        }

        async fn exists(&self, id: &WorkflowId) -> Result<bool, DomainError> {
            self.check_error()?;
            let workflows = self.workflows.lock().unwrap();
            Ok(workflows.contains_key(id.as_str()))
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::domain::workflow::{ChatCompletionStep, WorkflowStep, WorkflowStepType};

        fn create_test_workflow(id: &str) -> Workflow {
            Workflow::new(WorkflowId::new(id).unwrap(), format!("Test {}", id)).with_step(
                WorkflowStep::new(
                    "step1",
                    WorkflowStepType::ChatCompletion(ChatCompletionStep::new("gpt-4", "sys-prompt")),
                ),
            )
        }

        #[tokio::test]
        async fn test_mock_create_and_get() {
            let repo = MockWorkflowRepository::new();
            let workflow = create_test_workflow("test-1");

            let created = repo.create(workflow.clone()).await.unwrap();
            assert_eq!(created.id().as_str(), "test-1");

            let retrieved = repo.get(created.id()).await.unwrap();
            assert!(retrieved.is_some());
            assert_eq!(retrieved.unwrap().name(), "Test test-1");
        }

        #[tokio::test]
        async fn test_mock_create_duplicate() {
            let workflow = create_test_workflow("test-1");
            let repo = MockWorkflowRepository::new().with_workflow(workflow.clone());

            let result = repo.create(workflow).await;
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("already exists"));
        }

        #[tokio::test]
        async fn test_mock_update() {
            let mut workflow = create_test_workflow("test-1");
            let repo = MockWorkflowRepository::new().with_workflow(workflow.clone());

            workflow.set_name("Updated Name");
            let updated = repo.update(workflow).await.unwrap();
            assert_eq!(updated.name(), "Updated Name");
        }

        #[tokio::test]
        async fn test_mock_update_not_found() {
            let repo = MockWorkflowRepository::new();
            let workflow = create_test_workflow("nonexistent");

            let result = repo.update(workflow).await;
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("not found"));
        }

        #[tokio::test]
        async fn test_mock_delete() {
            let workflow = create_test_workflow("test-1");
            let id = workflow.id().clone();
            let repo = MockWorkflowRepository::new().with_workflow(workflow);

            assert!(repo.exists(&id).await.unwrap());

            let deleted = repo.delete(&id).await.unwrap();
            assert!(deleted);
            assert!(!repo.exists(&id).await.unwrap());

            // Second delete returns false
            let deleted = repo.delete(&id).await.unwrap();
            assert!(!deleted);
        }

        #[tokio::test]
        async fn test_mock_list() {
            let repo = MockWorkflowRepository::new()
                .with_workflow(create_test_workflow("test-1"))
                .with_workflow(create_test_workflow("test-2"));

            let workflows = repo.list().await.unwrap();
            assert_eq!(workflows.len(), 2);
        }

        #[tokio::test]
        async fn test_mock_list_enabled() {
            let disabled = create_test_workflow("disabled").with_enabled(false);
            let enabled = create_test_workflow("enabled").with_enabled(true);

            let repo = MockWorkflowRepository::new()
                .with_workflow(disabled)
                .with_workflow(enabled);

            let all = repo.list().await.unwrap();
            assert_eq!(all.len(), 2);

            let enabled_only = repo.list_enabled().await.unwrap();
            assert_eq!(enabled_only.len(), 1);
            assert_eq!(enabled_only[0].id().as_str(), "enabled");
        }

        #[tokio::test]
        async fn test_mock_with_error() {
            let repo = MockWorkflowRepository::new().with_error("Simulated error");

            let result = repo.list().await;
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("Simulated error"));
        }
    }
}
