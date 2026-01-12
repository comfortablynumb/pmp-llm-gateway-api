//! Workflow executor implementation

use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use serde_json::{json, Value};
use tracing::debug;

use crate::domain::knowledge_base::SearchParams;
use crate::domain::llm::ProviderResolver;
use crate::domain::storage::Storage;
use crate::domain::{
    ConditionalAction, HttpMethod, HttpRequestStep, LlmRequest, OnErrorAction, Prompt,
    StepExecutionResult, Workflow, WorkflowContext, WorkflowError, WorkflowExecutor,
    WorkflowResult, WorkflowStep, WorkflowStepType,
};
use crate::infrastructure::knowledge_base::KnowledgeBaseProviderRegistryTrait;

/// Configuration for the workflow executor
#[derive(Debug, Clone)]
pub struct WorkflowExecutorConfig {
    /// Default timeout for steps in milliseconds
    pub default_timeout_ms: u64,

    /// Maximum number of steps to execute (prevents infinite loops)
    pub max_steps: usize,
}

impl Default for WorkflowExecutorConfig {
    fn default() -> Self {
        Self {
            default_timeout_ms: 60000, // 60 seconds
            max_steps: 100,
        }
    }
}

/// Workflow executor implementation
///
/// Uses a `ProviderResolver` to dynamically select the appropriate LLM provider
/// for each model referenced in ChatCompletion steps. This allows workflows to
/// use different providers for different models based on credential configuration.
pub struct WorkflowExecutorImpl {
    /// Provider resolver for getting LLM providers per model
    provider_resolver: Arc<dyn ProviderResolver>,

    /// Prompt storage for resolving prompt_id references
    prompt_storage: Arc<dyn Storage<Prompt>>,

    /// Credential service for HTTP request step authentication
    credential_service: Arc<dyn crate::infrastructure::credentials::CredentialServiceTrait>,

    /// External API service for HTTP request step base URL and headers
    external_api_service: Arc<dyn crate::infrastructure::external_api::ExternalApiServiceTrait>,

    /// Knowledge base provider registry for KB search steps
    kb_provider_registry: Arc<dyn KnowledgeBaseProviderRegistryTrait>,

    /// Executor configuration
    config: WorkflowExecutorConfig,
}

impl std::fmt::Debug for WorkflowExecutorImpl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WorkflowExecutorImpl")
            .field("config", &self.config)
            .finish()
    }
}

impl WorkflowExecutorImpl {
    /// Create a new executor with all required dependencies
    pub fn new(
        provider_resolver: Arc<dyn ProviderResolver>,
        prompt_storage: Arc<dyn Storage<Prompt>>,
        credential_service: Arc<dyn crate::infrastructure::credentials::CredentialServiceTrait>,
        external_api_service: Arc<dyn crate::infrastructure::external_api::ExternalApiServiceTrait>,
        kb_provider_registry: Arc<dyn KnowledgeBaseProviderRegistryTrait>,
    ) -> Self {
        Self {
            provider_resolver,
            prompt_storage,
            credential_service,
            external_api_service,
            kb_provider_registry,
            config: WorkflowExecutorConfig::default(),
        }
    }

    /// Create with custom configuration
    pub fn with_config(
        provider_resolver: Arc<dyn ProviderResolver>,
        prompt_storage: Arc<dyn Storage<Prompt>>,
        credential_service: Arc<dyn crate::infrastructure::credentials::CredentialServiceTrait>,
        external_api_service: Arc<dyn crate::infrastructure::external_api::ExternalApiServiceTrait>,
        kb_provider_registry: Arc<dyn KnowledgeBaseProviderRegistryTrait>,
        config: WorkflowExecutorConfig,
    ) -> Self {
        Self {
            provider_resolver,
            prompt_storage,
            credential_service,
            external_api_service,
            kb_provider_registry,
            config,
        }
    }

