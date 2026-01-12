//! Workflow service - CRUD operations for workflows

use std::sync::Arc;

use crate::domain::storage::Storage;
use crate::domain::{
    DomainError, Workflow, WorkflowExecutor, WorkflowId, WorkflowResult,
    WorkflowStep, WorkflowStepType,
};

/// Request to create a new workflow
#[derive(Debug, Clone)]
pub struct CreateWorkflowRequest {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub input_schema: Option<serde_json::Value>,
    pub steps: Vec<WorkflowStep>,
    pub enabled: bool,
}

impl CreateWorkflowRequest {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: None,
            input_schema: None,
            steps: Vec::new(),
            enabled: true,
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_input_schema(mut self, schema: serde_json::Value) -> Self {
        self.input_schema = Some(schema);
        self
    }

    pub fn with_steps(mut self, steps: Vec<WorkflowStep>) -> Self {
        self.steps = steps;
        self
    }

    pub fn with_step(mut self, step: WorkflowStep) -> Self {
        self.steps.push(step);
        self
    }

    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

/// Request to update an existing workflow
#[derive(Debug, Clone, Default)]
pub struct UpdateWorkflowRequest {
    pub name: Option<String>,
    pub description: Option<Option<String>>,
    pub input_schema: Option<Option<serde_json::Value>>,
    pub steps: Option<Vec<WorkflowStep>>,
    pub enabled: Option<bool>,
}

impl UpdateWorkflowRequest {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn with_description(mut self, description: Option<String>) -> Self {
        self.description = Some(description);
        self
    }

    pub fn with_input_schema(mut self, schema: Option<serde_json::Value>) -> Self {
        self.input_schema = Some(schema);
        self
    }

    pub fn with_steps(mut self, steps: Vec<WorkflowStep>) -> Self {
        self.steps = Some(steps);
        self
    }

    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = Some(enabled);
        self
    }
}

/// Workflow service for CRUD operations
pub struct WorkflowService {
    storage: Arc<dyn Storage<Workflow>>,
    executor: Arc<dyn WorkflowExecutor>,
}

impl std::fmt::Debug for WorkflowService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WorkflowService").finish()
    }
}

impl WorkflowService {
    /// Create a new workflow service
    pub fn new(storage: Arc<dyn Storage<Workflow>>, executor: Arc<dyn WorkflowExecutor>) -> Self {
        Self { storage, executor }
    }

    /// Get a workflow by ID
    pub async fn get(&self, id: &str) -> Result<Option<Workflow>, DomainError> {
        let workflow_id = self.parse_id(id)?;
        self.storage.get(&workflow_id).await
    }

    /// List all workflows
    pub async fn list(&self) -> Result<Vec<Workflow>, DomainError> {
        self.storage.list().await
    }

    /// List only enabled workflows
    pub async fn list_enabled(&self) -> Result<Vec<Workflow>, DomainError> {
        let workflows = self.storage.list().await?;
        Ok(workflows.into_iter().filter(|w| w.is_enabled()).collect())
    }

    /// Create a new workflow
    pub async fn create(&self, request: CreateWorkflowRequest) -> Result<Workflow, DomainError> {
        let workflow_id = self.parse_id(&request.id)?;

        // Check for duplicate
        if self.storage.exists(&workflow_id).await? {
            return Err(DomainError::conflict(format!(
                "Workflow '{}' already exists",
                request.id
            )));
        }

        // Validate steps
        self.validate_steps(&request.steps)?;

        // Build workflow
        let mut workflow = Workflow::new(workflow_id, request.name);

        if let Some(desc) = request.description {
            workflow = workflow.with_description(desc);
        }

        if let Some(schema) = request.input_schema {
            workflow = workflow.with_input_schema(schema);
        }

        workflow = workflow.with_steps(request.steps).with_enabled(request.enabled);

        self.storage.create(workflow).await
    }

