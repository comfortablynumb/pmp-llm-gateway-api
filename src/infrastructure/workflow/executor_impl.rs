//! Workflow executor implementation

use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use serde_json::{json, Value};
use tracing::debug;

use crate::domain::knowledge_base::{MetadataFilter, SearchParams, SearchResult};
use crate::domain::llm::ProviderResolver;
use crate::domain::storage::Storage;
use crate::domain::{
    ConditionalAction, HttpMethod, HttpRequestStep, LlmRequest, OnErrorAction, Prompt,
    StepExecutionResult, Workflow, WorkflowContext, WorkflowError, WorkflowExecutor,
    WorkflowResult, WorkflowStep, WorkflowStepType, WorkflowTokenUsage,
};
use crate::infrastructure::knowledge_base::KnowledgeBaseProviderRegistryTrait;

/// Build XML representation of search results
///
/// Format: `<documents><document><id>...</id><content>...</content></document>...</documents>`
fn build_documents_xml(results: &[SearchResult]) -> String {
    let mut xml = String::from("<documents>");

    for result in results {
        xml.push_str("<document><id>");
        xml.push_str(&escape_xml(&result.id));
        xml.push_str("</id><content>");
        xml.push_str(&escape_xml(&result.content));
        xml.push_str("</content></document>");
    }

    xml.push_str("</documents>");
    xml
}