    /// Resolve a prompt_id to its content
    async fn resolve_prompt(&self, prompt_id: &str) -> Result<String, WorkflowError> {
        use crate::domain::PromptId;

        let id = PromptId::new(prompt_id)
            .map_err(|e| WorkflowError::step_execution("prompt_resolution", e.to_string()))?;

        let prompt = self
            .prompt_storage
            .get(&id)
            .await
            .map_err(|e| WorkflowError::step_execution("prompt_resolution", e.to_string()))?
            .ok_or_else(|| {
                WorkflowError::step_execution(
                    "prompt_resolution",
                    format!("Prompt '{}' not found", prompt_id),
                )
            })?;

        if !prompt.is_enabled() {
            return Err(WorkflowError::step_execution(
                "prompt_resolution",
                format!("Prompt '{}' is disabled", prompt_id),
            ));
        }

        Ok(prompt.content().to_string())
    }

    /// Execute a single step
    async fn execute_step(
        &self,
        step: &WorkflowStep,
        context: &mut WorkflowContext,
    ) -> Result<Value, WorkflowError> {
        match step.step_type() {
            WorkflowStepType::ChatCompletion(chat_step) => {
                self.execute_chat_completion(chat_step, context).await
            }
            WorkflowStepType::KnowledgeBaseSearch(kb_step) => {
                self.execute_kb_search(kb_step, context).await
            }
            WorkflowStepType::CragScoring(crag_step) => {
                self.execute_crag_scoring(crag_step, context).await
            }
            WorkflowStepType::Conditional(cond_step) => {
                self.execute_conditional(cond_step, context).await
            }
            WorkflowStepType::HttpRequest(http_step) => {
                self.execute_http_request(http_step, context).await
            }
        }
    }

    /// Execute a chat completion step
    async fn execute_chat_completion(
        &self,
        step: &crate::domain::ChatCompletionStep,
        context: &WorkflowContext,
    ) -> Result<Value, WorkflowError> {
        // Resolve variable references in messages
        let user_message = context.resolve_string(&step.user_message)?;

        // Resolve prompt_id to system message content
        let system_template = self.resolve_prompt(&step.prompt_id).await?;

        // The prompt content may contain variable references, resolve them
        let system_message = context.resolve_string(&system_template)?;

        // Build request
        let mut request_builder = LlmRequest::builder();
        request_builder = request_builder.system(&system_message);
        request_builder = request_builder.user(&user_message);

        if let Some(temp) = step.temperature {
            request_builder = request_builder.temperature(temp);
        }

        if let Some(max_tokens) = step.max_tokens {
            request_builder = request_builder.max_tokens(max_tokens);
        }

        if let Some(top_p) = step.top_p {
            request_builder = request_builder.top_p(top_p);
        }

        let request = request_builder.build();

        // Resolve the provider for this model
        let provider = self
            .provider_resolver
            .resolve(&step.model_id)
            .await
            .map_err(|e| WorkflowError::step_execution("chat_completion", e.to_string()))?;

        // Execute using the resolved provider
        let response = provider
            .chat(&step.model_id, request)
            .await
            .map_err(|e| WorkflowError::step_execution("chat_completion", e.to_string()))?;

        // Extract content from response
        let content = response.message.content_text().unwrap_or_default().to_string();

        // Try to parse as JSON, otherwise return as string
        let output = serde_json::from_str::<Value>(&content).unwrap_or_else(|_| json!({
            "content": content,
            "model": response.model,
            "finish_reason": format!("{:?}", response.finish_reason),
        }));

        Ok(output)
    }

    /// Execute a knowledge base search step
    async fn execute_kb_search(
        &self,
        step: &crate::domain::KnowledgeBaseSearchStep,
        context: &WorkflowContext,
    ) -> Result<Value, WorkflowError> {
        // Resolve query
        let query = context.resolve_string(&step.query)?;

        debug!(
            "KB search for KB '{}' with query: {}",
            step.knowledge_base_id, query
        );

        // Get the provider for this knowledge base
        let provider = self
            .kb_provider_registry
            .get_required(&step.knowledge_base_id)
            .await
            .map_err(|e| WorkflowError::step_execution("kb_search", e.to_string()))?;

        // Build search parameters
        let mut search_params = SearchParams::new(&query).with_top_k(step.top_k);

        if let Some(threshold) = step.similarity_threshold {
            search_params = search_params.with_similarity_threshold(threshold);
        }

        // Execute the search
        let results = provider
            .search(search_params)
            .await
            .map_err(|e| WorkflowError::step_execution("kb_search", e.to_string()))?;

        debug!("KB search returned {} results", results.len());

        // Convert results to JSON
        let documents: Vec<Value> = results
            .iter()
            .map(|r| {
                json!({
                    "id": r.id,
                    "content": r.content,
                    "score": r.score,
                    "source": r.source,
                    "metadata": r.metadata,
                })
            })
            .collect();

        Ok(json!({
            "documents": documents,
            "total": documents.len(),
            "knowledge_base_id": step.knowledge_base_id,
            "query": query,
        }))
    }