    /// Update an existing workflow
    pub async fn update(
        &self,
        id: &str,
        request: UpdateWorkflowRequest,
    ) -> Result<Workflow, DomainError> {
        let workflow_id = self.parse_id(id)?;

        let mut workflow = self
            .storage
            .get(&workflow_id)
            .await?
            .ok_or_else(|| DomainError::not_found(format!("Workflow '{}' not found", id)))?;

        // Apply updates
        if let Some(name) = request.name {
            workflow.set_name(name);
        }

        if let Some(description) = request.description {
            workflow.set_description(description);
        }

        if let Some(schema) = request.input_schema {
            workflow.set_input_schema(schema);
        }

        if let Some(steps) = request.steps {
            self.validate_steps(&steps)?;
            workflow.set_steps(steps);
        }

        if let Some(enabled) = request.enabled {
            workflow.set_enabled(enabled);
        }

        self.storage.update(workflow).await
    }

    /// Delete a workflow
    pub async fn delete(&self, id: &str) -> Result<bool, DomainError> {
        let workflow_id = self.parse_id(id)?;
        self.storage.delete(&workflow_id).await
    }

    /// Check if a workflow exists
    pub async fn exists(&self, id: &str) -> Result<bool, DomainError> {
        let workflow_id = self.parse_id(id)?;
        self.storage.exists(&workflow_id).await
    }

    /// Enable a workflow
    pub async fn enable(&self, id: &str) -> Result<Workflow, DomainError> {
        self.update(id, UpdateWorkflowRequest::new().with_enabled(true))
            .await
    }

    /// Disable a workflow
    pub async fn disable(&self, id: &str) -> Result<Workflow, DomainError> {
        self.update(id, UpdateWorkflowRequest::new().with_enabled(false))
            .await
    }

    /// Execute a workflow with the given input
    pub async fn execute(&self, id: &str, input: serde_json::Value) -> Result<WorkflowResult, DomainError> {
        let workflow_id = self.parse_id(id)?;

        let workflow = self
            .storage
            .get(&workflow_id)
            .await?
            .ok_or_else(|| DomainError::not_found(format!("Workflow '{}' not found", id)))?;

        if !workflow.is_enabled() {
            return Err(DomainError::validation(format!(
                "Workflow '{}' is disabled",
                id
            )));
        }

        self.executor
            .execute(&workflow, input)
            .await
            .map_err(|e| DomainError::internal(e.to_string()))
    }

    /// Parse and validate a workflow ID
    fn parse_id(&self, id: &str) -> Result<WorkflowId, DomainError> {
        WorkflowId::new(id).map_err(|e| DomainError::validation(e.to_string()))
    }

    /// Validate workflow steps
    fn validate_steps(&self, steps: &[WorkflowStep]) -> Result<(), DomainError> {
        // Check for empty steps
        if steps.is_empty() {
            return Err(DomainError::validation(
                "Workflow must have at least one step",
            ));
        }

        // Check for unique step names
        let mut seen_names = std::collections::HashSet::new();

        for step in steps {
            if !seen_names.insert(step.name()) {
                return Err(DomainError::validation(format!(
                    "Duplicate step name: '{}'",
                    step.name()
                )));
            }
        }

        // Validate each step
        for step in steps {
            self.validate_step(step)?;
        }

        Ok(())
    }

