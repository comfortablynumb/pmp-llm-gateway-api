//! Test case service - CRUD operations and execution for test cases

use std::sync::Arc;
use std::time::Instant;

use serde::{Deserialize, Serialize};

use crate::domain::test_case::{
    AssertionCriteria, AssertionEvaluator, ModelPromptInput, TestCase, TestCaseId, TestCaseInput,
    TestCaseQuery, TestCaseRepository, TestCaseResult, TestCaseResultQuery, TestCaseResultRepository,
    TokenUsage, WorkflowInput,
};
use crate::domain::{DomainError, LlmRequest, Message};

use super::super::plugin::ProviderRouter;
use crate::api::state::{CredentialServiceTrait, ModelServiceTrait, PromptServiceTrait, WorkflowServiceTrait};

/// Request to create a new test case
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTestCaseRequest {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub input: TestCaseInputRequest,
    pub assertions: Vec<AssertionCriteria>,
    pub tags: Vec<String>,
    pub enabled: bool,
}

/// Input configuration for a test case request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TestCaseInputRequest {
    ModelPrompt(ModelPromptInput),
    Workflow(WorkflowInput),
}

/// Request to update an existing test case
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateTestCaseRequest {
    pub name: Option<String>,
    pub description: Option<Option<String>>,
    pub input: Option<TestCaseInputRequest>,
    pub assertions: Option<Vec<AssertionCriteria>>,
    pub tags: Option<Vec<String>>,
    pub enabled: Option<bool>,
}

/// Response from executing a test case
#[derive(Debug, Clone, Serialize)]
pub struct ExecuteTestCaseResponse {
    pub test_case_id: String,
    pub test_case_name: String,
    pub passed: bool,
    pub output: Option<String>,
    pub assertion_results: Vec<AssertionResultResponse>,
    pub execution_time_ms: u64,
    pub tokens_used: Option<TokenUsage>,
    pub error: Option<String>,
}

/// Assertion result in response
#[derive(Debug, Clone, Serialize)]
pub struct AssertionResultResponse {
    pub name: String,
    pub passed: bool,
    pub operator: String,
    pub expected: String,
    pub actual: Option<String>,
    pub error: Option<String>,
}

/// Dependencies for test case service
pub struct TestCaseServiceDeps {
    pub model_service: Arc<dyn ModelServiceTrait>,
    pub prompt_service: Arc<dyn PromptServiceTrait>,
    pub workflow_service: Arc<dyn WorkflowServiceTrait>,
    pub credential_service: Arc<dyn CredentialServiceTrait>,
    pub provider_router: Arc<ProviderRouter>,
}

/// Test case service for CRUD operations and execution
pub struct TestCaseService<R, RR> {
    repository: Arc<R>,
    result_repository: Arc<RR>,
    deps: TestCaseServiceDeps,
}

impl<R: TestCaseRepository, RR: TestCaseResultRepository> TestCaseService<R, RR> {
    /// Create a new test case service
    pub fn new(
        repository: Arc<R>,
        result_repository: Arc<RR>,
        deps: TestCaseServiceDeps,
    ) -> Self {
        Self {
            repository,
            result_repository,
            deps,
        }
    }

    /// Get a test case by ID
    pub async fn get(&self, id: &str) -> Result<Option<TestCase>, DomainError> {
        let test_case_id = self.parse_id(id)?;
        self.repository.get(&test_case_id).await
    }

    /// Get a test case by ID, returning an error if not found
    pub async fn get_required(&self, id: &str) -> Result<TestCase, DomainError> {
        self.get(id)
            .await?
            .ok_or_else(|| DomainError::not_found(format!("Test case '{}' not found", id)))
    }

    /// List all test cases
    pub async fn list(&self, query: &TestCaseQuery) -> Result<Vec<TestCase>, DomainError> {
        self.repository.list(query).await
    }

    /// Count test cases matching query
    pub async fn count(&self, query: &TestCaseQuery) -> Result<usize, DomainError> {
        self.repository.count(query).await
    }

