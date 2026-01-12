//! Test case entity and related types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::domain::storage::{StorageEntity, StorageKey};
use crate::domain::{validate_model_id, ModelValidationError};

/// Test case identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct TestCaseId(String);

impl TestCaseId {
    pub fn new(id: impl Into<String>) -> Result<Self, ModelValidationError> {
        let id = id.into();
        validate_model_id(&id)?;
        Ok(Self(id))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for TestCaseId {
    type Error = ModelValidationError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<TestCaseId> for String {
    fn from(id: TestCaseId) -> Self {
        id.0
    }
}

impl std::fmt::Display for TestCaseId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl StorageKey for TestCaseId {
    fn as_str(&self) -> &str {
        &self.0
    }
}

/// Type of test case
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TestCaseType {
    /// Test a model + prompt combination
    ModelPrompt,
    /// Test a workflow
    Workflow,
}

impl std::fmt::Display for TestCaseType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TestCaseType::ModelPrompt => write!(f, "model_prompt"),
            TestCaseType::Workflow => write!(f, "workflow"),
        }
    }
}

/// Input configuration for a model+prompt test case
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPromptInput {
    /// Model ID to use
    pub model_id: String,
    /// Optional prompt ID for system message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_id: Option<String>,
    /// Variables for prompt templating
    #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
    pub variables: std::collections::HashMap<String, String>,
    /// User message to send
    pub user_message: String,
    /// Optional temperature override
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Optional max_tokens override
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
}

/// Input configuration for a workflow test case
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowInput {
    /// Workflow ID to execute
    pub workflow_id: String,
    /// Input data for the workflow
    #[serde(default)]
    pub input: Value,
}

/// Input configuration for a test case
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TestCaseInput {
    /// Model + prompt test
    ModelPrompt(ModelPromptInput),
    /// Workflow test
    Workflow(WorkflowInput),
}

/// Assertion operator for output validation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssertionOperator {
    /// Output contains the expected string
    Contains,
    /// Output does not contain the string
    NotContains,
    /// Output matches the regex pattern
    Regex,
    /// Output equals exactly
    Equals,
    /// Output does not equal
    NotEquals,
    /// JSON path exists in output
    JsonPathExists,
    /// JSON path equals value
    JsonPathEquals,
    /// Output length is greater than
    LengthGreaterThan,
    /// Output length is less than
    LengthLessThan,
}

impl std::fmt::Display for AssertionOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AssertionOperator::Contains => write!(f, "contains"),
            AssertionOperator::NotContains => write!(f, "not_contains"),
            AssertionOperator::Regex => write!(f, "regex"),
            AssertionOperator::Equals => write!(f, "equals"),
            AssertionOperator::NotEquals => write!(f, "not_equals"),
            AssertionOperator::JsonPathExists => write!(f, "json_path_exists"),
            AssertionOperator::JsonPathEquals => write!(f, "json_path_equals"),
            AssertionOperator::LengthGreaterThan => write!(f, "length_greater_than"),
            AssertionOperator::LengthLessThan => write!(f, "length_less_than"),
        }
    }
}

/// A single assertion criterion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssertionCriteria {
    /// Human-readable name for this assertion
    pub name: String,
    /// The operator to use
    pub operator: AssertionOperator,
    /// Expected value or pattern
    pub expected: String,
    /// Optional JSON path for JSON assertions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub json_path: Option<String>,
}

impl AssertionCriteria {
    pub fn contains(name: impl Into<String>, expected: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            operator: AssertionOperator::Contains,
            expected: expected.into(),
            json_path: None,
        }
    }

    pub fn not_contains(name: impl Into<String>, expected: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            operator: AssertionOperator::NotContains,
            expected: expected.into(),
            json_path: None,
        }
    }

    pub fn regex(name: impl Into<String>, pattern: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            operator: AssertionOperator::Regex,
            expected: pattern.into(),
            json_path: None,
        }
    }

    pub fn equals(name: impl Into<String>, expected: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            operator: AssertionOperator::Equals,
            expected: expected.into(),
            json_path: None,
        }
    }

    pub fn json_path_exists(name: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            operator: AssertionOperator::JsonPathExists,
            expected: String::new(),
            json_path: Some(path.into()),
        }
    }

    pub fn json_path_equals(
        name: impl Into<String>,
        path: impl Into<String>,
        expected: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            operator: AssertionOperator::JsonPathEquals,
            expected: expected.into(),
            json_path: Some(path.into()),
        }
    }

    pub fn length_greater_than(name: impl Into<String>, length: usize) -> Self {
        Self {
            name: name.into(),
            operator: AssertionOperator::LengthGreaterThan,
            expected: length.to_string(),
            json_path: None,
        }
    }

    pub fn length_less_than(name: impl Into<String>, length: usize) -> Self {
        Self {
            name: name.into(),
            operator: AssertionOperator::LengthLessThan,
            expected: length.to_string(),
            json_path: None,
        }
    }
}

/// A test case for validating model/workflow outputs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCase {
    /// Unique identifier
    id: TestCaseId,
    /// Display name
    name: String,
    /// Description of what this test validates
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    /// Type of test case
    test_type: TestCaseType,
    /// Input configuration
    input: TestCaseInput,
    /// Assertions to validate output
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    assertions: Vec<AssertionCriteria>,
    /// Tags for organization
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    tags: Vec<String>,
    /// Whether the test case is enabled
    enabled: bool,
    /// Creation timestamp
    created_at: DateTime<Utc>,
    /// Last update timestamp
    updated_at: DateTime<Utc>,
}