    /// Validate a single step
    fn validate_step(&self, step: &WorkflowStep) -> Result<(), DomainError> {
        // Validate step name
        if step.name().is_empty() {
            return Err(DomainError::validation("Step name cannot be empty"));
        }

        if step.name().len() > 50 {
            return Err(DomainError::validation("Step name too long (max 50 characters)"));
        }

        // Validate step-type specific requirements
        match step.step_type() {
            WorkflowStepType::ChatCompletion(chat_step) => {
                if chat_step.model_id.is_empty() {
                    return Err(DomainError::validation("ChatCompletion step requires model_id"));
                }

                // model_id must be configured directly, not as a variable
                if chat_step.model_id.contains("${") {
                    return Err(DomainError::validation(
                        "ChatCompletion step model_id must be configured directly, not as input variable",
                    ));
                }

                if chat_step.prompt_id.is_empty() {
                    return Err(DomainError::validation("ChatCompletion step requires prompt_id"));
                }

                // prompt_id must be configured directly, not as a variable
                if chat_step.prompt_id.contains("${") {
                    return Err(DomainError::validation(
                        "ChatCompletion step prompt_id must be configured directly, not as input variable",
                    ));
                }

                if chat_step.user_message.is_empty() {
                    return Err(DomainError::validation("ChatCompletion step requires user_message"));
                }
            }
            WorkflowStepType::KnowledgeBaseSearch(kb_step) => {
                if kb_step.knowledge_base_id.is_empty() {
                    return Err(DomainError::validation(
                        "KnowledgeBaseSearch step requires knowledge_base_id",
                    ));
                }

                // knowledge_base_id must be configured directly, not as a variable
                if kb_step.knowledge_base_id.contains("${") {
                    return Err(DomainError::validation(
                        "KnowledgeBaseSearch step knowledge_base_id must be configured directly, not as input variable",
                    ));
                }

                if kb_step.query.is_empty() {
                    return Err(DomainError::validation("KnowledgeBaseSearch step requires query"));
                }
            }
            WorkflowStepType::CragScoring(crag_step) => {
                if crag_step.input_documents.is_empty() {
                    return Err(DomainError::validation(
                        "CragScoring step requires input_documents",
                    ));
                }

                if crag_step.query.is_empty() {
                    return Err(DomainError::validation("CragScoring step requires query"));
                }

                if crag_step.model_id.is_empty() {
                    return Err(DomainError::validation("CragScoring step requires model_id"));
                }

                // model_id must be configured directly, not as a variable
                if crag_step.model_id.contains("${") {
                    return Err(DomainError::validation(
                        "CragScoring step model_id must be configured directly, not as input variable",
                    ));
                }

                if crag_step.prompt_id.is_empty() {
                    return Err(DomainError::validation("CragScoring step requires prompt_id"));
                }

                // prompt_id must be configured directly, not as a variable
                if crag_step.prompt_id.contains("${") {
                    return Err(DomainError::validation(
                        "CragScoring step prompt_id must be configured directly, not as input variable",
                    ));
                }

                if !(0.0..=1.0).contains(&crag_step.threshold) {
                    return Err(DomainError::validation(
                        "CragScoring threshold must be between 0.0 and 1.0",
                    ));
                }
            }
            WorkflowStepType::Conditional(cond_step) => {
                if cond_step.conditions.is_empty() {
                    return Err(DomainError::validation(
                        "Conditional step requires at least one condition",
                    ));
                }

                for condition in &cond_step.conditions {
                    if condition.field.is_empty() {
                        return Err(DomainError::validation("Condition field cannot be empty"));
                    }
                }
            }
            WorkflowStepType::HttpRequest(http_step) => {
                if http_step.external_api_id.is_empty() {
                    return Err(DomainError::validation(
                        "HttpRequest step requires external_api_id",
                    ));
                }

                // external_api_id must be configured directly, not as a variable
                if http_step.external_api_id.contains("${") {
                    return Err(DomainError::validation(
                        "HttpRequest step external_api_id must be configured directly, not as input variable",
                    ));
                }

                // credential_id must be configured directly if provided
                if let Some(ref cred_id) = http_step.credential_id {
                    if cred_id.contains("${") {
                        return Err(DomainError::validation(
                            "HttpRequest step credential_id must be configured directly, not as input variable",
                        ));
                    }
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::storage::mock::MockStorage;
    use crate::domain::workflow::{
        ChatCompletionStep, Condition, ConditionalAction, ConditionalStep, ConditionOperator,
        CragScoringStep, KnowledgeBaseSearchStep, StepExecutionResult, WorkflowError,
    };
    use async_trait::async_trait;

    /// Mock executor for testing
    #[derive(Debug)]
    struct MockExecutor;

    #[async_trait]
    impl WorkflowExecutor for MockExecutor {
        async fn execute(
            &self,
            _workflow: &Workflow,
            _input: serde_json::Value,
        ) -> Result<WorkflowResult, WorkflowError> {
            Ok(WorkflowResult::success(
                serde_json::json!({"mock": true}),
                vec![StepExecutionResult::success("mock-step", serde_json::json!({}), 0)],
                0,
            ))
        }
    }

    fn create_mock_executor() -> Arc<MockExecutor> {
        Arc::new(MockExecutor)
    }

    fn create_chat_step(name: &str) -> WorkflowStep {
        WorkflowStep::new(
            name,
            WorkflowStepType::ChatCompletion(ChatCompletionStep::new(
                "gpt-4",
                "system-prompt",
                "Hello",
            )),
        )
    }

    #[tokio::test]
    async fn test_create_workflow() {
        let storage = Arc::new(MockStorage::<Workflow>::new());
        let executor = create_mock_executor();
        let service = WorkflowService::new(storage, executor);

        let request = CreateWorkflowRequest::new("test-workflow", "Test Workflow")
            .with_description("A test workflow")
            .with_step(create_chat_step("step1"));

        let workflow = service.create(request).await.unwrap();

        assert_eq!(workflow.id().as_str(), "test-workflow");
        assert_eq!(workflow.name(), "Test Workflow");
        assert_eq!(workflow.description(), Some("A test workflow"));
        assert_eq!(workflow.step_count(), 1);
    }

    #[tokio::test]
    async fn test_create_duplicate() {
        let existing = Workflow::new(WorkflowId::new("existing").unwrap(), "Existing");
        let storage = Arc::new(MockStorage::<Workflow>::new().with_entity(existing));
        let executor = create_mock_executor();
        let service = WorkflowService::new(storage, executor);

        let request = CreateWorkflowRequest::new("existing", "New");
        let result = service.create(request).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }

    #[tokio::test]
    async fn test_get_workflow() {
        let workflow =
            Workflow::new(WorkflowId::new("test").unwrap(), "Test").with_step(create_chat_step("s1"));

        let storage = Arc::new(MockStorage::<Workflow>::new().with_entity(workflow));
        let executor = create_mock_executor();
        let service = WorkflowService::new(storage, executor);

        let retrieved = service.get("test").await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name(), "Test");

        let not_found = service.get("nonexistent").await.unwrap();
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_update_workflow() {
        let workflow = Workflow::new(WorkflowId::new("test").unwrap(), "Original")
            .with_step(create_chat_step("s1"));

        let storage = Arc::new(MockStorage::<Workflow>::new().with_entity(workflow));
        let executor = create_mock_executor();
        let service = WorkflowService::new(storage, executor);

        let request = UpdateWorkflowRequest::new()
            .with_name("Updated")
            .with_description(Some("New description".to_string()));

        let updated = service.update("test", request).await.unwrap();

        assert_eq!(updated.name(), "Updated");
        assert_eq!(updated.description(), Some("New description"));
    }

    #[tokio::test]
    async fn test_update_not_found() {
        let storage = Arc::new(MockStorage::<Workflow>::new());
        let executor = create_mock_executor();
        let service = WorkflowService::new(storage, executor);

        let request = UpdateWorkflowRequest::new().with_name("New");
        let result = service.update("nonexistent", request).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[tokio::test]
    async fn test_delete_workflow() {
        let workflow = Workflow::new(WorkflowId::new("test").unwrap(), "Test");
        let storage = Arc::new(MockStorage::<Workflow>::new().with_entity(workflow));
        let executor = create_mock_executor();
        let service = WorkflowService::new(storage, executor);

        let deleted = service.delete("test").await.unwrap();
        assert!(deleted);

        let exists = service.exists("test").await.unwrap();
        assert!(!exists);
    }

    #[tokio::test]
    async fn test_enable_disable() {
        let workflow = Workflow::new(WorkflowId::new("test").unwrap(), "Test").with_enabled(true);

        let storage = Arc::new(MockStorage::<Workflow>::new().with_entity(workflow));
        let executor = create_mock_executor();
        let service = WorkflowService::new(storage, executor);

        let disabled = service.disable("test").await.unwrap();
        assert!(!disabled.is_enabled());

        let enabled = service.enable("test").await.unwrap();
        assert!(enabled.is_enabled());
    }

    #[tokio::test]
    async fn test_list_workflows() {
        let storage = Arc::new(
            MockStorage::<Workflow>::new()
                .with_entity(Workflow::new(WorkflowId::new("w1").unwrap(), "W1"))
                .with_entity(Workflow::new(WorkflowId::new("w2").unwrap(), "W2")),
        );
        let executor = create_mock_executor();
        let service = WorkflowService::new(storage, executor);

        let workflows = service.list().await.unwrap();
        assert_eq!(workflows.len(), 2);
    }

    #[tokio::test]
    async fn test_validate_empty_steps_create() {
        let storage = Arc::new(MockStorage::<Workflow>::new());
        let executor = create_mock_executor();
        let service = WorkflowService::new(storage, executor);

        let request = CreateWorkflowRequest::new("test", "Test"); // No steps

        let result = service.create(request).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("must have at least one step"));
    }

    #[tokio::test]
    async fn test_validate_empty_steps_update() {
        let workflow = Workflow::new(WorkflowId::new("test").unwrap(), "Test")
            .with_step(create_chat_step("s1"));
        let storage = Arc::new(MockStorage::<Workflow>::new().with_entity(workflow));
        let executor = create_mock_executor();
        let service = WorkflowService::new(storage, executor);

        // Try to update with empty steps
        let request = UpdateWorkflowRequest::new().with_steps(vec![]);

        let result = service.update("test", request).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("must have at least one step"));
    }

    #[tokio::test]
    async fn test_validate_duplicate_step_names() {
        let storage = Arc::new(MockStorage::<Workflow>::new());
        let executor = create_mock_executor();
        let service = WorkflowService::new(storage, executor);

        let request = CreateWorkflowRequest::new("test", "Test")
            .with_step(create_chat_step("duplicate"))
            .with_step(create_chat_step("duplicate"));

        let result = service.create(request).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Duplicate step name"));
    }

    #[tokio::test]
    async fn test_validate_chat_step() {
        let storage = Arc::new(MockStorage::<Workflow>::new());
        let executor = create_mock_executor();
        let service = WorkflowService::new(storage, executor);

        // Missing model_id
        let step = WorkflowStep::new(
            "test",
            WorkflowStepType::ChatCompletion(ChatCompletionStep::new("", "sys-prompt", "Hello")),
        );
        let request = CreateWorkflowRequest::new("test", "Test").with_step(step);
        let result = service.create(request).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("requires model_id"));

        // Missing prompt_id
        let step = WorkflowStep::new(
            "test",
            WorkflowStepType::ChatCompletion(ChatCompletionStep::new("gpt-4", "", "Hello")),
        );
        let request = CreateWorkflowRequest::new("test2", "Test").with_step(step);
        let result = service.create(request).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("requires prompt_id"));

        // Missing user_message
        let step = WorkflowStep::new(
            "test",
            WorkflowStepType::ChatCompletion(ChatCompletionStep::new("gpt-4", "sys-prompt", "")),
        );
        let request = CreateWorkflowRequest::new("test3", "Test").with_step(step);
        let result = service.create(request).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("requires user_message"));
    }

