//! Test case domain - Test case definitions and execution results

mod entity;
mod repository;
mod result;
mod validation;

pub use entity::{
    AssertionCriteria, AssertionOperator, ModelPromptInput, TestCase, TestCaseId, TestCaseInput,
    TestCaseType, WorkflowInput,
};
pub use repository::{
    TestCaseQuery, TestCaseRepository, TestCaseResultQuery, TestCaseResultRepository,
};
pub use result::{AssertionEvaluator, AssertionResult, TestCaseResult, TestCaseResultId, TokenUsage};
pub use validation::{
    validate_assertion, validate_model_prompt_input, validate_test_case, validate_workflow_input,
    TestCaseValidationError,
};

#[cfg(test)]
pub use repository::mock::{MockTestCaseRepository, MockTestCaseResultRepository};
