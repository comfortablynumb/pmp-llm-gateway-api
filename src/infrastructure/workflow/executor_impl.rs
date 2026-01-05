//! Workflow executor implementation

use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use serde_json::{json, Value};
use tracing::debug;

use crate::domain::{
    ConditionalAction, LlmProvider, LlmRequest, OnErrorAction, StepExecutionResult, Workflow,
    WorkflowContext, WorkflowError, WorkflowExecutor, WorkflowResult, WorkflowStep,
    WorkflowStepType,
};

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
#[derive(Debug)]
pub struct WorkflowExecutorImpl {
    /// LLM provider for chat completions
    llm_provider: Arc<dyn LlmProvider>,

    /// Executor configuration
    config: WorkflowExecutorConfig,
}

impl WorkflowExecutorImpl {
    /// Create a new executor
    pub fn new(llm_provider: Arc<dyn LlmProvider>) -> Self {
        Self {
            llm_provider,
            config: WorkflowExecutorConfig::default(),
        }
    }

    /// Create with custom configuration
    pub fn with_config(llm_provider: Arc<dyn LlmProvider>, config: WorkflowExecutorConfig) -> Self {
        Self {
            llm_provider,
            config,
        }
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

        let system_message = if let Some(ref sys) = step.system_message {
            Some(context.resolve_string(sys)?)
        } else {
            None
        };

        // Build request
        let mut request_builder = LlmRequest::builder();

        if let Some(sys) = system_message {
            request_builder = request_builder.system(&sys);
        }

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

        // Execute
        let response = self
            .llm_provider
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
        let _query = context.resolve_string(&step.query)?;

        // TODO: Integrate with actual knowledge base provider
        // For now, return a placeholder indicating KB search is not yet implemented

        debug!(
            "KB search requested for KB '{}' - returning placeholder",
            step.knowledge_base_id
        );

        Ok(json!({
            "documents": [],
            "total": 0,
            "knowledge_base_id": step.knowledge_base_id,
            "message": "Knowledge base integration pending - no documents returned"
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        ChatCompletionStep, Condition, ConditionalStep, ConditionOperator, FinishReason,
        LlmResponse, LlmStream, Message,
    };
    use std::sync::atomic::{AtomicUsize, Ordering};

    // Mock LLM provider for testing
    #[derive(Debug)]
    struct MockLlmProvider {
        response: String,
        call_count: AtomicUsize,
    }

    impl MockLlmProvider {
        fn new(response: impl Into<String>) -> Self {
            Self {
                response: response.into(),
                call_count: AtomicUsize::new(0),
            }
        }

        fn call_count(&self) -> usize {
            self.call_count.load(Ordering::Relaxed)
        }
    }

    #[async_trait]
    impl LlmProvider for MockLlmProvider {
        async fn chat(
            &self,
            _model: &str,
            _request: LlmRequest,
        ) -> Result<LlmResponse, crate::domain::DomainError> {
            self.call_count.fetch_add(1, Ordering::Relaxed);

            Ok(LlmResponse::new(
                "test-id".to_string(),
                "test-model".to_string(),
                Message::assistant(&self.response),
            )
            .with_finish_reason(FinishReason::Stop))
        }

        async fn chat_stream(
            &self,
            _model: &str,
            _request: LlmRequest,
        ) -> Result<LlmStream, crate::domain::DomainError> {
            Err(crate::domain::DomainError::provider(
                "mock",
                "Streaming not supported",
            ))
        }

        fn provider_name(&self) -> &'static str {
            "mock"
        }

        fn available_models(&self) -> Vec<&'static str> {
            vec!["test-model"]
        }
    }

    fn create_simple_workflow() -> Workflow {
        use crate::domain::WorkflowId;

        Workflow::new(WorkflowId::new("test").unwrap(), "Test Workflow").with_step(
            WorkflowStep::new(
                "chat",
                WorkflowStepType::ChatCompletion(ChatCompletionStep::new(
                    "gpt-4",
                    "Hello ${request:name}",
                )),
            ),
        )
    }

    #[tokio::test]
    async fn test_execute_simple_workflow() {
        let provider = Arc::new(MockLlmProvider::new("Hello there!"));
        let executor = WorkflowExecutorImpl::new(provider.clone());

        let workflow = create_simple_workflow();
        let input = json!({"name": "World"});

        let result = executor.execute(&workflow, input).await.unwrap();

        assert!(result.success);
        assert_eq!(result.step_results.len(), 1);
        assert!(result.step_results[0].success);
        assert_eq!(provider.call_count(), 1);
    }

    #[tokio::test]
    async fn test_execute_disabled_workflow() {
        let provider = Arc::new(MockLlmProvider::new("test"));
        let executor = WorkflowExecutorImpl::new(provider);

        let workflow = create_simple_workflow().with_enabled(false);
        let result = executor.execute(&workflow, json!({})).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("disabled"));
    }

    #[tokio::test]
    async fn test_execute_empty_workflow() {
        use crate::domain::WorkflowId;

        let provider = Arc::new(MockLlmProvider::new("test"));
        let executor = WorkflowExecutorImpl::new(provider);

        let workflow = Workflow::new(WorkflowId::new("empty").unwrap(), "Empty");
        let result = executor.execute(&workflow, json!({})).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("no steps"));
    }

    #[tokio::test]
    async fn test_conditional_continue() {
        use crate::domain::WorkflowId;

        let provider = Arc::new(MockLlmProvider::new("response"));
        let executor = WorkflowExecutorImpl::new(provider);

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
                WorkflowStepType::ChatCompletion(ChatCompletionStep::new("gpt-4", "Hello")),
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

        let provider = Arc::new(MockLlmProvider::new("response"));
        let executor = WorkflowExecutorImpl::new(provider.clone());

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
                WorkflowStepType::ChatCompletion(ChatCompletionStep::new("gpt-4", "Hello")),
            ));

        // Empty value should end workflow early
        let result = executor.execute(&workflow, json!({})).await.unwrap();

        assert!(result.success);
        assert_eq!(result.output, json!({"ended_early": true}));
        assert_eq!(result.step_results.len(), 1); // Only conditional step executed
        assert_eq!(provider.call_count(), 0); // Chat step not called
    }

    #[tokio::test]
    async fn test_variable_resolution() {
        let provider = Arc::new(MockLlmProvider::new("The answer is 42"));
        let executor = WorkflowExecutorImpl::new(provider);

        let workflow = create_simple_workflow();
        let input = json!({"name": "Test User"});

        let result = executor.execute(&workflow, input).await.unwrap();
        assert!(result.success);
    }

    #[tokio::test]
    async fn test_step_output_chaining() {
        use crate::domain::WorkflowId;

        let provider = Arc::new(MockLlmProvider::new(r#"{"summary": "test summary"}"#));
        let executor = WorkflowExecutorImpl::new(provider);

        let workflow = Workflow::new(WorkflowId::new("chain").unwrap(), "Chain Test")
            .with_step(WorkflowStep::new(
                "step1",
                WorkflowStepType::ChatCompletion(ChatCompletionStep::new("gpt-4", "First")),
            ))
            .with_step(WorkflowStep::new(
                "step2",
                WorkflowStepType::ChatCompletion(ChatCompletionStep::new(
                    "gpt-4",
                    "Use ${step:step1:summary}",
                )),
            ));

        let result = executor.execute(&workflow, json!({})).await.unwrap();

        assert!(result.success);
        assert_eq!(result.step_results.len(), 2);
    }
}
