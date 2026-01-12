//! Test case result types

use chrono::{DateTime, Utc};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use super::{AssertionCriteria, AssertionOperator, TestCaseId};
use crate::domain::storage::{StorageEntity, StorageKey};

/// Unique identifier for a test case result
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TestCaseResultId(String);

impl TestCaseResultId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    pub fn from_string(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for TestCaseResultId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for TestCaseResultId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl StorageKey for TestCaseResultId {
    fn as_str(&self) -> &str {
        &self.0
    }
}

impl StorageEntity for TestCaseResult {
    type Key = TestCaseResultId;

    fn key(&self) -> &Self::Key {
        &self.id
    }
}

/// Result of a single assertion check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssertionResult {
    /// Name of the assertion
    pub name: String,
    /// Whether the assertion passed
    pub passed: bool,
    /// The operator used
    pub operator: AssertionOperator,
    /// Expected value
    pub expected: String,
    /// Actual value (relevant portion)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actual: Option<String>,
    /// Error message if failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl AssertionResult {
    pub fn passed(name: impl Into<String>, operator: AssertionOperator, expected: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            passed: true,
            operator,
            expected: expected.into(),
            actual: None,
            error: None,
        }
    }

    pub fn failed(
        name: impl Into<String>,
        operator: AssertionOperator,
        expected: impl Into<String>,
        actual: Option<String>,
        error: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            passed: false,
            operator,
            expected: expected.into(),
            actual,
            error: Some(error.into()),
        }
    }
}

/// Result of running a test case
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCaseResult {
    /// Unique result ID
    id: TestCaseResultId,
    /// Reference to the test case
    test_case_id: TestCaseId,
    /// Whether the test passed (all assertions passed)
    passed: bool,
    /// The actual output from execution
    #[serde(skip_serializing_if = "Option::is_none")]
    output: Option<String>,
    /// For workflows, the full JSON output
    #[serde(skip_serializing_if = "Option::is_none")]
    output_json: Option<Value>,
    /// Individual assertion results
    assertion_results: Vec<AssertionResult>,
    /// Execution time in milliseconds
    execution_time_ms: u64,
    /// Token usage (for model tests)
    #[serde(skip_serializing_if = "Option::is_none")]
    tokens_used: Option<TokenUsage>,
    /// Error if execution failed
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    /// When the test was run
    executed_at: DateTime<Utc>,
}

/// Token usage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

impl TestCaseResult {
    /// Create a successful test result
    pub fn success(
        test_case_id: TestCaseId,
        output: String,
        assertion_results: Vec<AssertionResult>,
        execution_time_ms: u64,
    ) -> Self {
        let passed = assertion_results.iter().all(|r| r.passed);
        Self {
            id: TestCaseResultId::new(),
            test_case_id,
            passed,
            output: Some(output),
            output_json: None,
            assertion_results,
            execution_time_ms,
            tokens_used: None,
            error: None,
            executed_at: Utc::now(),
        }
    }

    /// Create a successful workflow test result
    pub fn workflow_success(
        test_case_id: TestCaseId,
        output_json: Value,
        assertion_results: Vec<AssertionResult>,
        execution_time_ms: u64,
    ) -> Self {
        let passed = assertion_results.iter().all(|r| r.passed);
        let output_str = serde_json::to_string_pretty(&output_json).ok();
        Self {
            id: TestCaseResultId::new(),
            test_case_id,
            passed,
            output: output_str,
            output_json: Some(output_json),
            assertion_results,
            execution_time_ms,
            tokens_used: None,
            error: None,
            executed_at: Utc::now(),
        }
    }

    /// Create a failed test result (execution error)
    pub fn execution_error(
        test_case_id: TestCaseId,
        error: impl Into<String>,
        execution_time_ms: u64,
    ) -> Self {
        Self {
            id: TestCaseResultId::new(),
            test_case_id,
            passed: false,
            output: None,
            output_json: None,
            assertion_results: Vec::new(),
            execution_time_ms,
            tokens_used: None,
            error: Some(error.into()),
            executed_at: Utc::now(),
        }
    }

    // Builder methods
    pub fn with_tokens(mut self, tokens: TokenUsage) -> Self {
        self.tokens_used = Some(tokens);
        self
    }

    // Getters
    pub fn id(&self) -> &TestCaseResultId {
        &self.id
    }

