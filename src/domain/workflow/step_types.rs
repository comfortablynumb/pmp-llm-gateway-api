//! Workflow step type definitions

use serde::{Deserialize, Serialize};

/// Type of workflow step
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WorkflowStepType {
    /// LLM chat completion step
    ChatCompletion(ChatCompletionStep),

    /// Knowledge base search step
    KnowledgeBaseSearch(KnowledgeBaseSearchStep),

    /// CRAG document scoring step
    CragScoring(CragScoringStep),

    /// Conditional branching step
    Conditional(ConditionalStep),

    /// HTTP request step
    HttpRequest(HttpRequestStep),
}

impl WorkflowStepType {
    /// Get a human-readable type name
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::ChatCompletion(_) => "chat_completion",
            Self::KnowledgeBaseSearch(_) => "knowledge_base_search",
            Self::CragScoring(_) => "crag_scoring",
            Self::Conditional(_) => "conditional",
            Self::HttpRequest(_) => "http_request",
        }
    }
}

/// Chat completion step configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChatCompletionStep {
    /// Model ID to use for completion
    pub model_id: String,

    /// Prompt ID to use as system message template
    pub prompt_id: String,

    /// Prompt template variables (key -> value, values can contain variable references)
    /// These are passed to the prompt template for rendering
    #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
    pub prompt_variables: std::collections::HashMap<String, String>,

    /// Optional temperature override
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,

    /// Optional max tokens override
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,

    /// Optional top_p override
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
}

impl ChatCompletionStep {
    pub fn new(model_id: impl Into<String>, prompt_id: impl Into<String>) -> Self {
        Self {
            model_id: model_id.into(),
            prompt_id: prompt_id.into(),
            prompt_variables: std::collections::HashMap::new(),
            temperature: None,
            max_tokens: None,
            top_p: None,
        }
    }

    pub fn with_prompt_variable(
        mut self,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        self.prompt_variables.insert(key.into(), value.into());
        self
    }

    pub fn with_prompt_variables(
        mut self,
        variables: std::collections::HashMap<String, String>,
    ) -> Self {
        self.prompt_variables = variables;
        self
    }

    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    pub fn with_top_p(mut self, top_p: f32) -> Self {
        self.top_p = Some(top_p);
        self
    }
}

/// Knowledge base search step configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KnowledgeBaseSearchStep {
    /// Knowledge base ID to search
    pub knowledge_base_id: String,

    /// Search query (can contain variable references)
    pub query: String,

    /// Number of results to return
    #[serde(default = "default_top_k")]
    pub top_k: u32,

    /// Optional similarity threshold
    #[serde(skip_serializing_if = "Option::is_none")]
    pub similarity_threshold: Option<f32>,

    /// Optional metadata filter
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<serde_json::Value>,
}

fn default_top_k() -> u32 {
    10
}

impl KnowledgeBaseSearchStep {
    pub fn new(knowledge_base_id: impl Into<String>, query: impl Into<String>) -> Self {
        Self {
            knowledge_base_id: knowledge_base_id.into(),
            query: query.into(),
            top_k: default_top_k(),
            similarity_threshold: None,
            filter: None,
        }
    }

    pub fn with_top_k(mut self, top_k: u32) -> Self {
        self.top_k = top_k;
        self
    }

    pub fn with_similarity_threshold(mut self, threshold: f32) -> Self {
        self.similarity_threshold = Some(threshold);
        self
    }

    pub fn with_filter(mut self, filter: serde_json::Value) -> Self {
        self.filter = Some(filter);
        self
    }
}

/// CRAG scoring step configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CragScoringStep {
    /// Model ID to use for LLM-based scoring
    pub model_id: String,

    /// Prompt ID for LLM-based scoring instructions
    /// The prompt should have variables for rendering (e.g., document, query)
    /// and must return a "scores" map (doc_id -> score)
    pub prompt_id: String,

    /// Source for documents array to filter.
    /// Can be a variable reference like "${step:search:documents}" or just "documents".
    /// Defaults to "documents" which looks for documents in the step output.
    #[serde(default = "default_documents_source")]
    pub documents_source: String,

    /// Prompt variables to pass to the scoring prompt
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_variables: Option<std::collections::HashMap<String, String>>,

    /// Relevance threshold (0.0 - 1.0)
    #[serde(default = "default_threshold")]
    pub threshold: f32,

    /// Scoring strategy: "threshold", "llm", or "hybrid"
    #[serde(default = "default_strategy")]
    pub strategy: ScoringStrategy,
}

fn default_documents_source() -> String {
    "documents".to_string()
}

fn default_threshold() -> f32 {
    0.5
}

fn default_strategy() -> ScoringStrategy {
    ScoringStrategy::Hybrid
}