    /// Create a new test case
    pub async fn create(&self, request: CreateTestCaseRequest) -> Result<TestCase, DomainError> {
        let test_case_id = self.parse_id(&request.id)?;

        // Check for duplicate
        if self.repository.exists(&test_case_id).await? {
            return Err(DomainError::conflict(format!(
                "Test case '{}' already exists",
                request.id
            )));
        }

        // Clone name before moving input
        let name = request.name.clone();

        // Build test case based on input type
        let test_case = match &request.input {
            TestCaseInputRequest::ModelPrompt(input) => {
                self.validate_model_prompt_input(input).await?;
                TestCase::model_prompt(test_case_id, name, input.clone())
            }
            TestCaseInputRequest::Workflow(input) => {
                self.validate_workflow_input(input).await?;
                TestCase::workflow(test_case_id, name, input.clone())
            }
        };

        let test_case = self.apply_common_fields(test_case, &request);
        self.repository.save(&test_case).await?;
        Ok(test_case)
    }

    fn apply_common_fields(
        &self,
        mut test_case: TestCase,
        request: &CreateTestCaseRequest,
    ) -> TestCase {
        if let Some(ref desc) = request.description {
            test_case = test_case.with_description(desc);
        }
        test_case = test_case.with_assertions(request.assertions.clone());
        test_case = test_case.with_tags(request.tags.clone());
        test_case = test_case.with_enabled(request.enabled);
        test_case
    }

    /// Update an existing test case
    pub async fn update(
        &self,
        id: &str,
        request: UpdateTestCaseRequest,
    ) -> Result<TestCase, DomainError> {
        let mut test_case = self.get_required(id).await?;

        if let Some(name) = request.name {
            test_case.set_name(name);
        }

        if let Some(description) = request.description {
            test_case.set_description(description);
        }

        if let Some(input) = request.input {
            let new_input = match input {
                TestCaseInputRequest::ModelPrompt(mp) => {
                    self.validate_model_prompt_input(&mp).await?;
                    TestCaseInput::ModelPrompt(mp)
                }
                TestCaseInputRequest::Workflow(wf) => {
                    self.validate_workflow_input(&wf).await?;
                    TestCaseInput::Workflow(wf)
                }
            };
            test_case.set_input(new_input);
        }

        if let Some(assertions) = request.assertions {
            test_case.set_assertions(assertions);
        }

        if let Some(tags) = request.tags {
            test_case.set_tags(tags);
        }

        if let Some(enabled) = request.enabled {
            test_case.set_enabled(enabled);
        }

        self.repository.save(&test_case).await?;
        Ok(test_case)
    }

    /// Delete a test case
    pub async fn delete(&self, id: &str) -> Result<bool, DomainError> {
        let test_case_id = self.parse_id(id)?;

        // Also delete associated results
        self.result_repository
            .delete_for_test_case(&test_case_id)
            .await?;

        self.repository.delete(&test_case_id).await
    }