    /// Execute a CRAG scoring step
    async fn execute_crag_scoring(
        &self,
        step: &crate::domain::CragScoringStep,
        context: &WorkflowContext,
    ) -> Result<Value, WorkflowError> {
        // Resolve input documents reference
        let documents = context.resolve_expression(&step.input_documents)?;
        let _query = context.resolve_string(&step.query)?;

        // TODO: Integrate with actual CRAG pipeline
        // For now, pass through documents as "correct" if they exist

        debug!("CRAG scoring requested - returning placeholder");

        let doc_array = documents.as_array().cloned().unwrap_or_default();
        let correct_count = doc_array.len();

        Ok(json!({
            "correct_documents": doc_array,
            "ambiguous_documents": [],
            "incorrect_documents": [],
            "correct_count": correct_count,
            "threshold": step.threshold,
            "strategy": format!("{:?}", step.strategy),
            "message": "CRAG integration pending - all documents marked as correct"
        }))
    }

    /// Execute a conditional step
    async fn execute_conditional(
        &self,
        step: &crate::domain::ConditionalStep,
        context: &WorkflowContext,
    ) -> Result<Value, WorkflowError> {
        // Evaluate conditions in order
        for condition in &step.conditions {
            let field_value = context.resolve_expression(&condition.field)?;
            let matched = condition.operator.evaluate(&field_value, &condition.value);

            if matched {
                return Ok(json!({
                    "matched": true,
                    "condition_field": condition.field,
                    "action": format!("{:?}", condition.action)
                }));
            }
        }

        // No condition matched, use default action
        Ok(json!({
            "matched": false,
            "action": format!("{:?}", step.default_action)
        }))
    }

    /// Execute an HTTP request step
    async fn execute_http_request(
        &self,
        step: &HttpRequestStep,
        context: &WorkflowContext,
    ) -> Result<Value, WorkflowError> {
        // Get the external API configuration
        let external_api = self
            .external_api_service
            .get(&step.external_api_id)
            .await
            .map_err(|e| WorkflowError::step_execution("http_request", e.to_string()))?
            .ok_or_else(|| {
                WorkflowError::step_execution(
                    "http_request",
                    format!("External API '{}' not found", step.external_api_id),
                )
            })?;

        // Check if external API is enabled
        if !external_api.is_enabled() {
            return Err(WorkflowError::step_execution(
                "http_request",
                format!("External API '{}' is disabled", step.external_api_id),
            ));
        }

        // Resolve path with variable substitution
        let resolved_path = context.resolve_string(&step.path)?;

        // Construct full URL from external API base_url + path
        let url = format!(
            "{}{}",
            external_api.base_url().trim_end_matches('/'),
            if resolved_path.starts_with('/') {
                resolved_path.clone()
            } else {
                format!("/{}", resolved_path)
            }
        );

        debug!("Executing HTTP request to: {}", url);

        // Build the HTTP client
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(step.timeout_ms))
            .build()
            .map_err(|e| WorkflowError::step_execution("http_request", e.to_string()))?;

        // Build the request
        let method = match step.method {
            HttpMethod::GET => reqwest::Method::GET,
            HttpMethod::POST => reqwest::Method::POST,
            HttpMethod::PUT => reqwest::Method::PUT,
            HttpMethod::DELETE => reqwest::Method::DELETE,
            HttpMethod::PATCH => reqwest::Method::PATCH,
            HttpMethod::HEAD => reqwest::Method::HEAD,
            HttpMethod::OPTIONS => reqwest::Method::OPTIONS,
        };

