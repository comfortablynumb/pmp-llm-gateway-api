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
}

impl WorkflowStepType {
    /// Get a human-readable type name
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::ChatCompletion(_) => "chat_completion",
            Self::KnowledgeBaseSearch(_) => "knowledge_base_search",
            Self::CragScoring(_) => "crag_scoring",
            Self::Conditional(_) => "conditional",
        }
    }
}

/// Chat completion step configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChatCompletionStep {
    /// Model ID to use for completion
    pub model_id: String,

    /// Optional prompt ID to use as template
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_id: Option<String>,

    /// Optional system message (can contain variable references)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_message: Option<String>,

    /// User message (can contain variable references)
    pub user_message: String,

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
    pub fn new(model_id: impl Into<String>, user_message: impl Into<String>) -> Self {
        Self {
            model_id: model_id.into(),
            prompt_id: None,
            system_message: None,
            user_message: user_message.into(),
            temperature: None,
            max_tokens: None,
            top_p: None,
        }
    }

    pub fn with_prompt_id(mut self, prompt_id: impl Into<String>) -> Self {
        self.prompt_id = Some(prompt_id.into());
        self
    }

    pub fn with_system_message(mut self, message: impl Into<String>) -> Self {
        self.system_message = Some(message.into());
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
    /// Reference to input documents (variable reference like ${step:search:documents})
    pub input_documents: String,

    /// Query for relevance scoring (can contain variable references)
    pub query: String,

    /// Relevance threshold (0.0 - 1.0)
    #[serde(default = "default_threshold")]
    pub threshold: f32,

    /// Scoring strategy: "threshold", "llm", or "hybrid"
    #[serde(default = "default_strategy")]
    pub strategy: ScoringStrategy,

    /// Optional prompt ID for LLM-based scoring
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_id: Option<String>,
}

fn default_threshold() -> f32 {
    0.5
}

fn default_strategy() -> ScoringStrategy {
    ScoringStrategy::Threshold
}

/// CRAG scoring strategy
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ScoringStrategy {
    /// Score based on similarity threshold only
    #[default]
    Threshold,

    /// Score using LLM evaluation
    Llm,

    /// Combine threshold and LLM scoring
    Hybrid,
}

impl CragScoringStep {
    pub fn new(input_documents: impl Into<String>, query: impl Into<String>) -> Self {
        Self {
            input_documents: input_documents.into(),
            query: query.into(),
            threshold: default_threshold(),
            strategy: default_strategy(),
            prompt_id: None,
        }
    }

    pub fn with_threshold(mut self, threshold: f32) -> Self {
        self.threshold = threshold;
        self
    }

    pub fn with_strategy(mut self, strategy: ScoringStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    pub fn with_prompt_id(mut self, prompt_id: impl Into<String>) -> Self {
        self.prompt_id = Some(prompt_id.into());
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_chat_completion_step_builder() {
        let step = ChatCompletionStep::new("gpt-4", "Hello ${request:question}")
            .with_system_message("You are a helpful assistant")
            .with_temperature(0.7)
            .with_max_tokens(1000);

        assert_eq!(step.model_id, "gpt-4");
        assert_eq!(step.user_message, "Hello ${request:question}");
        assert_eq!(step.system_message, Some("You are a helpful assistant".to_string()));
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
        let step = CragScoringStep::new("${step:search:documents}", "${request:query}")
            .with_threshold(0.6)
            .with_strategy(ScoringStrategy::Hybrid);

        assert_eq!(step.input_documents, "${step:search:documents}");
        assert_eq!(step.threshold, 0.6);
        assert_eq!(step.strategy, ScoringStrategy::Hybrid);
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
        let step = WorkflowStepType::ChatCompletion(
            ChatCompletionStep::new("gpt-4", "Hello")
        );

        let json = serde_json::to_string(&step).unwrap();
        assert!(json.contains("\"type\":\"chat_completion\""));
        assert!(json.contains("\"model_id\":\"gpt-4\""));

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