    /// Execute a test case and return results
    pub async fn execute(&self, id: &str) -> Result<ExecuteTestCaseResponse, DomainError> {
        let test_case = self.get_required(id).await?;

        if !test_case.is_enabled() {
            return Err(DomainError::validation(format!(
                "Test case '{}' is disabled",
                id
            )));
        }

        let start = Instant::now();

        let (output, tokens, error) = match test_case.input() {
            TestCaseInput::ModelPrompt(input) => self.execute_model_prompt(input).await,
            TestCaseInput::Workflow(input) => self.execute_workflow(input).await,
        };

        let execution_time_ms = start.elapsed().as_millis() as u64;

        // Evaluate assertions if we have output
        let assertion_results = if let Some(ref output_str) = output {
            AssertionEvaluator::evaluate_all(test_case.assertions(), output_str)
        } else {
            Vec::new()
        };

        // Determine if passed
        let passed = error.is_none() && assertion_results.iter().all(|r| r.passed);

        // Create and store result
        let result = if let Some(ref err) = error {
            TestCaseResult::execution_error(test_case.id().clone(), err, execution_time_ms)
        } else {
            let mut result = TestCaseResult::success(
                test_case.id().clone(),
                output.clone().unwrap_or_default(),
                assertion_results.clone(),
                execution_time_ms,
            );
            if let Some(t) = tokens.clone() {
                result = result.with_tokens(t);
            }
            result
        };

        self.result_repository.save(&result).await?;

        // Build response
        let assertion_responses: Vec<AssertionResultResponse> = assertion_results
            .into_iter()
            .map(|r| AssertionResultResponse {
                name: r.name,
                passed: r.passed,
                operator: format!("{}", r.operator),
                expected: r.expected,
                actual: r.actual,
                error: r.error,
            })
            .collect();

        Ok(ExecuteTestCaseResponse {
            test_case_id: test_case.id().to_string(),
            test_case_name: test_case.name().to_string(),
            passed,
            output,
            assertion_results: assertion_responses,
            execution_time_ms,
            tokens_used: tokens,
            error,
        })
    }

    async fn execute_model_prompt(
        &self,
        input: &ModelPromptInput,
    ) -> (Option<String>, Option<TokenUsage>, Option<String>) {
        // Get model
        let model = match self.deps.model_service.get(&input.model_id).await {
            Ok(Some(m)) => m,
            Ok(None) => {
                return (
                    None,
                    None,
                    Some(format!("Model '{}' not found", input.model_id)),
                );
            }
            Err(e) => return (None, None, Some(format!("Failed to get model: {}", e))),
        };

        // Get credential
        let stored_credential = match self.deps.credential_service.get(model.credential_id()).await {
            Ok(Some(c)) => c,
            Ok(None) => {
                return (
                    None,
                    None,
                    Some(format!("Credential '{}' not found", model.credential_id())),
                );
            }
            Err(e) => return (None, None, Some(format!("Failed to get credential: {}", e))),
        };

        if !stored_credential.is_enabled() {
            return (
                None,
                None,
                Some(format!("Credential '{}' is disabled", model.credential_id())),
            );
        }

        let credential = stored_credential.to_credential();

        // Render system prompt if provided
        let system_message = if let Some(ref prompt_id) = input.prompt_id {
            match self
                .deps
                .prompt_service
                .render(prompt_id, &input.variables)
                .await
            {
                Ok(rendered) => Some(rendered),
                Err(e) => {
                    return (
                        None,
                        None,
                        Some(format!("Failed to render prompt: {}", e)),
                    );
                }
            }
        } else {
            None
        };

        // Build messages
        let mut messages = Vec::new();
        if let Some(sys) = system_message {
            messages.push(Message::system(sys));
        }
        messages.push(Message::user(&input.user_message));

        // Build LLM request
        let mut llm_request = LlmRequest::new(messages);

        if let Some(temp) = input.temperature {
            llm_request.temperature = Some(temp);
        }

        if let Some(max_tokens) = input.max_tokens {
            llm_request.max_tokens = Some(max_tokens);
        }

        // Get provider and execute
        let provider = match self
            .deps
            .provider_router
            .get_provider(&model, &credential)
            .await
        {
            Ok(p) => p,
            Err(e) => return (None, None, Some(format!("Failed to get provider: {}", e))),
        };

        match provider.chat(model.provider_model(), llm_request).await {
            Ok(response) => {
                let output = response.content().unwrap_or("").to_string();
                let tokens = response.usage.as_ref().map(|u| TokenUsage {
                    prompt_tokens: u.prompt_tokens,
                    completion_tokens: u.completion_tokens,
                    total_tokens: u.total_tokens,
                });
                (Some(output), tokens, None)
            }
            Err(e) => (None, None, Some(format!("LLM call failed: {}", e))),
        }
    }