/// CRAG scoring strategy
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ScoringStrategy {
    /// Score based on similarity threshold only
    Threshold,

    /// Score using LLM evaluation
    Llm,

    /// Combine threshold and LLM scoring
    #[default]
    Hybrid,
}

impl CragScoringStep {
    pub fn new(model_id: impl Into<String>, prompt_id: impl Into<String>) -> Self {
        Self {
            model_id: model_id.into(),
            prompt_id: prompt_id.into(),
            documents_source: default_documents_source(),
            prompt_variables: None,
            threshold: default_threshold(),
            strategy: default_strategy(),
        }
    }

    pub fn with_documents_source(mut self, source: impl Into<String>) -> Self {
        self.documents_source = source.into();
        self
    }

    pub fn with_threshold(mut self, threshold: f32) -> Self {
        self.threshold = threshold;
        self
    }

    pub fn with_strategy(mut self, strategy: ScoringStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    pub fn with_prompt_variables(
        mut self,
        variables: std::collections::HashMap<String, String>,
    ) -> Self {
        self.prompt_variables = Some(variables);
        self
    }
}

/// Conditional branching step
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConditionalStep {
    /// Conditions to evaluate (in order, first match wins)
    pub conditions: Vec<Condition>,

    /// Default action if no condition matches
    #[serde(default)]
    pub default_action: ConditionalAction,
}

impl ConditionalStep {
    pub fn new(conditions: Vec<Condition>) -> Self {
        Self {
            conditions,
            default_action: ConditionalAction::Continue,
        }
    }

    pub fn with_default_action(mut self, action: ConditionalAction) -> Self {
        self.default_action = action;
        self
    }

    pub fn add_condition(mut self, condition: Condition) -> Self {
        self.conditions.push(condition);
        self
    }
}

/// A single condition to evaluate
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Condition {
    /// Field to evaluate (variable reference like ${step:search:documents})
    pub field: String,

    /// Comparison operator
    pub operator: ConditionOperator,

    /// Value to compare against
    #[serde(default)]
    pub value: serde_json::Value,

    /// Action to take if condition is true
    pub action: ConditionalAction,
}

impl Condition {
    pub fn new(
        field: impl Into<String>,
        operator: ConditionOperator,
        action: ConditionalAction,
    ) -> Self {
        Self {
            field: field.into(),
            operator,
            value: serde_json::Value::Null,
            action,
        }
    }

    pub fn with_value(mut self, value: serde_json::Value) -> Self {
        self.value = value;
        self
    }
}

/// Condition comparison operators
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConditionOperator {
    /// Equal to
    Eq,

    /// Not equal to
    Ne,

    /// Greater than
    Gt,

    /// Greater than or equal to
    Gte,

    /// Less than
    Lt,

    /// Less than or equal to
    Lte,

    /// Is empty (for arrays/strings/null)
    IsEmpty,

    /// Is not empty
    IsNotEmpty,

    /// Contains (for strings/arrays)
    Contains,

    /// Starts with (for strings)
    StartsWith,

    /// Ends with (for strings)
    EndsWith,
}

impl ConditionOperator {
    /// Evaluate the condition
    pub fn evaluate(&self, field_value: &serde_json::Value, compare_value: &serde_json::Value) -> bool {
        match self {
            Self::Eq => field_value == compare_value,
            Self::Ne => field_value != compare_value,
            Self::Gt => compare_numbers(field_value, compare_value, |a, b| a > b),
            Self::Gte => compare_numbers(field_value, compare_value, |a, b| a >= b),
            Self::Lt => compare_numbers(field_value, compare_value, |a, b| a < b),
            Self::Lte => compare_numbers(field_value, compare_value, |a, b| a <= b),
            Self::IsEmpty => is_empty(field_value),
            Self::IsNotEmpty => !is_empty(field_value),
            Self::Contains => contains(field_value, compare_value),
            Self::StartsWith => starts_with(field_value, compare_value),
            Self::EndsWith => ends_with(field_value, compare_value),
        }
    }
}

fn compare_numbers<F>(a: &serde_json::Value, b: &serde_json::Value, f: F) -> bool
where
    F: Fn(f64, f64) -> bool,
{
    match (a.as_f64(), b.as_f64()) {
        (Some(a), Some(b)) => f(a, b),
        _ => false,
    }
}

fn is_empty(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Null => true,
        serde_json::Value::String(s) => s.is_empty(),
        serde_json::Value::Array(arr) => arr.is_empty(),
        serde_json::Value::Object(obj) => obj.is_empty(),
        _ => false,
    }
}

fn contains(field: &serde_json::Value, value: &serde_json::Value) -> bool {
    match field {
        serde_json::Value::String(s) => {
            if let Some(v) = value.as_str() {
                s.contains(v)
            } else {
                false
            }
        }
        serde_json::Value::Array(arr) => arr.contains(value),
        _ => false,
    }
}