        let mut request = client.request(method, &url);

        // Add base headers from external API
        for (key, value) in external_api.base_headers() {
            request = request.header(key, value);
        }

        // Add auth header from credential if provided
        if let Some(credential_id) = &step.credential_id {
            let credential = self
                .credential_service
                .get(credential_id)
                .await
                .map_err(|e| WorkflowError::step_execution("http_request", e.to_string()))?
                .ok_or_else(|| {
                    WorkflowError::step_execution(
                        "http_request",
                        format!("Credential '{}' not found", credential_id),
                    )
                })?;

            // Validate credential type
            if *credential.credential_type()
                != crate::domain::credentials::CredentialType::HttpApiKey
            {
                return Err(WorkflowError::step_execution(
                    "http_request",
                    format!(
                        "Credential '{}' is not an HTTP API Key credential",
                        credential_id
                    ),
                ));
            }

            // Add auth header from credential
            if let Some(header_name) = credential.deployment() {
                let header_value_template = credential.header_value().unwrap_or("${api-key}");

                // Interpolate ${api-key} in the header value
                let header_value =
                    header_value_template.replace("${api-key}", credential.api_key());
                request = request.header(header_name, header_value);
            }
        }

        // Add step-specific headers with variable substitution (override base headers)
        for (key, value) in &step.headers {
            let resolved_value = context.resolve_string(value)?;
            request = request.header(key, resolved_value);
        }

        // Add body if present
        if let Some(body) = &step.body {
            // Resolve variables in body if it's a string
            let resolved_body: Value = if let Some(body_str) = body.as_str() {
                let resolved = context.resolve_string(body_str)?;
                // Try to parse as JSON, otherwise use as raw string
                serde_json::from_str(&resolved).unwrap_or(Value::String(resolved))
            } else {
                // For non-string bodies, resolve any string values within
                resolve_json_variables(body, context)?
            };

            request = request.json(&resolved_body);
        }

        // Execute the request
        let response = request
            .send()
            .await
            .map_err(|e| WorkflowError::step_execution("http_request", e.to_string()))?;

        let status = response.status();
        let status_code = status.as_u16();
        let is_success = status.is_success();

        // Check for error status if fail_on_error is true
        if step.fail_on_error && !is_success {
            let error_body = response.text().await.unwrap_or_default();
            return Err(WorkflowError::step_execution(
                "http_request",
                format!(
                    "HTTP request failed with status {}: {}",
                    status_code, error_body
                ),
            ));
        }

        // Parse response based on content type
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        let body: Value = if content_type.contains("application/json") {
            response
                .json()
                .await
                .unwrap_or_else(|_| Value::Null)
        } else {
            let text = response.text().await.unwrap_or_default();
            Value::String(text)
        };

        // Extract specific path if configured
        let extracted = if let Some(extract_path) = &step.extract_path {
            extract_json_path(&body, extract_path).unwrap_or(body.clone())
        } else {
            body.clone()
        };

        Ok(json!({
            "status_code": status_code,
            "success": is_success,
            "body": body,
            "extracted": extracted
        }))
    }

    /// Get the action from a conditional result
    fn get_conditional_action(
        &self,
        step: &crate::domain::ConditionalStep,
        context: &WorkflowContext,
    ) -> Result<ConditionalAction, WorkflowError> {
        // Evaluate conditions in order
        for condition in &step.conditions {
            let field_value = context.resolve_expression(&condition.field)?;
            let matched = condition.operator.evaluate(&field_value, &condition.value);

            if matched {
                return Ok(condition.action.clone());
            }
        }

        Ok(step.default_action.clone())
    }
}

