//! Test case validation

use thiserror::Error;

/// Validation errors for test cases
#[derive(Debug, Clone, Error, PartialEq)]
pub enum TestCaseValidationError {
    #[error("Test case ID is required")]
    IdRequired,

    #[error("Test case name is required")]
    NameRequired,

    #[error("Test case name is too long (max 100 characters)")]
    NameTooLong,

    #[error("User message is required for model+prompt tests")]
    UserMessageRequired,

    #[error("Model ID is required for model+prompt tests")]
    ModelIdRequired,

    #[error("Workflow ID is required for workflow tests")]
    WorkflowIdRequired,

    #[error("At least one assertion is required")]
    NoAssertions,

    #[error("Invalid regex pattern: {0}")]
    InvalidRegex(String),

    #[error("Invalid JSON path: {0}")]
    InvalidJsonPath(String),

    #[error("Test case not found: {0}")]
    NotFound(String),

    #[error("Test case already exists: {0}")]
    AlreadyExists(String),
}

use super::{AssertionCriteria, AssertionOperator, ModelPromptInput, TestCase, TestCaseInput, WorkflowInput};
use regex::Regex;

/// Validate a test case
pub fn validate_test_case(test_case: &TestCase) -> Result<(), TestCaseValidationError> {
    // Validate name
    if test_case.name().is_empty() {
        return Err(TestCaseValidationError::NameRequired);
    }

    if test_case.name().len() > 100 {
        return Err(TestCaseValidationError::NameTooLong);
    }

    // Validate input based on type
    match test_case.input() {
        TestCaseInput::ModelPrompt(input) => validate_model_prompt_input(input)?,
        TestCaseInput::Workflow(input) => validate_workflow_input(input)?,
    }

    // Validate assertions
    for assertion in test_case.assertions() {
        validate_assertion(assertion)?;
    }

    Ok(())
}

/// Validate model+prompt input
pub fn validate_model_prompt_input(input: &ModelPromptInput) -> Result<(), TestCaseValidationError> {
    if input.model_id.is_empty() {
        return Err(TestCaseValidationError::ModelIdRequired);
    }

    if input.user_message.is_empty() {
        return Err(TestCaseValidationError::UserMessageRequired);
    }

    Ok(())
}

/// Validate workflow input
pub fn validate_workflow_input(input: &WorkflowInput) -> Result<(), TestCaseValidationError> {
    if input.workflow_id.is_empty() {
        return Err(TestCaseValidationError::WorkflowIdRequired);
    }

    Ok(())
}

/// Validate an assertion
pub fn validate_assertion(assertion: &AssertionCriteria) -> Result<(), TestCaseValidationError> {
    // Validate regex patterns
    if assertion.operator == AssertionOperator::Regex {
        if let Err(e) = Regex::new(&assertion.expected) {
            return Err(TestCaseValidationError::InvalidRegex(e.to_string()));
        }
    }

    // Validate JSON path for JSON assertions
    if matches!(
        assertion.operator,
        AssertionOperator::JsonPathExists | AssertionOperator::JsonPathEquals
    ) {
        if assertion.json_path.is_none() || assertion.json_path.as_ref().map(|p| p.is_empty()).unwrap_or(true) {
            return Err(TestCaseValidationError::InvalidJsonPath(
                "JSON path is required for JSON assertions".to_string(),
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::test_case::TestCaseId;
    use std::collections::HashMap;

    fn create_valid_model_input() -> ModelPromptInput {
        ModelPromptInput {
            model_id: "gpt-4".to_string(),
            prompt_id: None,
            variables: HashMap::new(),
            user_message: "Hello".to_string(),
            temperature: None,
            max_tokens: None,
        }
    }

    #[test]
    fn test_valid_test_case() {
        let test_case = TestCase::model_prompt(
            TestCaseId::new("test-1").unwrap(),
            "Valid Test",
            create_valid_model_input(),
        );

        assert!(validate_test_case(&test_case).is_ok());
    }

    #[test]
    fn test_empty_name() {
        let test_case = TestCase::model_prompt(
            TestCaseId::new("test-1").unwrap(),
            "",
            create_valid_model_input(),
        );

        assert_eq!(
            validate_test_case(&test_case),
            Err(TestCaseValidationError::NameRequired)
        );
    }

    #[test]
    fn test_empty_model_id() {
        let input = ModelPromptInput {
            model_id: "".to_string(),
            prompt_id: None,
            variables: HashMap::new(),
            user_message: "Hello".to_string(),
            temperature: None,
            max_tokens: None,
        };

        assert_eq!(
            validate_model_prompt_input(&input),
            Err(TestCaseValidationError::ModelIdRequired)
        );
    }

    #[test]
    fn test_empty_user_message() {
        let input = ModelPromptInput {
            model_id: "gpt-4".to_string(),
            prompt_id: None,
            variables: HashMap::new(),
            user_message: "".to_string(),
            temperature: None,
            max_tokens: None,
        };

        assert_eq!(
            validate_model_prompt_input(&input),
            Err(TestCaseValidationError::UserMessageRequired)
        );
    }

    #[test]
    fn test_invalid_regex() {
        let assertion = AssertionCriteria::regex("test", "[invalid");

        assert!(matches!(
            validate_assertion(&assertion),
            Err(TestCaseValidationError::InvalidRegex(_))
        ));
    }

    #[test]
    fn test_json_path_required() {
        let assertion = AssertionCriteria {
            name: "test".to_string(),
            operator: AssertionOperator::JsonPathExists,
            expected: "".to_string(),
            json_path: None,
        };

        assert!(matches!(
            validate_assertion(&assertion),
            Err(TestCaseValidationError::InvalidJsonPath(_))
        ));
    }
}