    async fn execute_workflow(
        &self,
        input: &WorkflowInput,
    ) -> (Option<String>, Option<TokenUsage>, Option<String>) {
        match self
            .deps
            .workflow_service
            .execute(&input.workflow_id, input.input.clone())
            .await
        {
            Ok(result) => {
                let output = serde_json::to_string_pretty(&result.output)
                    .unwrap_or_else(|_| "{}".to_string());
                (Some(output), None, None)
            }
            Err(e) => (None, None, Some(format!("Workflow execution failed: {}", e))),
        }
    }

    /// Get results for a test case
    pub async fn get_results(
        &self,
        id: &str,
        query: &TestCaseResultQuery,
    ) -> Result<Vec<TestCaseResult>, DomainError> {
        let _ = self.get_required(id).await?;
        self.result_repository.list(query).await
    }

    /// Get latest result for a test case
    pub async fn get_latest_result(
        &self,
        id: &str,
    ) -> Result<Option<TestCaseResult>, DomainError> {
        let test_case_id = self.parse_id(id)?;
        self.result_repository.get_latest(&test_case_id).await
    }

    fn parse_id(&self, id: &str) -> Result<TestCaseId, DomainError> {
        TestCaseId::new(id).map_err(|e| DomainError::validation(e.to_string()))
    }

    async fn validate_model_prompt_input(&self, input: &ModelPromptInput) -> Result<(), DomainError> {
        // Check model exists
        if self.deps.model_service.get(&input.model_id).await?.is_none() {
            return Err(DomainError::validation(format!(
                "Model '{}' not found",
                input.model_id
            )));
        }

        // Check prompt exists if specified
        if let Some(ref prompt_id) = input.prompt_id {
            if self.deps.prompt_service.get(prompt_id).await?.is_none() {
                return Err(DomainError::validation(format!(
                    "Prompt '{}' not found",
                    prompt_id
                )));
            }
        }

        Ok(())
    }