#[async_trait]
impl WorkflowExecutor for WorkflowExecutorImpl {
    async fn execute(
        &self,
        workflow: &Workflow,
        input: Value,
    ) -> Result<WorkflowResult, WorkflowError> {
        let start = Instant::now();
        let mut step_results = Vec::new();
        let mut context = WorkflowContext::new(input);

        // Validate workflow
        if !workflow.is_enabled() {
            return Err(WorkflowError::disabled(workflow.id().as_str()));
        }

        if workflow.is_empty() {
            return Err(WorkflowError::empty_workflow(workflow.id().as_str()));
        }

        debug!("Executing workflow '{}'", workflow.id());

        let steps = workflow.steps();
        let mut step_index = 0;
        let mut steps_executed = 0;

        while step_index < steps.len() && steps_executed < self.config.max_steps {
            let step = &steps[step_index];
            let step_start = Instant::now();
            steps_executed += 1;

            debug!("Executing step '{}' (index {})", step.name(), step_index);

            // Handle conditional step specially
            if let WorkflowStepType::Conditional(cond_step) = step.step_type() {
                let action = self.get_conditional_action(cond_step, &context)?;

                let step_result = StepExecutionResult::success(
                    step.name(),
                    json!({"action": format!("{:?}", action)}),
                    step_start.elapsed().as_millis() as u64,
                );
                step_results.push(step_result);

                match action {
                    ConditionalAction::Continue => {
                        step_index += 1;
                    }
                    ConditionalAction::GoToStep(target) => {
                        if let Some(idx) = workflow.get_step_index(&target) {
                            step_index = idx;
                        } else {
                            return Err(WorkflowError::step_not_found(target));
                        }
                    }
                    ConditionalAction::EndWorkflow(output) => {
                        let final_output = output.unwrap_or(Value::Null);
                        return Ok(WorkflowResult::success(
                            final_output,
                            step_results,
                            start.elapsed().as_millis() as u64,
                        ));
                    }
                }
                continue;
            }

            // Execute non-conditional step
            match self.execute_step(step, &mut context).await {
                Ok(output) => {
                    context.set_step_output(step.name(), output.clone());

                    let step_result = StepExecutionResult::success(
                        step.name(),
                        output,
                        step_start.elapsed().as_millis() as u64,
                    );
                    step_results.push(step_result);
                    step_index += 1;
                }
                Err(e) => {
                    let step_result = StepExecutionResult::failure(
                        step.name(),
                        e.to_string(),
                        step_start.elapsed().as_millis() as u64,
                    );
                    step_results.push(step_result);

                    match step.on_error() {
                        OnErrorAction::FailWorkflow => {
                            return Ok(WorkflowResult::failure(
                                format!("Step '{}' failed: {}", step.name(), e),
                                step_results,
                                start.elapsed().as_millis() as u64,
                            ));
                        }
                        OnErrorAction::SkipStep => {
                            debug!("Skipping failed step '{}'", step.name());
                            step_index += 1;
                        }
                    }
                }
            }
        }

        // Get final output from last successful step
        let final_output = step_results
            .iter()
            .rev()
            .find(|r| r.success && !r.skipped)
            .and_then(|r| r.output.clone())
            .unwrap_or(Value::Null);

        Ok(WorkflowResult::success(
            final_output,
            step_results,
            start.elapsed().as_millis() as u64,
        ))
    }
}

/// Recursively resolve variable references in a JSON value
fn resolve_json_variables(
    value: &Value,
    context: &WorkflowContext,
) -> Result<Value, WorkflowError> {
    match value {
        Value::String(s) => {
            let resolved = context.resolve_string(s)?;
            // Try to parse as JSON if it looks like JSON
            if resolved.starts_with('{') || resolved.starts_with('[') {
                Ok(serde_json::from_str(&resolved).unwrap_or(Value::String(resolved)))
            } else {
                Ok(Value::String(resolved))
            }
        }
        Value::Array(arr) => {
            let resolved: Result<Vec<Value>, _> = arr
                .iter()
                .map(|v| resolve_json_variables(v, context))
                .collect();
            Ok(Value::Array(resolved?))
        }
        Value::Object(obj) => {
            let resolved: Result<serde_json::Map<String, Value>, _> = obj
                .iter()
                .map(|(k, v)| resolve_json_variables(v, context).map(|rv| (k.clone(), rv)))
                .collect();
            Ok(Value::Object(resolved?))
        }
        // Pass through other types unchanged
        other => Ok(other.clone()),
    }
}