    pub fn test_case_id(&self) -> &TestCaseId {
        &self.test_case_id
    }

    pub fn passed(&self) -> bool {
        self.passed
    }

    pub fn output(&self) -> Option<&str> {
        self.output.as_deref()
    }

    pub fn output_json(&self) -> Option<&Value> {
        self.output_json.as_ref()
    }

    pub fn assertion_results(&self) -> &[AssertionResult] {
        &self.assertion_results
    }

    pub fn execution_time_ms(&self) -> u64 {
        self.execution_time_ms
    }

    pub fn tokens_used(&self) -> Option<&TokenUsage> {
        self.tokens_used.as_ref()
    }

    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub fn executed_at(&self) -> DateTime<Utc> {
        self.executed_at
    }

    /// Get count of passed assertions
    pub fn passed_count(&self) -> usize {
        self.assertion_results.iter().filter(|r| r.passed).count()
    }

    /// Get count of failed assertions
    pub fn failed_count(&self) -> usize {
        self.assertion_results.iter().filter(|r| !r.passed).count()
    }
}

/// Assertion evaluator
pub struct AssertionEvaluator;

impl AssertionEvaluator {
    /// Evaluate a single assertion against output
    pub fn evaluate(criteria: &AssertionCriteria, output: &str) -> AssertionResult {
        match criteria.operator {
            AssertionOperator::Contains => {
                if output.contains(&criteria.expected) {
                    AssertionResult::passed(&criteria.name, criteria.operator.clone(), &criteria.expected)
                } else {
                    AssertionResult::failed(
                        &criteria.name,
                        criteria.operator.clone(),
                        &criteria.expected,
                        Some(Self::truncate_output(output)),
                        format!("Output does not contain '{}'", criteria.expected),
                    )
                }
            }

            AssertionOperator::NotContains => {
                if !output.contains(&criteria.expected) {
                    AssertionResult::passed(&criteria.name, criteria.operator.clone(), &criteria.expected)
                } else {
                    AssertionResult::failed(
                        &criteria.name,
                        criteria.operator.clone(),
                        &criteria.expected,
                        Some(Self::truncate_output(output)),
                        format!("Output contains '{}' but should not", criteria.expected),
                    )
                }
            }

            AssertionOperator::Regex => {
                match Regex::new(&criteria.expected) {
                    Ok(re) => {
                        if re.is_match(output) {
                            AssertionResult::passed(&criteria.name, criteria.operator.clone(), &criteria.expected)
                        } else {
                            AssertionResult::failed(
                                &criteria.name,
                                criteria.operator.clone(),
                                &criteria.expected,
                                Some(Self::truncate_output(output)),
                                "Output does not match regex pattern",
                            )
                        }
                    }
                    Err(e) => AssertionResult::failed(
                        &criteria.name,
                        criteria.operator.clone(),
                        &criteria.expected,
                        None,
                        format!("Invalid regex: {}", e),
                    ),
                }
            }

            AssertionOperator::Equals => {
                if output.trim() == criteria.expected.trim() {
                    AssertionResult::passed(&criteria.name, criteria.operator.clone(), &criteria.expected)
                } else {
                    AssertionResult::failed(
                        &criteria.name,
                        criteria.operator.clone(),
                        &criteria.expected,
                        Some(Self::truncate_output(output)),
                        "Output does not equal expected value",
                    )
                }
            }

            AssertionOperator::NotEquals => {
                if output.trim() != criteria.expected.trim() {
                    AssertionResult::passed(&criteria.name, criteria.operator.clone(), &criteria.expected)
                } else {
                    AssertionResult::failed(
                        &criteria.name,
                        criteria.operator.clone(),
                        &criteria.expected,
                        Some(Self::truncate_output(output)),
                        "Output equals forbidden value",
                    )
                }
            }

            AssertionOperator::LengthGreaterThan => {
                match criteria.expected.parse::<usize>() {
                    Ok(expected_len) => {
                        if output.len() > expected_len {
                            AssertionResult::passed(&criteria.name, criteria.operator.clone(), &criteria.expected)
                        } else {
                            AssertionResult::failed(
                                &criteria.name,
                                criteria.operator.clone(),
                                &criteria.expected,
                                Some(format!("length: {}", output.len())),
                                format!("Output length {} is not greater than {}", output.len(), expected_len),
                            )
                        }
                    }
                    Err(_) => AssertionResult::failed(
                        &criteria.name,
                        criteria.operator.clone(),
                        &criteria.expected,
                        None,
                        "Invalid length value",
                    ),
                }
            }

            AssertionOperator::LengthLessThan => {
                match criteria.expected.parse::<usize>() {
                    Ok(expected_len) => {
                        if output.len() < expected_len {
                            AssertionResult::passed(&criteria.name, criteria.operator.clone(), &criteria.expected)
                        } else {
                            AssertionResult::failed(
                                &criteria.name,
                                criteria.operator.clone(),
                                &criteria.expected,
                                Some(format!("length: {}", output.len())),
                                format!("Output length {} is not less than {}", output.len(), expected_len),
                            )
                        }
                    }
                    Err(_) => AssertionResult::failed(
                        &criteria.name,
                        criteria.operator.clone(),
                        &criteria.expected,
                        None,
                        "Invalid length value",
                    ),
                }
            }

            AssertionOperator::JsonPathExists | AssertionOperator::JsonPathEquals => {
                Self::evaluate_json_assertion(criteria, output)
            }
        }
    }