    #[tokio::test]
    async fn test_validate_kb_step() {
        let storage = Arc::new(MockStorage::<Workflow>::new());
        let executor = create_mock_executor();
        let service = WorkflowService::new(storage, executor);

        let step = WorkflowStep::new(
            "test",
            WorkflowStepType::KnowledgeBaseSearch(KnowledgeBaseSearchStep::new("", "query")),
        );
        let request = CreateWorkflowRequest::new("test", "Test").with_step(step);
        let result = service.create(request).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("requires knowledge_base_id"));
    }

    #[tokio::test]
    async fn test_validate_resource_ids_not_variables() {
        let storage = Arc::new(MockStorage::<Workflow>::new());
        let executor = create_mock_executor();
        let service = WorkflowService::new(storage, executor);

        // KB step with variable reference in knowledge_base_id
        let step = WorkflowStep::new(
            "test",
            WorkflowStepType::KnowledgeBaseSearch(KnowledgeBaseSearchStep::new(
                "${request:kb_id}",
                "query",
            )),
        );
        let request = CreateWorkflowRequest::new("test1", "Test").with_step(step);
        let result = service.create(request).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("must be configured directly"));

        // Chat step with variable reference in model_id
        let step = WorkflowStep::new(
            "test",
            WorkflowStepType::ChatCompletion(ChatCompletionStep::new(
                "${request:model}",
                "prompt-id",
                "Hello",
            )),
        );
        let request = CreateWorkflowRequest::new("test2", "Test").with_step(step);
        let result = service.create(request).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("model_id must be configured directly"));

        // Chat step with variable reference in prompt_id
        let step = WorkflowStep::new(
            "test",
            WorkflowStepType::ChatCompletion(ChatCompletionStep::new(
                "gpt-4o",
                "${request:prompt}",
                "Hello",
            )),
        );
        let request = CreateWorkflowRequest::new("test3", "Test").with_step(step);
        let result = service.create(request).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("prompt_id must be configured directly"));
    }

    #[tokio::test]
    async fn test_validate_crag_step() {
        let storage = Arc::new(MockStorage::<Workflow>::new());
        let executor = create_mock_executor();
        let service = WorkflowService::new(storage, executor);

        // Missing model_id
        let step = CragScoringStep::new("${step:search:docs}", "query", "", "crag-prompt");
        let request = CreateWorkflowRequest::new("test", "Test")
            .with_step(WorkflowStep::new("test", WorkflowStepType::CragScoring(step)));
        let result = service.create(request).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("requires model_id"));

        // Missing prompt_id
        let step = CragScoringStep::new("${step:search:docs}", "query", "gpt-4o", "");
        let request = CreateWorkflowRequest::new("test2", "Test")
            .with_step(WorkflowStep::new("test", WorkflowStepType::CragScoring(step)));
        let result = service.create(request).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("requires prompt_id"));

        // Invalid threshold
        let mut step = CragScoringStep::new("${step:search:docs}", "query", "gpt-4o", "crag-prompt");
        step.threshold = 1.5; // Invalid
        let request = CreateWorkflowRequest::new("test3", "Test")
            .with_step(WorkflowStep::new("test", WorkflowStepType::CragScoring(step)));
        let result = service.create(request).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("threshold must be"));
    }

    #[tokio::test]
    async fn test_validate_conditional_step() {
        let storage = Arc::new(MockStorage::<Workflow>::new());
        let executor = create_mock_executor();
        let service = WorkflowService::new(storage, executor);

        // Empty conditions
        let step = WorkflowStep::new(
            "test",
            WorkflowStepType::Conditional(ConditionalStep::new(vec![])),
        );
        let request = CreateWorkflowRequest::new("test", "Test").with_step(step);
        let result = service.create(request).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("at least one condition"));

        // Condition with empty field
        let condition = Condition::new("", ConditionOperator::IsEmpty, ConditionalAction::Continue);
        let step = WorkflowStep::new(
            "test",
            WorkflowStepType::Conditional(ConditionalStep::new(vec![condition])),
        );
        let request = CreateWorkflowRequest::new("test2", "Test").with_step(step);
        let result = service.create(request).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("field cannot be empty"));
    }

    #[tokio::test]
    async fn test_invalid_workflow_id() {
        let storage = Arc::new(MockStorage::<Workflow>::new());
        let executor = create_mock_executor();
        let service = WorkflowService::new(storage, executor);

        let request = CreateWorkflowRequest::new("-invalid-id-", "Test");
        let result = service.create(request).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_execute_workflow() {
        let workflow = Workflow::new(WorkflowId::new("test").unwrap(), "Test")
            .with_step(create_chat_step("s1"))
            .with_enabled(true);

        let storage = Arc::new(MockStorage::<Workflow>::new().with_entity(workflow));
        let executor = create_mock_executor();
        let service = WorkflowService::new(storage, executor);

        let result = service.execute("test", serde_json::json!({})).await.unwrap();
        assert!(result.success);
    }

    #[tokio::test]
    async fn test_execute_disabled_workflow() {
        let workflow = Workflow::new(WorkflowId::new("test").unwrap(), "Test")
            .with_step(create_chat_step("s1"))
            .with_enabled(false);

        let storage = Arc::new(MockStorage::<Workflow>::new().with_entity(workflow));
        let executor = create_mock_executor();
        let service = WorkflowService::new(storage, executor);

        let result = service.execute("test", serde_json::json!({})).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("is disabled"));
    }

    #[tokio::test]
    async fn test_execute_not_found() {
        let storage = Arc::new(MockStorage::<Workflow>::new());
        let executor = create_mock_executor();
        let service = WorkflowService::new(storage, executor);

        let result = service.execute("nonexistent", serde_json::json!({})).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }
}