/// Extract a value from JSON using a simple path expression
/// Supports: $.field, $.field.nested, $.array[0], $.array[*].field
fn extract_json_path(value: &Value, path: &str) -> Option<Value> {
    // Remove leading $. if present
    let path = path.strip_prefix("$.").unwrap_or(path);

    if path.is_empty() {
        return Some(value.clone());
    }

    let mut current = value.clone();

    for segment in path.split('.') {
        // Check for array index: field[0]
        if let Some(bracket_pos) = segment.find('[') {
            let field = &segment[..bracket_pos];
            let index_part = &segment[bracket_pos + 1..];
            let index_str = index_part.strip_suffix(']').unwrap_or(index_part);

            // Get the field first if not empty
            if !field.is_empty() {
                current = current.get(field)?.clone();
            }

            // Handle array access
            if index_str == "*" {
                // Wildcard - return array as-is for now
                return Some(current);
            } else if let Ok(idx) = index_str.parse::<usize>() {
                current = current.get(idx)?.clone();
            }
        } else {
            // Simple field access
            current = current.get(segment)?.clone();
        }
    }

    Some(current)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::credentials::StoredCredential;
    use crate::domain::llm::{LlmResponse, Message, MockLlmProvider, StaticProviderResolver};
    use crate::domain::storage::mock::MockStorage;
    use crate::domain::{
        ChatCompletionStep, Condition, ConditionalAction, ConditionalStep, ConditionOperator,
        PromptId,
    };
    use crate::infrastructure::credentials::CredentialServiceTrait;

    /// Mock credential service for testing
    #[derive(Debug)]
    struct MockCredentialService;

    #[async_trait]
    impl CredentialServiceTrait for MockCredentialService {
        async fn get(
            &self,
            _id: &str,
        ) -> Result<Option<StoredCredential>, crate::domain::DomainError> {
            Ok(None)
        }

        async fn list(&self) -> Result<Vec<StoredCredential>, crate::domain::DomainError> {
            Ok(vec![])
        }
    }

    fn create_mock_credential_service() -> Arc<dyn CredentialServiceTrait> {
        Arc::new(MockCredentialService)
    }

    /// Mock external API service for testing
    #[derive(Debug)]
    struct MockExternalApiService;

    #[async_trait]
    impl crate::infrastructure::external_api::ExternalApiServiceTrait for MockExternalApiService {
        async fn get(
            &self,
            _id: &str,
        ) -> Result<Option<crate::domain::ExternalApi>, crate::domain::DomainError> {
            Ok(None)
        }

        async fn list(&self) -> Result<Vec<crate::domain::ExternalApi>, crate::domain::DomainError> {
            Ok(vec![])
        }
    }

    fn create_mock_external_api_service(
    ) -> Arc<dyn crate::infrastructure::external_api::ExternalApiServiceTrait> {
        Arc::new(MockExternalApiService)
    }

    /// Mock KB provider registry for testing
    #[derive(Debug)]
    struct MockKbProviderRegistry;

    #[async_trait]
    impl KnowledgeBaseProviderRegistryTrait for MockKbProviderRegistry {
        async fn get(
            &self,
            _kb_id: &str,
        ) -> Option<Arc<dyn crate::domain::knowledge_base::KnowledgeBaseProvider>> {
            None
        }

        async fn get_required(
            &self,
            kb_id: &str,
        ) -> Result<
            Arc<dyn crate::domain::knowledge_base::KnowledgeBaseProvider>,
            crate::domain::DomainError,
        > {
            Err(crate::domain::DomainError::not_found(format!(
                "Knowledge base '{}' not found",
                kb_id
            )))
        }

        async fn has_provider(&self, _kb_id: &str) -> bool {
            false
        }

        async fn register(
            &self,
            _provider: Arc<dyn crate::domain::knowledge_base::KnowledgeBaseProvider>,
        ) {
        }
    }

    fn create_mock_kb_registry() -> Arc<dyn KnowledgeBaseProviderRegistryTrait> {
        Arc::new(MockKbProviderRegistry)
    }

    fn create_test_prompt(id: &str, content: &str) -> Prompt {
        Prompt::new(
            PromptId::new(id).unwrap(),
            format!("{} Prompt", id),
            content,
        )
    }

    fn create_prompt_storage() -> Arc<MockStorage<Prompt>> {
        Arc::new(
            MockStorage::<Prompt>::new()
                .with_entity(create_test_prompt("system-prompt", "You are a helpful assistant."))
                .with_entity(create_test_prompt("greeting-prompt", "Hello ${request:name}!"))
                .with_entity(create_test_prompt("chat-prompt", "Process the following.")),
        )
    }

    fn create_simple_workflow() -> Workflow {
        use crate::domain::WorkflowId;

        Workflow::new(WorkflowId::new("test").unwrap(), "Test Workflow").with_step(
            WorkflowStep::new(
                "chat",
                WorkflowStepType::ChatCompletion(ChatCompletionStep::new(
                    "gpt-4",
                    "greeting-prompt",
                    "What is ${request:name}?",
                )),
            ),
        )
    }

    fn create_mock_response(content: &str) -> LlmResponse {
        LlmResponse::new(
            "test-id".to_string(),
            "test-model".to_string(),
            Message::assistant(content),
        )
    }

    fn create_resolver(response_content: &str) -> Arc<StaticProviderResolver> {
        let response = create_mock_response(response_content);
        let provider = Arc::new(MockLlmProvider::new("mock").with_response(response));
        Arc::new(StaticProviderResolver::new(provider))
    }

    #[tokio::test]
    async fn test_execute_simple_workflow() {
        let resolver = create_resolver("Hello there!");
        let prompt_storage = create_prompt_storage();
        let executor = WorkflowExecutorImpl::new(resolver, prompt_storage, create_mock_credential_service(), create_mock_external_api_service(), create_mock_kb_registry());

        let workflow = create_simple_workflow();
        let input = json!({"name": "World"});

        let result = executor.execute(&workflow, input).await.unwrap();

        assert!(result.success);
        assert_eq!(result.step_results.len(), 1);
        assert!(result.step_results[0].success);
    }

    #[tokio::test]
    async fn test_execute_disabled_workflow() {
        let resolver = create_resolver("test");
        let prompt_storage = create_prompt_storage();
        let executor = WorkflowExecutorImpl::new(resolver, prompt_storage, create_mock_credential_service(), create_mock_external_api_service(), create_mock_kb_registry());

        let workflow = create_simple_workflow().with_enabled(false);
        let result = executor.execute(&workflow, json!({})).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("disabled"));
    }

    #[tokio::test]
    async fn test_execute_empty_workflow() {
        use crate::domain::WorkflowId;

        let resolver = create_resolver("test");
        let prompt_storage = create_prompt_storage();
        let executor = WorkflowExecutorImpl::new(resolver, prompt_storage, create_mock_credential_service(), create_mock_external_api_service(), create_mock_kb_registry());

        let workflow = Workflow::new(WorkflowId::new("empty").unwrap(), "Empty");
        let result = executor.execute(&workflow, json!({})).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("no steps"));
    }

    #[tokio::test]
    async fn test_conditional_continue() {
        use crate::domain::WorkflowId;

        let resolver = create_resolver("response");
        let prompt_storage = create_prompt_storage();
        let executor = WorkflowExecutorImpl::new(resolver, prompt_storage, create_mock_credential_service(), create_mock_external_api_service(), create_mock_kb_registry());

        let workflow = Workflow::new(WorkflowId::new("cond-test").unwrap(), "Conditional Test")
            .with_step(WorkflowStep::new(
                "check",
                WorkflowStepType::Conditional(
                    ConditionalStep::new(vec![Condition::new(
                        "${request:value}",
                        ConditionOperator::IsEmpty,
                        ConditionalAction::end_workflow_with(json!({"empty": true})),
                    )])
                    .with_default_action(ConditionalAction::Continue),
                ),
            ))
            .with_step(WorkflowStep::new(
                "chat",
                WorkflowStepType::ChatCompletion(ChatCompletionStep::new(
                    "gpt-4",
                    "system-prompt",
                    "Hello",
                )),
            ));

        // Non-empty value should continue to chat step
        let result = executor
            .execute(&workflow, json!({"value": "not empty"}))
            .await
            .unwrap();

        assert!(result.success);
        assert_eq!(result.step_results.len(), 2);
    }

    #[tokio::test]
    async fn test_conditional_end_workflow() {
        use crate::domain::WorkflowId;

        let resolver = create_resolver("response");
        let prompt_storage = create_prompt_storage();
        let executor = WorkflowExecutorImpl::new(resolver, prompt_storage, create_mock_credential_service(), create_mock_external_api_service(), create_mock_kb_registry());

        let workflow = Workflow::new(WorkflowId::new("cond-end").unwrap(), "Conditional End")
            .with_step(WorkflowStep::new(
                "check",
                WorkflowStepType::Conditional(
                    // Use empty default so missing field resolves to empty string
                    ConditionalStep::new(vec![Condition::new(
                        "${request:value:}",
                        ConditionOperator::IsEmpty,
                        ConditionalAction::end_workflow_with(json!({"ended_early": true})),
                    )])
                    .with_default_action(ConditionalAction::Continue),
                ),
            ))
            .with_step(WorkflowStep::new(
                "chat",
                WorkflowStepType::ChatCompletion(ChatCompletionStep::new(
                    "gpt-4",
                    "system-prompt",
                    "Hello",
                )),
            ));

        // Empty value should end workflow early
        let result = executor.execute(&workflow, json!({})).await.unwrap();

        assert!(result.success);
        assert_eq!(result.output, json!({"ended_early": true}));
        assert_eq!(result.step_results.len(), 1); // Only conditional step executed
    }

    #[tokio::test]
    async fn test_variable_resolution() {
        let resolver = create_resolver("The answer is 42");
        let prompt_storage = create_prompt_storage();
        let executor = WorkflowExecutorImpl::new(resolver, prompt_storage, create_mock_credential_service(), create_mock_external_api_service(), create_mock_kb_registry());

        let workflow = create_simple_workflow();
        let input = json!({"name": "Test User"});

        let result = executor.execute(&workflow, input).await.unwrap();
        assert!(result.success);
    }

    #[tokio::test]
    async fn test_step_output_chaining() {
        use crate::domain::WorkflowId;

        let resolver = create_resolver(r#"{"summary": "test summary"}"#);
        let prompt_storage = create_prompt_storage();
        let executor = WorkflowExecutorImpl::new(resolver, prompt_storage, create_mock_credential_service(), create_mock_external_api_service(), create_mock_kb_registry());

        let workflow = Workflow::new(WorkflowId::new("chain").unwrap(), "Chain Test")
            .with_step(WorkflowStep::new(
                "step1",
                WorkflowStepType::ChatCompletion(ChatCompletionStep::new(
                    "gpt-4",
                    "system-prompt",
                    "First",
                )),
            ))
            .with_step(WorkflowStep::new(
                "step2",
                WorkflowStepType::ChatCompletion(ChatCompletionStep::new(
                    "gpt-4",
                    "chat-prompt",
                    "Use ${step:step1:summary}",
                )),
            ));

        let result = executor.execute(&workflow, json!({})).await.unwrap();

        assert!(result.success);
        assert_eq!(result.step_results.len(), 2);
    }

    #[tokio::test]
    async fn test_prompt_not_found() {
        use crate::domain::WorkflowId;

        let resolver = create_resolver("test");
        let prompt_storage = Arc::new(MockStorage::<Prompt>::new()); // Empty storage
        let executor = WorkflowExecutorImpl::new(resolver, prompt_storage, create_mock_credential_service(), create_mock_external_api_service(), create_mock_kb_registry());

        let workflow = Workflow::new(WorkflowId::new("test").unwrap(), "Test")
            .with_step(WorkflowStep::new(
                "chat",
                WorkflowStepType::ChatCompletion(ChatCompletionStep::new(
                    "gpt-4",
                    "nonexistent-prompt",
                    "Hello",
                )),
            ));

        let result = executor.execute(&workflow, json!({})).await;

        // Should succeed but the step should fail
        let workflow_result = result.unwrap();
        assert!(!workflow_result.success);
        assert!(workflow_result.error.unwrap().contains("not found"));
    }
}