    /// Evaluate JSON-based assertions
    fn evaluate_json_assertion(criteria: &AssertionCriteria, output: &str) -> AssertionResult {
        let json_path = match &criteria.json_path {
            Some(path) => path,
            None => {
                return AssertionResult::failed(
                    &criteria.name,
                    criteria.operator.clone(),
                    &criteria.expected,
                    None,
                    "JSON path not specified",
                );
            }
        };

        // Try to parse output as JSON
        let json: Value = match serde_json::from_str(output) {
            Ok(v) => v,
            Err(e) => {
                return AssertionResult::failed(
                    &criteria.name,
                    criteria.operator.clone(),
                    &criteria.expected,
                    Some(Self::truncate_output(output)),
                    format!("Output is not valid JSON: {}", e),
                );
            }
        };

        // Simple JSON path evaluation (supports $.field.subfield syntax)
        let value = Self::get_json_path(&json, json_path);

        match criteria.operator {
            AssertionOperator::JsonPathExists => {
                if value.is_some() {
                    AssertionResult::passed(&criteria.name, criteria.operator.clone(), json_path)
                } else {
                    AssertionResult::failed(
                        &criteria.name,
                        criteria.operator.clone(),
                        json_path,
                        None,
                        format!("JSON path '{}' does not exist", json_path),
                    )
                }
            }

            AssertionOperator::JsonPathEquals => {
                match value {
                    Some(v) => {
                        let actual_str = match v {
                            Value::String(s) => s.clone(),
                            other => other.to_string(),
                        };

                        if actual_str == criteria.expected {
                            AssertionResult::passed(&criteria.name, criteria.operator.clone(), &criteria.expected)
                        } else {
                            AssertionResult::failed(
                                &criteria.name,
                                criteria.operator.clone(),
                                &criteria.expected,
                                Some(actual_str),
                                format!("JSON path '{}' value does not match", json_path),
                            )
                        }
                    }
                    None => AssertionResult::failed(
                        &criteria.name,
                        criteria.operator.clone(),
                        &criteria.expected,
                        None,
                        format!("JSON path '{}' does not exist", json_path),
                    ),
                }
            }
            _ => AssertionResult::failed(
                &criteria.name,
                criteria.operator.clone(),
                &criteria.expected,
                None,
                "Invalid operator for JSON assertion",
            ),
        }
    }