fn starts_with(field: &serde_json::Value, value: &serde_json::Value) -> bool {
    match (field.as_str(), value.as_str()) {
        (Some(f), Some(v)) => f.starts_with(v),
        _ => false,
    }
}

fn ends_with(field: &serde_json::Value, value: &serde_json::Value) -> bool {
    match (field.as_str(), value.as_str()) {
        (Some(f), Some(v)) => f.ends_with(v),
        _ => false,
    }
}

/// Action to take in conditional branching
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ConditionalAction {
    /// Continue to the next step
    #[default]
    Continue,

    /// Jump to a specific step by name
    GoToStep(String),

    /// End the workflow with an optional output value
    EndWorkflow(Option<serde_json::Value>),
}

impl ConditionalAction {
    pub fn go_to_step(name: impl Into<String>) -> Self {
        Self::GoToStep(name.into())
    }

    pub fn end_workflow() -> Self {
        Self::EndWorkflow(None)
    }

    pub fn end_workflow_with(output: serde_json::Value) -> Self {
        Self::EndWorkflow(Some(output))
    }
}

/// HTTP request method
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    #[default]
    GET,
    POST,
    PUT,
    DELETE,
    PATCH,
    HEAD,
    OPTIONS,
}

/// HTTP request step configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HttpRequestStep {
    /// External API ID to use (required) - provides base URL and base headers
    pub external_api_id: String,

    /// Credential ID for authentication (optional, must be of type HttpApiKey)
    /// Provides auth header (header_name + header_value with ${api-key} interpolation)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credential_id: Option<String>,

    /// URI path to append to the external API's base URL (can contain variable references)
    /// Example: "/users/${input:user_id}" will be appended to the external API's base_url
    #[serde(default)]
    pub path: String,

    /// HTTP method
    #[serde(default)]
    pub method: HttpMethod,

    /// Additional request headers (key -> value, values can contain variable references)
    /// These are merged with the external API's base_headers
    #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
    pub headers: std::collections::HashMap<String, String>,

    /// Request body (can contain variable references or be a JSON template)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<serde_json::Value>,

    /// Timeout in milliseconds (default: 30000)
    #[serde(default = "default_http_timeout")]
    pub timeout_ms: u64,

    /// Expected response content type (default: "application/json")
    #[serde(default = "default_content_type")]
    pub expected_content_type: String,

    /// Whether to fail on non-2xx status codes (default: true)
    #[serde(default = "default_true")]
    pub fail_on_error: bool,

    /// JSON path to extract from response (optional, e.g., "$.data.result")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extract_path: Option<String>,
}

fn default_http_timeout() -> u64 {
    30000
}

fn default_content_type() -> String {
    "application/json".to_string()
}

fn default_true() -> bool {
    true
}

impl HttpRequestStep {
    /// Create a new HTTP request step with an external API ID
    pub fn new(external_api_id: impl Into<String>) -> Self {
        Self {
            external_api_id: external_api_id.into(),
            credential_id: None,
            path: String::new(),
            method: HttpMethod::GET,
            headers: std::collections::HashMap::new(),
            body: None,
            timeout_ms: default_http_timeout(),
            expected_content_type: default_content_type(),
            fail_on_error: true,
            extract_path: None,
        }
    }

    /// Set the credential ID for authentication
    pub fn with_credential(mut self, credential_id: impl Into<String>) -> Self {
        self.credential_id = Some(credential_id.into());
        self
    }

