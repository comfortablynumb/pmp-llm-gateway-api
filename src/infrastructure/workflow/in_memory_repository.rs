//! In-memory workflow repository implementation

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::domain::{DomainError, Workflow, WorkflowId, WorkflowRepository};

/// In-memory implementation of WorkflowRepository
#[derive(Debug)]
pub struct InMemoryWorkflowRepository {
    workflows: Arc<RwLock<HashMap<String, Workflow>>>,
}

impl InMemoryWorkflowRepository {
    /// Create a new empty repository
    pub fn new() -> Self {
        Self {
            workflows: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a repository pre-populated with workflows
    pub fn with_workflows(workflows: Vec<Workflow>) -> Self {
        let map: HashMap<String, Workflow> = workflows
            .into_iter()
            .map(|w| (w.id().as_str().to_string(), w))
            .collect();

        Self {
            workflows: Arc::new(RwLock::new(map)),
        }
    }
}

impl Default for InMemoryWorkflowRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl WorkflowRepository for InMemoryWorkflowRepository {
    async fn get(&self, id: &WorkflowId) -> Result<Option<Workflow>, DomainError> {
        let workflows = self.workflows.read().await;
        Ok(workflows.get(id.as_str()).cloned())
    }

    async fn list(&self) -> Result<Vec<Workflow>, DomainError> {
        let workflows = self.workflows.read().await;
        Ok(workflows.values().cloned().collect())
    }

    async fn list_enabled(&self) -> Result<Vec<Workflow>, DomainError> {
        let workflows = self.workflows.read().await;
        Ok(workflows
            .values()
            .filter(|w| w.is_enabled())
            .cloned()
            .collect())
    }

    async fn create(&self, workflow: Workflow) -> Result<Workflow, DomainError> {
        let mut workflows = self.workflows.write().await;

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
        let mut workflows = self.workflows.write().await;

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
        let mut workflows = self.workflows.write().await;
        Ok(workflows.remove(id.as_str()).is_some())
    }

    async fn exists(&self, id: &WorkflowId) -> Result<bool, DomainError> {
        let workflows = self.workflows.read().await;
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
                WorkflowStepType::ChatCompletion(ChatCompletionStep::new("gpt-4", "test")),
            ),
        )
    }

    #[tokio::test]
    async fn test_create_and_get() {
        let repo = InMemoryWorkflowRepository::new();
        let workflow = create_test_workflow("test-1");

        let created = repo.create(workflow.clone()).await.unwrap();
        assert_eq!(created.id().as_str(), "test-1");

        let retrieved = repo.get(created.id()).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name(), "Test test-1");
    }

    #[tokio::test]
    async fn test_create_duplicate() {
        let workflow = create_test_workflow("test-1");
        let repo = InMemoryWorkflowRepository::with_workflows(vec![workflow.clone()]);

        let result = repo.create(workflow).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }

    #[tokio::test]
    async fn test_update() {
        let mut workflow = create_test_workflow("test-1");
        let repo = InMemoryWorkflowRepository::with_workflows(vec![workflow.clone()]);

        workflow.set_name("Updated Name");
        let updated = repo.update(workflow).await.unwrap();
        assert_eq!(updated.name(), "Updated Name");

        let retrieved = repo.get(&WorkflowId::new("test-1").unwrap()).await.unwrap();
        assert_eq!(retrieved.unwrap().name(), "Updated Name");
    }

    #[tokio::test]
    async fn test_update_not_found() {
        let repo = InMemoryWorkflowRepository::new();
        let workflow = create_test_workflow("nonexistent");

        let result = repo.update(workflow).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[tokio::test]
    async fn test_delete() {
        let workflow = create_test_workflow("test-1");
        let id = workflow.id().clone();
        let repo = InMemoryWorkflowRepository::with_workflows(vec![workflow]);

        assert!(repo.exists(&id).await.unwrap());

        let deleted = repo.delete(&id).await.unwrap();
        assert!(deleted);
        assert!(!repo.exists(&id).await.unwrap());

        // Second delete returns false
        let deleted = repo.delete(&id).await.unwrap();
        assert!(!deleted);
    }

    #[tokio::test]
    async fn test_list() {
        let repo = InMemoryWorkflowRepository::with_workflows(vec![
            create_test_workflow("test-1"),
            create_test_workflow("test-2"),
        ]);

        let workflows = repo.list().await.unwrap();
        assert_eq!(workflows.len(), 2);
    }

    #[tokio::test]
    async fn test_list_enabled() {
        let disabled = create_test_workflow("disabled").with_enabled(false);
        let enabled = create_test_workflow("enabled").with_enabled(true);

        let repo = InMemoryWorkflowRepository::with_workflows(vec![disabled, enabled]);

        let all = repo.list().await.unwrap();
        assert_eq!(all.len(), 2);

        let enabled_only = repo.list_enabled().await.unwrap();
        assert_eq!(enabled_only.len(), 1);
        assert_eq!(enabled_only[0].id().as_str(), "enabled");
    }

    #[tokio::test]
    async fn test_concurrent_access() {
        let repo = Arc::new(InMemoryWorkflowRepository::new());

        let handles: Vec<_> = (0..10)
            .map(|i| {
                let repo = repo.clone();
                tokio::spawn(async move {
                    let workflow = create_test_workflow(&format!("test-{}", i));
                    repo.create(workflow).await
                })
            })
            .collect();

        for handle in handles {
            let result = handle.await.unwrap();
            assert!(result.is_ok());
        }

        let workflows = repo.list().await.unwrap();
        assert_eq!(workflows.len(), 10);
    }
}