    /// Simple JSON path evaluation
    fn get_json_path<'a>(json: &'a Value, path: &str) -> Option<&'a Value> {
        let path = path.trim_start_matches('$').trim_start_matches('.');
        let parts: Vec<&str> = path.split('.').filter(|p| !p.is_empty()).collect();

        let mut current = json;

        for part in parts {
            // Handle array indexing like field[0]
            if let Some(bracket_pos) = part.find('[') {
                let field = &part[..bracket_pos];
                let index_str = &part[bracket_pos + 1..part.len() - 1];

                if !field.is_empty() {
                    current = current.get(field)?;
                }

                if let Ok(index) = index_str.parse::<usize>() {
                    current = current.get(index)?;
                } else {
                    return None;
                }
            } else {
                current = current.get(part)?;
            }
        }

        Some(current)
    }

    fn truncate_output(output: &str) -> String {
        if output.len() > 200 {
            format!("{}...", &output[..200])
        } else {
            output.to_string()
        }
    }

    /// Evaluate all assertions
    pub fn evaluate_all(criteria: &[AssertionCriteria], output: &str) -> Vec<AssertionResult> {
        criteria.iter().map(|c| Self::evaluate(c, output)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_assertion_contains() {
        let criteria = AssertionCriteria::contains("test", "hello");
        let result = AssertionEvaluator::evaluate(&criteria, "hello world");
        assert!(result.passed);

        let result = AssertionEvaluator::evaluate(&criteria, "goodbye world");
        assert!(!result.passed);
    }

    #[test]
    fn test_assertion_not_contains() {
        let criteria = AssertionCriteria::not_contains("test", "error");
        let result = AssertionEvaluator::evaluate(&criteria, "success");
        assert!(result.passed);

        let result = AssertionEvaluator::evaluate(&criteria, "error occurred");
        assert!(!result.passed);
    }

    #[test]
    fn test_assertion_regex() {
        let criteria = AssertionCriteria::regex("test", r"\d{3}-\d{4}");
        let result = AssertionEvaluator::evaluate(&criteria, "Call 555-1234");
        assert!(result.passed);

        let result = AssertionEvaluator::evaluate(&criteria, "No phone");
        assert!(!result.passed);
    }

    #[test]
    fn test_assertion_equals() {
        let criteria = AssertionCriteria::equals("test", "exact match");
        let result = AssertionEvaluator::evaluate(&criteria, "exact match");
        assert!(result.passed);

        let result = AssertionEvaluator::evaluate(&criteria, "not exact");
        assert!(!result.passed);
    }

    #[test]
    fn test_assertion_length() {
        let gt = AssertionCriteria::length_greater_than("test", 5);
        assert!(AssertionEvaluator::evaluate(&gt, "hello world").passed);
        assert!(!AssertionEvaluator::evaluate(&gt, "hi").passed);

        let lt = AssertionCriteria::length_less_than("test", 10);
        assert!(AssertionEvaluator::evaluate(&lt, "short").passed);
        assert!(!AssertionEvaluator::evaluate(&lt, "this is long").passed);
    }

    #[test]
    fn test_assertion_json_path() {
        let exists = AssertionCriteria::json_path_exists("test", "$.name");
        let json = r#"{"name": "test", "value": 123}"#;
        assert!(AssertionEvaluator::evaluate(&exists, json).passed);

        let not_exists = AssertionCriteria::json_path_exists("test", "$.missing");
        assert!(!AssertionEvaluator::evaluate(&not_exists, json).passed);

        let equals = AssertionCriteria::json_path_equals("test", "$.name", "test");
        assert!(AssertionEvaluator::evaluate(&equals, json).passed);

        let wrong_value = AssertionCriteria::json_path_equals("test", "$.name", "wrong");
        assert!(!AssertionEvaluator::evaluate(&wrong_value, json).passed);
    }

    #[test]
    fn test_json_path_nested() {
        let json = r#"{"user": {"name": "Alice", "roles": ["admin", "user"]}}"#;

        let nested = AssertionCriteria::json_path_equals("test", "$.user.name", "Alice");
        assert!(AssertionEvaluator::evaluate(&nested, json).passed);

        let array = AssertionCriteria::json_path_equals("test", "$.user.roles[0]", "admin");
        assert!(AssertionEvaluator::evaluate(&array, json).passed);
    }

    #[test]
    fn test_test_case_result() {
        let test_case_id = TestCaseId::new("test-1").unwrap();

        let result = TestCaseResult::success(
            test_case_id.clone(),
            "Hello world".to_string(),
            vec![
                AssertionResult::passed("check1", AssertionOperator::Contains, "Hello"),
                AssertionResult::failed("check2", AssertionOperator::Contains, "Goodbye", None, "Not found"),
            ],
            100,
        );

        assert!(!result.passed()); // One assertion failed
        assert_eq!(result.passed_count(), 1);
        assert_eq!(result.failed_count(), 1);
    }

    #[test]
    fn test_test_case_result_all_passed() {
        let test_case_id = TestCaseId::new("test-2").unwrap();

        let result = TestCaseResult::success(
            test_case_id,
            "Success".to_string(),
            vec![
                AssertionResult::passed("check1", AssertionOperator::Contains, "Success"),
            ],
            50,
        );

        assert!(result.passed());
    }
}