    /// Set the URI path
    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.path = path.into();
        self
    }

    /// Set the HTTP method
    pub fn with_method(mut self, method: HttpMethod) -> Self {
        self.method = method;
        self
    }

    /// Add a header
    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    /// Set the request body
    pub fn with_body(mut self, body: serde_json::Value) -> Self {
        self.body = Some(body);
        self
    }

    /// Set the timeout in milliseconds
    pub fn with_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    /// Set the expected content type
    pub fn with_expected_content_type(mut self, content_type: impl Into<String>) -> Self {
        self.expected_content_type = content_type.into();
        self
    }

    /// Set whether to fail on non-2xx status codes
    pub fn with_fail_on_error(mut self, fail_on_error: bool) -> Self {
        self.fail_on_error = fail_on_error;
        self
    }

    /// Set the JSON path to extract from response
    pub fn with_extract_path(mut self, path: impl Into<String>) -> Self {
        self.extract_path = Some(path.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_chat_completion_step_builder() {
        let step = ChatCompletionStep::new("gpt-4", "helpful-assistant")
            .with_prompt_variable("question", "${request:question}")
            .with_temperature(0.7)
            .with_max_tokens(1000);

        assert_eq!(step.model_id, "gpt-4");
        assert_eq!(step.prompt_id, "helpful-assistant");
        assert_eq!(
            step.prompt_variables.get("question"),
            Some(&"${request:question}".to_string())
        );
        assert_eq!(step.temperature, Some(0.7));
        assert_eq!(step.max_tokens, Some(1000));
    }

    #[test]
    fn test_kb_search_step_builder() {
        let step = KnowledgeBaseSearchStep::new("docs-kb", "${request:query}")
            .with_top_k(5)
            .with_similarity_threshold(0.8);

        assert_eq!(step.knowledge_base_id, "docs-kb");
        assert_eq!(step.query, "${request:query}");
        assert_eq!(step.top_k, 5);
        assert_eq!(step.similarity_threshold, Some(0.8));
    }

    #[test]
    fn test_crag_scoring_step_builder() {
        let step = CragScoringStep::new("gpt-4o", "crag-scorer")
            .with_threshold(0.6)
            .with_strategy(ScoringStrategy::Llm);

        assert_eq!(step.model_id, "gpt-4o");
        assert_eq!(step.prompt_id, "crag-scorer");
        assert_eq!(step.threshold, 0.6);
        assert_eq!(step.strategy, ScoringStrategy::Llm);
    }

    #[test]
    fn test_conditional_step_builder() {
        let step = ConditionalStep::new(vec![
            Condition::new(
                "${step:search:documents}",
                ConditionOperator::IsEmpty,
                ConditionalAction::end_workflow_with(json!({"error": "No results"})),
            ),
        ])
        .with_default_action(ConditionalAction::Continue);

        assert_eq!(step.conditions.len(), 1);
        assert_eq!(step.default_action, ConditionalAction::Continue);
    }

    #[test]
    fn test_condition_operator_eq() {
        let op = ConditionOperator::Eq;
        assert!(op.evaluate(&json!("test"), &json!("test")));
        assert!(!op.evaluate(&json!("test"), &json!("other")));
        assert!(op.evaluate(&json!(42), &json!(42)));
    }

    #[test]
    fn test_condition_operator_comparisons() {
        assert!(ConditionOperator::Gt.evaluate(&json!(10), &json!(5)));
        assert!(!ConditionOperator::Gt.evaluate(&json!(5), &json!(10)));

        assert!(ConditionOperator::Gte.evaluate(&json!(10), &json!(10)));
        assert!(ConditionOperator::Lt.evaluate(&json!(5), &json!(10)));
        assert!(ConditionOperator::Lte.evaluate(&json!(10), &json!(10)));
    }

    #[test]
    fn test_condition_operator_is_empty() {
        let op = ConditionOperator::IsEmpty;
        assert!(op.evaluate(&json!(null), &json!(null)));
        assert!(op.evaluate(&json!(""), &json!(null)));
        assert!(op.evaluate(&json!([]), &json!(null)));
        assert!(op.evaluate(&json!({}), &json!(null)));
        assert!(!op.evaluate(&json!("text"), &json!(null)));
        assert!(!op.evaluate(&json!([1, 2, 3]), &json!(null)));
    }

    #[test]
    fn test_condition_operator_contains() {
        let op = ConditionOperator::Contains;
        assert!(op.evaluate(&json!("hello world"), &json!("world")));
        assert!(!op.evaluate(&json!("hello"), &json!("world")));
        assert!(op.evaluate(&json!([1, 2, 3]), &json!(2)));
        assert!(!op.evaluate(&json!([1, 2, 3]), &json!(4)));
    }

    #[test]
    fn test_condition_operator_starts_ends_with() {
        assert!(ConditionOperator::StartsWith.evaluate(&json!("hello world"), &json!("hello")));
        assert!(!ConditionOperator::StartsWith.evaluate(&json!("hello world"), &json!("world")));

        assert!(ConditionOperator::EndsWith.evaluate(&json!("hello world"), &json!("world")));
        assert!(!ConditionOperator::EndsWith.evaluate(&json!("hello world"), &json!("hello")));
    }

    #[test]
    fn test_workflow_step_type_serialization() {
        let step = WorkflowStepType::ChatCompletion(ChatCompletionStep::new(
            "gpt-4",
            "helpful-assistant",
        ));

        let json = serde_json::to_string(&step).unwrap();
        assert!(json.contains("\"type\":\"chat_completion\""));
        assert!(json.contains("\"model_id\":\"gpt-4\""));
        assert!(json.contains("\"prompt_id\":\"helpful-assistant\""));

        let deserialized: WorkflowStepType = serde_json::from_str(&json).unwrap();
        assert_eq!(step, deserialized);
    }

    #[test]
    fn test_conditional_action_serialization() {
        let action = ConditionalAction::go_to_step("error-handler");
        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("go_to_step"));

        let action = ConditionalAction::end_workflow_with(json!({"status": "done"}));
        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("end_workflow"));
    }
}