    async fn validate_workflow_input(&self, input: &WorkflowInput) -> Result<(), DomainError> {
        // Check workflow exists
        if self.deps.workflow_service.get(&input.workflow_id).await?.is_none() {
            return Err(DomainError::validation(format!(
                "Workflow '{}' not found",
                input.workflow_id
            )));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::test_case::TestCaseType;
    use crate::domain::{Model, Prompt, StoredCredential, Workflow, WorkflowResult};
    use crate::infrastructure::services::{
        CreateModelRequest, CreatePromptRequest, CreateWorkflowRequest, UpdateModelRequest,
        UpdatePromptRequest, UpdateWorkflowRequest,
    };
    use crate::infrastructure::test_case::{
        InMemoryTestCaseRepository, InMemoryTestCaseResultRepository,
    };
    use crate::infrastructure::credentials::{CreateCredentialRequest, UpdateCredentialRequest};
    use async_trait::async_trait;
    use serde_json::Value;
    use std::collections::HashMap;
    use std::sync::Mutex;

    // Mock model service
    struct MockModelService {
        models: Mutex<HashMap<String, Model>>,
    }

    impl MockModelService {
        fn new() -> Self {
            Self {
                models: Mutex::new(HashMap::new()),
            }
        }

        fn add_model(&self, id: &str) {
            let model = Model::new(
                crate::domain::ModelId::new(id).unwrap(),
                "Test Model",
                crate::domain::CredentialType::OpenAi,
                "gpt-4",
                "cred-1",
            );
            self.models.lock().unwrap().insert(id.to_string(), model);
        }
    }

    #[async_trait]
    impl ModelServiceTrait for MockModelService {
        async fn get(&self, id: &str) -> Result<Option<Model>, DomainError> {
            Ok(self.models.lock().unwrap().get(id).cloned())
        }

        async fn list(&self) -> Result<Vec<Model>, DomainError> {
            Ok(self.models.lock().unwrap().values().cloned().collect())
        }

        async fn create(&self, _request: CreateModelRequest) -> Result<Model, DomainError> {
            unimplemented!()
        }

        async fn update(
            &self,
            _id: &str,
            _request: UpdateModelRequest,
        ) -> Result<Model, DomainError> {
            unimplemented!()
        }

        async fn delete(&self, _id: &str) -> Result<bool, DomainError> {
            unimplemented!()
        }
    }

    // Mock prompt service
    struct MockPromptService {
        prompts: Mutex<HashMap<String, Prompt>>,
    }

    impl MockPromptService {
        fn new() -> Self {
            Self {
                prompts: Mutex::new(HashMap::new()),
            }
        }

        #[allow(dead_code)]
        fn add_prompt(&self, id: &str) {
            let prompt = Prompt::new(
                crate::domain::PromptId::new(id).unwrap(),
                "Test Prompt",
                "You are a helpful assistant.",
            );
            self.prompts.lock().unwrap().insert(id.to_string(), prompt);
        }
    }

    #[async_trait]
    impl PromptServiceTrait for MockPromptService {
        async fn get(&self, id: &str) -> Result<Option<Prompt>, DomainError> {
            Ok(self.prompts.lock().unwrap().get(id).cloned())
        }

        async fn list(&self) -> Result<Vec<Prompt>, DomainError> {
            Ok(self.prompts.lock().unwrap().values().cloned().collect())
        }

        async fn create(&self, _request: CreatePromptRequest) -> Result<Prompt, DomainError> {
            unimplemented!()
        }

        async fn update(
            &self,
            _id: &str,
            _request: UpdatePromptRequest,
        ) -> Result<Prompt, DomainError> {
            unimplemented!()
        }

        async fn delete(&self, _id: &str) -> Result<bool, DomainError> {
            unimplemented!()
        }

        async fn render(
            &self,
            id: &str,
            _variables: &HashMap<String, String>,
        ) -> Result<String, DomainError> {
            if let Some(prompt) = self.prompts.lock().unwrap().get(id) {
                Ok(prompt.content().to_string())
            } else {
                Err(DomainError::not_found(format!("Prompt '{}' not found", id)))
            }
        }

        async fn revert(&self, _id: &str, _version: u32) -> Result<Prompt, DomainError> {
            unimplemented!()
        }
    }

    // Mock workflow service
    struct MockWorkflowService {
        workflows: Mutex<HashMap<String, Workflow>>,
    }

    impl MockWorkflowService {
        fn new() -> Self {
            Self {
                workflows: Mutex::new(HashMap::new()),
            }
        }

        fn add_workflow(&self, id: &str) {
            let workflow = Workflow::new(
                crate::domain::WorkflowId::new(id).unwrap(),
                "Test Workflow",
            );
            self.workflows
                .lock()
                .unwrap()
                .insert(id.to_string(), workflow);
        }
    }

    #[async_trait]
    impl WorkflowServiceTrait for MockWorkflowService {
        async fn get(&self, id: &str) -> Result<Option<Workflow>, DomainError> {
            Ok(self.workflows.lock().unwrap().get(id).cloned())
        }

        async fn list(&self) -> Result<Vec<Workflow>, DomainError> {
            Ok(self.workflows.lock().unwrap().values().cloned().collect())
        }

        async fn create(&self, _request: CreateWorkflowRequest) -> Result<Workflow, DomainError> {
            unimplemented!()
        }

        async fn update(
            &self,
            _id: &str,
            _request: UpdateWorkflowRequest,
        ) -> Result<Workflow, DomainError> {
            unimplemented!()
        }

        async fn delete(&self, _id: &str) -> Result<bool, DomainError> {
            unimplemented!()
        }

        async fn execute(&self, _id: &str, _input: Value) -> Result<WorkflowResult, DomainError> {
            // Return a simple result
            Ok(WorkflowResult::success(
                serde_json::json!({"result": "success"}),
                Vec::new(),
                0,
            ))
        }
    }

    // Mock credential service
    struct MockCredentialService;

    #[async_trait]
    impl CredentialServiceTrait for MockCredentialService {
        async fn get(&self, _id: &str) -> Result<Option<StoredCredential>, DomainError> {
            Ok(None)
        }

        async fn list(&self) -> Result<Vec<StoredCredential>, DomainError> {
            Ok(Vec::new())
        }

        async fn create(&self, _request: CreateCredentialRequest) -> Result<StoredCredential, DomainError> {
            unimplemented!()
        }

        async fn update(&self, _id: &str, _request: UpdateCredentialRequest) -> Result<StoredCredential, DomainError> {
            unimplemented!()
        }

        async fn delete(&self, _id: &str) -> Result<(), DomainError> {
            unimplemented!()
        }

        async fn exists(&self, _id: &str) -> Result<bool, DomainError> {
            Ok(false)
        }
    }

    fn create_deps() -> TestCaseServiceDeps {
        TestCaseServiceDeps {
            model_service: Arc::new(MockModelService::new()),
            prompt_service: Arc::new(MockPromptService::new()),
            workflow_service: Arc::new(MockWorkflowService::new()),
            credential_service: Arc::new(MockCredentialService),
            provider_router: Arc::new(ProviderRouter::new()),
        }
    }

    #[tokio::test]
    async fn test_create_model_prompt_test_case() {
        let repo = Arc::new(InMemoryTestCaseRepository::new());
        let result_repo = Arc::new(InMemoryTestCaseResultRepository::new());
        let model_service = Arc::new(MockModelService::new());

        model_service.add_model("gpt-4");

        let deps = TestCaseServiceDeps {
            model_service,
            prompt_service: Arc::new(MockPromptService::new()),
            workflow_service: Arc::new(MockWorkflowService::new()),
            credential_service: Arc::new(MockCredentialService),
            provider_router: Arc::new(ProviderRouter::new()),
        };

        let service = TestCaseService::new(repo, result_repo, deps);

        let request = CreateTestCaseRequest {
            id: "test-1".to_string(),
            name: "Test Case 1".to_string(),
            description: Some("A test case".to_string()),
            input: TestCaseInputRequest::ModelPrompt(ModelPromptInput {
                model_id: "gpt-4".to_string(),
                prompt_id: None,
                variables: HashMap::new(),
                user_message: "Hello".to_string(),
                temperature: None,
                max_tokens: None,
            }),
            assertions: vec![AssertionCriteria::contains("check", "hello")],
            tags: vec!["test".to_string()],
            enabled: true,
        };

        let result = service.create(request).await.unwrap();

        assert_eq!(result.id().as_str(), "test-1");
        assert_eq!(result.name(), "Test Case 1");
        assert_eq!(result.test_type(), &TestCaseType::ModelPrompt);
    }

    #[tokio::test]
    async fn test_create_workflow_test_case() {
        let repo = Arc::new(InMemoryTestCaseRepository::new());
        let result_repo = Arc::new(InMemoryTestCaseResultRepository::new());
        let workflow_service = Arc::new(MockWorkflowService::new());

        workflow_service.add_workflow("my-workflow");

        let deps = TestCaseServiceDeps {
            model_service: Arc::new(MockModelService::new()),
            prompt_service: Arc::new(MockPromptService::new()),
            workflow_service,
            credential_service: Arc::new(MockCredentialService),
            provider_router: Arc::new(ProviderRouter::new()),
        };

        let service = TestCaseService::new(repo, result_repo, deps);

        let request = CreateTestCaseRequest {
            id: "test-workflow-1".to_string(),
            name: "Workflow Test".to_string(),
            description: None,
            input: TestCaseInputRequest::Workflow(WorkflowInput {
                workflow_id: "my-workflow".to_string(),
                input: serde_json::json!({"query": "test"}),
            }),
            assertions: vec![],
            tags: vec![],
            enabled: true,
        };

        let result = service.create(request).await.unwrap();
        assert_eq!(result.test_type(), &TestCaseType::Workflow);
    }

    #[tokio::test]
    async fn test_validation_fails_for_missing_model() {
        let repo = Arc::new(InMemoryTestCaseRepository::new());
        let result_repo = Arc::new(InMemoryTestCaseResultRepository::new());
        let deps = create_deps();

        let service = TestCaseService::new(repo, result_repo, deps);

        let request = CreateTestCaseRequest {
            id: "test-1".to_string(),
            name: "Test".to_string(),
            description: None,
            input: TestCaseInputRequest::ModelPrompt(ModelPromptInput {
                model_id: "non-existent".to_string(),
                prompt_id: None,
                variables: HashMap::new(),
                user_message: "Hello".to_string(),
                temperature: None,
                max_tokens: None,
            }),
            assertions: vec![],
            tags: vec![],
            enabled: true,
        };

        let result = service.create(request).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[tokio::test]
    async fn test_update_test_case() {
        let repo = Arc::new(InMemoryTestCaseRepository::new());
        let result_repo = Arc::new(InMemoryTestCaseResultRepository::new());
        let model_service = Arc::new(MockModelService::new());

        model_service.add_model("gpt-4");

        let deps = TestCaseServiceDeps {
            model_service,
            prompt_service: Arc::new(MockPromptService::new()),
            workflow_service: Arc::new(MockWorkflowService::new()),
            credential_service: Arc::new(MockCredentialService),
            provider_router: Arc::new(ProviderRouter::new()),
        };

        let service = TestCaseService::new(repo, result_repo, deps);

        // Create test case
        let create_request = CreateTestCaseRequest {
            id: "test-1".to_string(),
            name: "Original Name".to_string(),
            description: None,
            input: TestCaseInputRequest::ModelPrompt(ModelPromptInput {
                model_id: "gpt-4".to_string(),
                prompt_id: None,
                variables: HashMap::new(),
                user_message: "Hello".to_string(),
                temperature: None,
                max_tokens: None,
            }),
            assertions: vec![],
            tags: vec![],
            enabled: true,
        };
        service.create(create_request).await.unwrap();

        // Update it
        let update_request = UpdateTestCaseRequest {
            name: Some("Updated Name".to_string()),
            description: Some(Some("New description".to_string())),
            enabled: Some(false),
            ..Default::default()
        };

        let updated = service.update("test-1", update_request).await.unwrap();

        assert_eq!(updated.name(), "Updated Name");
        assert_eq!(updated.description(), Some("New description"));
        assert!(!updated.is_enabled());
    }

    #[tokio::test]
    async fn test_delete_test_case() {
        let repo = Arc::new(InMemoryTestCaseRepository::new());
        let result_repo = Arc::new(InMemoryTestCaseResultRepository::new());
        let model_service = Arc::new(MockModelService::new());

        model_service.add_model("gpt-4");

        let deps = TestCaseServiceDeps {
            model_service,
            prompt_service: Arc::new(MockPromptService::new()),
            workflow_service: Arc::new(MockWorkflowService::new()),
            credential_service: Arc::new(MockCredentialService),
            provider_router: Arc::new(ProviderRouter::new()),
        };

        let service = TestCaseService::new(repo, result_repo, deps);

        // Create test case
        let create_request = CreateTestCaseRequest {
            id: "test-1".to_string(),
            name: "Test".to_string(),
            description: None,
            input: TestCaseInputRequest::ModelPrompt(ModelPromptInput {
                model_id: "gpt-4".to_string(),
                prompt_id: None,
                variables: HashMap::new(),
                user_message: "Hello".to_string(),
                temperature: None,
                max_tokens: None,
            }),
            assertions: vec![],
            tags: vec![],
            enabled: true,
        };
        service.create(create_request).await.unwrap();

        // Delete it
        let deleted = service.delete("test-1").await.unwrap();
        assert!(deleted);

        // Verify it's gone
        let found = service.get("test-1").await.unwrap();
        assert!(found.is_none());
    }
}