impl TestCase {
    /// Create a new model+prompt test case
    pub fn model_prompt(
        id: TestCaseId,
        name: impl Into<String>,
        input: ModelPromptInput,
    ) -> Self {
        let now = Utc::now();
        Self {
            id,
            name: name.into(),
            description: None,
            test_type: TestCaseType::ModelPrompt,
            input: TestCaseInput::ModelPrompt(input),
            assertions: Vec::new(),
            tags: Vec::new(),
            enabled: true,
            created_at: now,
            updated_at: now,
        }
    }

    /// Create a new workflow test case
    pub fn workflow(id: TestCaseId, name: impl Into<String>, input: WorkflowInput) -> Self {
        let now = Utc::now();
        Self {
            id,
            name: name.into(),
            description: None,
            test_type: TestCaseType::Workflow,
            input: TestCaseInput::Workflow(input),
            assertions: Vec::new(),
            tags: Vec::new(),
            enabled: true,
            created_at: now,
            updated_at: now,
        }
    }

    // Builder methods
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_assertion(mut self, assertion: AssertionCriteria) -> Self {
        self.assertions.push(assertion);
        self
    }

    pub fn with_assertions(mut self, assertions: Vec<AssertionCriteria>) -> Self {
        self.assertions = assertions;
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    // Getters
    pub fn id(&self) -> &TestCaseId {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    pub fn test_type(&self) -> &TestCaseType {
        &self.test_type
    }

    pub fn input(&self) -> &TestCaseInput {
        &self.input
    }

    pub fn assertions(&self) -> &[AssertionCriteria] {
        &self.assertions
    }

    pub fn tags(&self) -> &[String] {
        &self.tags
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    // Mutators
    pub fn set_name(&mut self, name: impl Into<String>) {
        self.name = name.into();
        self.touch();
    }

    pub fn set_description(&mut self, description: Option<String>) {
        self.description = description;
        self.touch();
    }

    pub fn set_input(&mut self, input: TestCaseInput) {
        // Update test_type based on input
        self.test_type = match &input {
            TestCaseInput::ModelPrompt(_) => TestCaseType::ModelPrompt,
            TestCaseInput::Workflow(_) => TestCaseType::Workflow,
        };
        self.input = input;
        self.touch();
    }

    pub fn set_assertions(&mut self, assertions: Vec<AssertionCriteria>) {
        self.assertions = assertions;
        self.touch();
    }

    pub fn add_assertion(&mut self, assertion: AssertionCriteria) {
        self.assertions.push(assertion);
        self.touch();
    }

    pub fn set_tags(&mut self, tags: Vec<String>) {
        self.tags = tags;
        self.touch();
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        self.touch();
    }

    fn touch(&mut self) {
        self.updated_at = Utc::now();
    }
}

impl StorageEntity for TestCase {
    type Key = TestCaseId;

    fn key(&self) -> &Self::Key {
        &self.id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_case_id(id: &str) -> TestCaseId {
        TestCaseId::new(id).unwrap()
    }

    #[test]
    fn test_test_case_id_valid() {
        let id = TestCaseId::new("my-test-case-1").unwrap();
        assert_eq!(id.as_str(), "my-test-case-1");
    }

    #[test]
    fn test_test_case_id_invalid() {
        let result = TestCaseId::new("invalid test!");
        assert!(result.is_err());
    }

    #[test]
    fn test_model_prompt_test_case() {
        let input = ModelPromptInput {
            model_id: "gpt-4".to_string(),
            prompt_id: Some("system-prompt".to_string()),
            variables: std::collections::HashMap::new(),
            user_message: "Hello".to_string(),
            temperature: Some(0.7),
            max_tokens: None,
        };

        let test_case = TestCase::model_prompt(
            create_test_case_id("test-1"),
            "Test GPT-4",
            input,
        )
        .with_description("Test GPT-4 greeting")
        .with_assertion(AssertionCriteria::contains("has_hello", "hello"));

        assert_eq!(test_case.id().as_str(), "test-1");
        assert_eq!(test_case.name(), "Test GPT-4");
        assert_eq!(test_case.test_type(), &TestCaseType::ModelPrompt);
        assert_eq!(test_case.assertions().len(), 1);
        assert!(test_case.is_enabled());
    }

    #[test]
    fn test_workflow_test_case() {
        let input = WorkflowInput {
            workflow_id: "basic-rag".to_string(),
            input: serde_json::json!({"query": "What is Rust?"}),
        };

        let test_case = TestCase::workflow(
            create_test_case_id("test-workflow-1"),
            "Test RAG workflow",
            input,
        )
        .with_enabled(false);

        assert_eq!(test_case.test_type(), &TestCaseType::Workflow);
        assert!(!test_case.is_enabled());
    }

    #[test]
    fn test_assertion_criteria_builders() {
        let contains = AssertionCriteria::contains("test", "hello");
        assert_eq!(contains.operator, AssertionOperator::Contains);
        assert_eq!(contains.expected, "hello");

        let regex = AssertionCriteria::regex("pattern", r"\d+");
        assert_eq!(regex.operator, AssertionOperator::Regex);

        let json_path = AssertionCriteria::json_path_equals("check_name", "$.name", "test");
        assert_eq!(json_path.operator, AssertionOperator::JsonPathEquals);
        assert_eq!(json_path.json_path, Some("$.name".to_string()));
    }

    #[test]
    fn test_test_case_serialization() {
        let input = ModelPromptInput {
            model_id: "gpt-4".to_string(),
            prompt_id: None,
            variables: std::collections::HashMap::new(),
            user_message: "Hi".to_string(),
            temperature: None,
            max_tokens: None,
        };

        let test_case = TestCase::model_prompt(
            create_test_case_id("ser-test"),
            "Serialization Test",
            input,
        );

        let json = serde_json::to_string(&test_case).unwrap();
        let deserialized: TestCase = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.id().as_str(), "ser-test");
        assert_eq!(deserialized.name(), "Serialization Test");
    }
}