/// Escape XML special characters
fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

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

    /// Render a prompt template with variables
    ///
    /// This performs two-phase variable resolution:
    /// 1. Resolve ${request:*} and ${step:*:*} references in the variable values
    /// 2. Substitute ${var:*} placeholders in the template with resolved values
    fn render_prompt_with_variables(
        &self,
        template: &str,
        prompt_variables: &std::collections::HashMap<String, String>,
        context: &WorkflowContext,
    ) -> Result<String, WorkflowError> {
        use crate::domain::PromptTemplate;

        // If no variables provided, just resolve context variables in the template
        if prompt_variables.is_empty() {
            return context.resolve_string(template);
        }

        // Phase 1: Resolve context variables in each prompt_variable value
        let mut resolved_variables = std::collections::HashMap::new();

        for (key, value) in prompt_variables {
            let resolved_value = context.resolve_string(value)?;
            resolved_variables.insert(key.clone(), resolved_value);
        }

        // Phase 2: Parse and render the prompt template with resolved variables
        let parsed = PromptTemplate::parse(template)
            .map_err(|e| WorkflowError::step_execution("prompt_rendering", e.to_string()))?;

        let rendered = parsed.render(&resolved_variables)
            .map_err(|e| WorkflowError::step_execution("prompt_rendering", e.to_string()))?;

        // Phase 3: Resolve any remaining context variables in the rendered template
        context.resolve_string(&rendered)
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
        // Resolve prompt_id to message content
        let prompt_template = self.resolve_prompt(&step.prompt_id).await?;

        // Render the prompt template with prompt_variables
        // First, resolve any ${request:*} or ${step:*:*} in the variable values
        // Then, substitute ${var:*} placeholders with the resolved values
        let rendered_prompt = self.render_prompt_with_variables(
            &prompt_template,
            &step.prompt_variables,
            context,
        )?;

        // Build request - use rendered prompt as user message
        let mut request_builder = LlmRequest::builder();
        request_builder = request_builder.user(&rendered_prompt);

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
            .map_err(|e| {
                tracing::error!(
                    model_id = %step.model_id,
                    error = %e,
                    "Failed to resolve LLM provider for chat completion"
                );
                WorkflowError::step_execution("chat_completion", e.to_string())
            })?;

        // Execute using the resolved provider
        let response = provider
            .chat(&step.model_id, request)
            .await
            .map_err(|e| {
                tracing::error!(
                    model_id = %step.model_id,
                    prompt_id = %step.prompt_id,
                    error = %e,
                    "Chat completion failed"
                );
                WorkflowError::step_execution("chat_completion", e.to_string())
            })?;

        // Extract content from response
        let content = response.message.content_text().unwrap_or_default().to_string();

        // Build prompt object with exact message used
        let prompt = json!({
            "content": rendered_prompt,
        });

        // Serialize full response object
        let response_json = serde_json::to_value(&response).unwrap_or_else(|_| json!({}));

        // Try to parse content as JSON for structured output
        let parsed_content = serde_json::from_str::<Value>(&content).ok();

        // Build output with prompt and full response
        let mut output = json!({
            "content": content,
            "prompt": prompt,
            "response": response_json,
        });

        // If LLM returned valid JSON, merge fields at top level for backward compatibility
        // and also store under parsed_content for explicit access
        if let Some(parsed) = parsed_content {
            output["parsed_content"] = parsed.clone();

            // Merge parsed JSON object fields at top level for ${step:name:field} access
            if let Value::Object(map) = parsed {
                if let Value::Object(ref mut out_map) = output {
                    for (key, value) in map {
                        // Don't overwrite reserved fields
                        if !["content", "prompt", "response", "parsed_content"].contains(&key.as_str()) {
                            out_map.insert(key, value);
                        }
                    }
                }
            }
        }

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
            kb_id = %step.knowledge_base_id,
            query = %query,
            top_k = step.top_k,
            has_filter = step.filter.is_some(),
            "Executing KB search step"
        );

        // Get the provider for this knowledge base
        let provider = self
            .kb_provider_registry
            .get_required(&step.knowledge_base_id)
            .await
            .map_err(|e| {
                tracing::error!(
                    kb_id = %step.knowledge_base_id,
                    error = %e,
                    "Failed to get KB provider"
                );
                WorkflowError::step_execution("kb_search", e.to_string())
            })?;

        // Build search parameters
        let mut search_params = SearchParams::new(&query).with_top_k(step.top_k);

        if let Some(threshold) = step.similarity_threshold {
            search_params = search_params.with_similarity_threshold(threshold);
        }

        // Apply metadata filter if provided
        if let Some(filter_json) = &step.filter {
            match serde_json::from_value::<MetadataFilter>(filter_json.clone()) {
                Ok(filter) => {
                    debug!(filter = ?filter_json, "Applying metadata filter to KB search");
                    search_params = search_params.with_filter(filter);
                }
                Err(e) => {
                    tracing::warn!(
                        kb_id = %step.knowledge_base_id,
                        error = %e,
                        filter = ?filter_json,
                        "Failed to parse metadata filter, continuing without filter"
                    );
                }
            }
        }

        // Execute the search
        let results = provider
            .search(search_params)
            .await
            .map_err(|e| {
                tracing::error!(
                    kb_id = %step.knowledge_base_id,
                    error = %e,
                    query = %query,
                    "KB search failed"
                );
                WorkflowError::step_execution("kb_search", e.to_string())
            })?;

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

        // Build XML representation for easy template injection
        let documents_xml = build_documents_xml(&results);

        Ok(json!({
            "documents": documents,
            "documents_xml": documents_xml,
            "total": documents.len(),
            "knowledge_base_id": step.knowledge_base_id,
            "query": query,
        }))
    }

    /// Execute a CRAG scoring step
    ///
    /// This step:
    /// 1. Renders the configured prompt with all prompt_variables (resolved from context)
    /// 2. Makes ONE LLM call with structured output expecting a "scores" map (doc_id -> score)
    /// 3. Filters the documents array based on scores >= threshold
    /// 4. Returns the filtered documents and the rendered prompt
    async fn execute_crag_scoring(
        &self,
        step: &crate::domain::CragScoringStep,
        context: &WorkflowContext,
    ) -> Result<Value, WorkflowError> {
        debug!(
            model_id = %step.model_id,
            prompt_id = %step.prompt_id,
            threshold = step.threshold,
            "Executing CRAG scoring step"
        );

        // Get the prompt template
        let prompt = self.resolve_prompt(&step.prompt_id).await?;

        // Get documents array from documents_source
        // Can be a variable reference like "${step:search:documents}" or a simple field name
        let documents = if step.documents_source.starts_with("${") {
            // It's a variable reference, resolve it
            context.resolve_expression(&step.documents_source)?
        } else {
            // It's a simple field name, treat as empty (user should use variable syntax)
            Value::Array(vec![])
        };

        let doc_array = documents.as_array().cloned().unwrap_or_default();

        // Render the prompt with all prompt_variables resolved
        let rendered_prompt = self.render_prompt_with_variables(
            &prompt,
            &step.prompt_variables.clone().unwrap_or_default(),
            context,
        )?;

        // If no documents, return empty result with the rendered prompt
        if doc_array.is_empty() {
            return Ok(json!({
                "documents": [],
                "relevant_documents": [],
                "scored_documents": [],
                "scores": {},
                "relevant_count": 0,
                "threshold": step.threshold,
                "rendered_prompt": rendered_prompt,
            }));
        }

        // Resolve the LLM provider and get the provider_model name
        let resolved = self
            .provider_resolver
            .resolve_with_model(&step.model_id)
            .await
            .map_err(|e| {
                tracing::error!(
                    model_id = %step.model_id,
                    error = %e,
                    "Failed to resolve LLM provider for CRAG scoring"
                );
                WorkflowError::step_execution("crag_scoring", e.to_string())
            })?;

        debug!(
            model_id = %step.model_id,
            provider_model = %resolved.provider_model,
            "Using provider_model for CRAG scoring"
        );

        // Build LLM request with JSON schema for structured output
        // Note: Requires a model that supports structured outputs (gpt-4o, gpt-4o-mini, etc.)
        // Using array format since OpenAI strict mode doesn't support additionalProperties
        let request = LlmRequest::builder()
            .user(&rendered_prompt)
            .json_schema(
                "crag_scores",
                json!({
                    "type": "object",
                    "description": "Document relevance scores for CRAG (Corrective RAG) scoring",
                    "properties": {
                        "scores": {
                            "type": "array",
                            "description": "Array of document relevance scores. Each document must be scored on a scale from 0.00 to 1.00 where: 0.00 = completely irrelevant, 0.25 = slightly relevant, 0.50 = moderately relevant, 0.75 = highly relevant, 1.00 = perfectly relevant. Use decimal values with two decimal places (e.g., 0.85, 0.42).",
                            "items": {
                                "type": "object",
                                "description": "A single document's relevance score",
                                "properties": {
                                    "id": {
                                        "type": "string",
                                        "description": "The unique document ID (must match the ID from the input documents)"
                                    },
                                    "score": {
                                        "type": "number",
                                        "description": "Relevance score as a decimal between 0.00 and 1.00 (inclusive). Must NOT be an integer like 1, 5, or 10. Examples: 0.00, 0.25, 0.50, 0.75, 0.85, 1.00"
                                    }
                                },
                                "required": ["id", "score"],
                                "additionalProperties": false
                            }
                        }
                    },
                    "required": ["scores"],
                    "additionalProperties": false
                }),
                true,
            )
            .build();

        // Execute scoring with ONE LLM call using the provider_model
        let response = resolved
            .provider
            .chat(&resolved.provider_model, request)
            .await
            .map_err(|e| {
                tracing::error!(
                    model_id = %step.model_id,
                    provider_model = %resolved.provider_model,
                    error = %e,
                    "CRAG scoring LLM call failed"
                );
                WorkflowError::step_execution("crag_scoring", e.to_string())
            })?;

        // Parse scores array from response and convert to map
        let content = response.message.content_text().unwrap_or_default();
        let scores_array: Vec<Value> = serde_json::from_str::<Value>(content)
            .ok()
            .and_then(|v| v.get("scores").cloned())
            .and_then(|s| s.as_array().cloned())
            .unwrap_or_default();

        // Convert array of {id, score} to HashMap
        let scores_map: std::collections::HashMap<String, f64> = scores_array
            .iter()
            .filter_map(|item| {
                let id = item.get("id")?.as_str()?.to_string();
                let score = item.get("score")?.as_f64()?;
                Some((id, score))
            })
            .collect();

        debug!(
            scores_count = scores_map.len(),
            "Received scores from LLM"
        );

        // Filter documents based on scores >= threshold
        let mut relevant_documents = Vec::new();
        let mut scored_documents = Vec::new();

        for doc in &doc_array {
            let doc_id = doc
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            let score = scores_map.get(doc_id).copied().unwrap_or(0.0);

            // Build scored document with score attached
            let scored_doc = json!({
                "id": doc_id,
                "content": doc.get("content").cloned().unwrap_or(Value::Null),
                "score": score,
                "metadata": doc.get("metadata").cloned().unwrap_or(Value::Null),
                "source": doc.get("source").cloned().unwrap_or(Value::Null),
            });

            scored_documents.push(scored_doc.clone());

            // Filter: keep documents with score >= threshold
            if score >= step.threshold as f64 {
                relevant_documents.push(scored_doc);
            }
        }

        let relevant_count = relevant_documents.len();

        Ok(json!({
            "documents": relevant_documents.clone(),
            "relevant_documents": relevant_documents,
            "scored_documents": scored_documents,
            "scores": scores_map,
            "relevant_count": relevant_count,
            "threshold": step.threshold,
            "rendered_prompt": rendered_prompt,
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
            .map_err(|e| {
                tracing::error!(
                    external_api_id = %step.external_api_id,
                    url = %url,
                    method = ?step.method,
                    error = %e,
                    "HTTP request failed to send"
                );
                WorkflowError::step_execution("http_request", e.to_string())
            })?;

        let status = response.status();
        let status_code = status.as_u16();
        let is_success = status.is_success();

        // Check for error status if fail_on_error is true
        if step.fail_on_error && !is_success {
            let error_body = response.text().await.unwrap_or_default();
            tracing::error!(
                external_api_id = %step.external_api_id,
                url = %url,
                method = ?step.method,
                status_code = status_code,
                error_body = %error_body,
                "HTTP request returned error status"
            );
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

    /// Build input data for a step (for logging purposes)
    fn build_step_input(
        &self,
        step: &WorkflowStep,
        context: &WorkflowContext,
    ) -> Result<Value, WorkflowError> {
        match step.step_type() {
            WorkflowStepType::ChatCompletion(chat_step) => {
                Ok(json!({
                    "model_id": chat_step.model_id,
                    "prompt_id": chat_step.prompt_id,
                    "prompt_variables": chat_step.prompt_variables,
                    "temperature": chat_step.temperature,
                    "max_tokens": chat_step.max_tokens,
                }))
            }
            WorkflowStepType::KnowledgeBaseSearch(kb_step) => {
                let resolved_query = context.resolve_string(&kb_step.query)?;
                Ok(json!({
                    "knowledge_base_id": kb_step.knowledge_base_id,
                    "query": resolved_query,
                    "top_k": kb_step.top_k,
                    "similarity_threshold": kb_step.similarity_threshold,
                }))
            }
            WorkflowStepType::CragScoring(crag_step) => {
                // Get documents array from documents_source
                let doc_count = if crag_step.documents_source.starts_with("${") {
                    context
                        .resolve_expression(&crag_step.documents_source)
                        .ok()
                        .and_then(|v| v.as_array().map(|a| a.len()))
                        .unwrap_or(0)
                } else {
                    0
                };

                Ok(json!({
                    "model_id": crag_step.model_id,
                    "prompt_id": crag_step.prompt_id,
                    "documents_source": crag_step.documents_source,
                    "documents_count": doc_count,
                    "prompt_variables": crag_step.prompt_variables,
                    "threshold": crag_step.threshold,
                }))
            }
            WorkflowStepType::Conditional(cond_step) => {
                let conditions: Vec<Value> = cond_step
                    .conditions
                    .iter()
                    .map(|c| {
                        let resolved_field = context.resolve_expression(&c.field).ok();
                        json!({
                            "field": c.field,
                            "resolved_value": resolved_field,
                            "operator": format!("{:?}", c.operator),
                        })
                    })
                    .collect();
                Ok(json!({
                    "conditions": conditions,
                    "default_action": format!("{:?}", cond_step.default_action),
                }))
            }
            WorkflowStepType::HttpRequest(http_step) => {
                let resolved_path = context.resolve_string(&http_step.path)?;
                let resolved_body = if let Some(body) = &http_step.body {
                    Some(resolve_json_variables(body, context)?)
                } else {
                    None
                };
                Ok(json!({
                    "external_api_id": http_step.external_api_id,
                    "method": format!("{:?}", http_step.method),
                    "path": resolved_path,
                    "body": resolved_body,
                    "credential_id": http_step.credential_id,
                }))
            }
        }
    }
}

/// Get the step type string from a WorkflowStepType
fn get_step_type_name(step_type: &WorkflowStepType) -> &'static str {
    match step_type {
        WorkflowStepType::ChatCompletion(_) => "chat_completion",
        WorkflowStepType::KnowledgeBaseSearch(_) => "knowledge_base_search",
        WorkflowStepType::CragScoring(_) => "crag_scoring",
        WorkflowStepType::Conditional(_) => "conditional",
        WorkflowStepType::HttpRequest(_) => "http_request",
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
        let mut total_token_usage = WorkflowTokenUsage::default();

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

            let step_type_name = get_step_type_name(step.step_type());

            // Build step input (best effort - if it fails, we still execute the step)
            let step_input = self.build_step_input(step, &context).ok();

            // Handle conditional step specially
            if let WorkflowStepType::Conditional(cond_step) = step.step_type() {
                let action = self.get_conditional_action(cond_step, &context)?;

                let mut step_result = StepExecutionResult::success(
                    step.name(),
                    step_type_name,
                    json!({"action": format!("{:?}", action)}),
                    step_start.elapsed().as_millis() as u64,
                );

                if let Some(input) = step_input {
                    step_result = step_result.with_input(input);
                }
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
                        let mut result = WorkflowResult::success(
                            final_output,
                            step_results,
                            start.elapsed().as_millis() as u64,
                        );

                        if total_token_usage.total_tokens > 0 {
                            result = result.with_token_usage(total_token_usage);
                        }

                        return Ok(result);
                    }
                }
                continue;
            }

            // Execute non-conditional step
            match self.execute_step(step, &mut context).await {
                Ok(output) => {
                    context.set_step_output(step.name(), output.clone());

                    let mut step_result = StepExecutionResult::success(
                        step.name(),
                        step_type_name,
                        output.clone(),
                        step_start.elapsed().as_millis() as u64,
                    );

                    // Extract token usage from ChatCompletion step output
                    if matches!(step.step_type(), WorkflowStepType::ChatCompletion(_)) {
                        if let Some(usage) = output
                            .get("response")
                            .and_then(|r| r.get("usage"))
                        {
                            let input_tokens = usage
                                .get("prompt_tokens")
                                .and_then(|v| v.as_u64())
                                .unwrap_or(0) as u32;
                            let output_tokens = usage
                                .get("completion_tokens")
                                .and_then(|v| v.as_u64())
                                .unwrap_or(0) as u32;

                            let step_usage = WorkflowTokenUsage::new(input_tokens, output_tokens);
                            total_token_usage.add(&step_usage);
                            step_result = step_result.with_token_usage(step_usage);
                        }
                    }

                    if let Some(input) = step_input {
                        step_result = step_result.with_input(input);
                    }
                    step_results.push(step_result);
                    step_index += 1;
                }
                Err(e) => {
                    let mut step_result = StepExecutionResult::failure(
                        step.name(),
                        step_type_name,
                        e.to_string(),
                        step_start.elapsed().as_millis() as u64,
                    );

                    if let Some(input) = step_input {
                        step_result = step_result.with_input(input);
                    }
                    step_results.push(step_result);

                    match step.on_error() {
                        OnErrorAction::FailWorkflow => {
                            let mut result = WorkflowResult::failure(
                                format!("Step '{}' failed: {}", step.name(), e),
                                step_results,
                                start.elapsed().as_millis() as u64,
                            );

                            if total_token_usage.total_tokens > 0 {
                                result = result.with_token_usage(total_token_usage);
                            }

                            return Ok(result);
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

        // Build result with token usage if any tokens were consumed
        let mut result = WorkflowResult::success(
            final_output,
            step_results,
            start.elapsed().as_millis() as u64,
        );

        if total_token_usage.total_tokens > 0 {
            result = result.with_token_usage(total_token_usage);
        }

        Ok(result)
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
                WorkflowStepType::ChatCompletion(
                    ChatCompletionStep::new("gpt-4", "greeting-prompt")
                        .with_prompt_variable("name", "${request:name}"),
                ),
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
                WorkflowStepType::ChatCompletion(ChatCompletionStep::new("gpt-4", "system-prompt")),
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
                WorkflowStepType::ChatCompletion(ChatCompletionStep::new("gpt-4", "system-prompt")),
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
                WorkflowStepType::ChatCompletion(ChatCompletionStep::new("gpt-4", "system-prompt")),
            ))
            .with_step(WorkflowStep::new(
                "step2",
                WorkflowStepType::ChatCompletion(
                    ChatCompletionStep::new("gpt-4", "chat-prompt")
                        .with_prompt_variable("summary", "${step:step1:summary}"),
                ),
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

        let workflow = Workflow::new(WorkflowId::new("test").unwrap(), "Test").with_step(
            WorkflowStep::new(
                "chat",
                WorkflowStepType::ChatCompletion(ChatCompletionStep::new(
                    "gpt-4",
                    "nonexistent-prompt",
                )),
            ),
        );

        let result = executor.execute(&workflow, json!({})).await;

        // Should succeed but the step should fail
        let workflow_result = result.unwrap();
        assert!(!workflow_result.success);
        assert!(workflow_result.error.unwrap().contains("not found"));
    }
}
