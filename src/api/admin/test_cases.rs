//! Test case management admin endpoints

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use crate::api::middleware::RequireAdmin;
use crate::api::state::AppState;
use crate::api::types::ApiError;
use crate::domain::test_case::{
    AssertionCriteria, AssertionOperator, ModelPromptInput, TestCase, TestCaseInput, TestCaseQuery,
    TestCaseResultQuery, TestCaseType, WorkflowInput,
};
use crate::infrastructure::services::{
    CreateTestCaseRequest, TestCaseInputRequest, UpdateTestCaseRequest,
};

/// Query parameters for listing test cases
#[derive(Debug, Default, Deserialize)]
pub struct ListTestCasesQuery {
    pub test_type: Option<String>,
    pub enabled: Option<bool>,
    pub tag: Option<String>,
    pub model_id: Option<String>,
    pub workflow_id: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// Request to create a new test case
#[derive(Debug, Clone, Deserialize)]
pub struct CreateTestCaseApiRequest {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub input: TestCaseInputApiRequest,
    #[serde(default)]
    pub assertions: Vec<AssertionApiRequest>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_enabled() -> bool {
    true
}

/// Test case input configuration in API request
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TestCaseInputApiRequest {
    ModelPrompt(ModelPromptInputApiRequest),
    Workflow(WorkflowInputApiRequest),
}

/// Model+prompt input in API request
#[derive(Debug, Clone, Deserialize)]
pub struct ModelPromptInputApiRequest {
    pub model_id: String,
    #[serde(default)]
    pub prompt_id: Option<String>,
    #[serde(default)]
    pub variables: std::collections::HashMap<String, String>,
    pub user_message: String,
    #[serde(default)]
    pub temperature: Option<f32>,
    #[serde(default)]
    pub max_tokens: Option<u32>,
}

/// Workflow input in API request
#[derive(Debug, Clone, Deserialize)]
pub struct WorkflowInputApiRequest {
    pub workflow_id: String,
    #[serde(default)]
    pub input: serde_json::Value,
}

/// Assertion in API request
#[derive(Debug, Clone, Deserialize)]
pub struct AssertionApiRequest {
    pub name: String,
    pub operator: String,
    pub expected: String,
    #[serde(default)]
    pub json_path: Option<String>,
}

/// Request to update a test case
#[derive(Debug, Clone, Default, Deserialize)]
pub struct UpdateTestCaseApiRequest {
    pub name: Option<String>,
    pub description: Option<Option<String>>,
    pub input: Option<TestCaseInputApiRequest>,
    pub assertions: Option<Vec<AssertionApiRequest>>,
    pub tags: Option<Vec<String>>,
    pub enabled: Option<bool>,
}

/// Test case response for admin API
#[derive(Debug, Clone, Serialize)]
pub struct TestCaseResponse {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub test_type: String,
    pub input: TestCaseInputResponse,
    pub assertions: Vec<AssertionResponse>,
    pub tags: Vec<String>,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// Test case input in response
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TestCaseInputResponse {
    ModelPrompt(ModelPromptInputResponse),
    Workflow(WorkflowInputResponse),
}

/// Model+prompt input in response
#[derive(Debug, Clone, Serialize)]
pub struct ModelPromptInputResponse {
    pub model_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_id: Option<String>,
    #[serde(skip_serializing_if = "std::collections::HashMap::is_empty")]
    pub variables: std::collections::HashMap<String, String>,
    pub user_message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
}

/// Workflow input in response
#[derive(Debug, Clone, Serialize)]
pub struct WorkflowInputResponse {
    pub workflow_id: String,
    pub input: serde_json::Value,
}

/// Assertion in response
#[derive(Debug, Clone, Serialize)]
pub struct AssertionResponse {
    pub name: String,
    pub operator: String,
    pub expected: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub json_path: Option<String>,
}

/// List test cases response
#[derive(Debug, Clone, Serialize)]
pub struct ListTestCasesResponse {
    pub test_cases: Vec<TestCaseResponse>,
    pub total: usize,
}

/// List test case results response
#[derive(Debug, Clone, Serialize)]
pub struct ListTestCaseResultsResponse {
    pub results: Vec<TestCaseResultResponse>,
    pub total: usize,
}

/// Test case execution result response
#[derive(Debug, Clone, Serialize)]
pub struct ExecuteTestCaseApiResponse {
    pub test_case_id: String,
    pub test_case_name: String,
    pub passed: bool,
    pub output: Option<String>,
    pub assertion_results: Vec<AssertionResultApiResponse>,
    pub execution_time_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens_used: Option<TokenUsageResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Assertion result in API response
#[derive(Debug, Clone, Serialize)]
pub struct AssertionResultApiResponse {
    pub name: String,
    pub passed: bool,
    pub operator: String,
    pub expected: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actual: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Token usage in API response
#[derive(Debug, Clone, Serialize)]
pub struct TokenUsageResponse {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// GET /admin/test-cases
pub async fn list_test_cases(
    State(state): State<AppState>,
    RequireAdmin(_): RequireAdmin,
    Query(params): Query<ListTestCasesQuery>,
) -> Result<Json<ListTestCasesResponse>, ApiError> {
    debug!("Listing test cases");

    let mut query = TestCaseQuery::new();

    if let Some(ref test_type) = params.test_type {
        let tt = parse_test_type(test_type)?;
        query = query.with_test_type(tt);
    }

    if let Some(enabled) = params.enabled {
        query = query.with_enabled(enabled);
    }

    if let Some(ref tag) = params.tag {
        query = query.with_tag(tag);
    }

    if let Some(ref model_id) = params.model_id {
        query = query.with_model_id(model_id);
    }

    if let Some(ref workflow_id) = params.workflow_id {
        query = query.with_workflow_id(workflow_id);
    }

    if let Some(limit) = params.limit {
        query = query.with_limit(limit);
    }

    if let Some(offset) = params.offset {
        query = query.with_offset(offset);
    }

    let total = state.test_case_service.count(&query).await?;
    let test_cases = state.test_case_service.list(&query).await?;
    let responses: Vec<TestCaseResponse> = test_cases.into_iter().map(to_response).collect();

    Ok(Json(ListTestCasesResponse {
        test_cases: responses,
        total,
    }))
}

/// GET /admin/test-cases/:id
pub async fn get_test_case(
    State(state): State<AppState>,
    RequireAdmin(_): RequireAdmin,
    Path(id): Path<String>,
) -> Result<Json<TestCaseResponse>, ApiError> {
    debug!(test_case_id = %id, "Getting test case");

    let test_case = state
        .test_case_service
        .get(&id)
        .await?
        .ok_or_else(|| ApiError::not_found(format!("Test case '{}' not found", id)))?;

    Ok(Json(to_response(test_case)))
}

/// POST /admin/test-cases
pub async fn create_test_case(
    State(state): State<AppState>,
    RequireAdmin(_): RequireAdmin,
    Json(request): Json<CreateTestCaseApiRequest>,
) -> Result<Json<TestCaseResponse>, ApiError> {
    info!(test_case_id = %request.id, "Creating test case");

    let service_request = CreateTestCaseRequest {
        id: request.id,
        name: request.name,
        description: request.description,
        input: convert_input_request(request.input)?,
        assertions: convert_assertions(request.assertions)?,
        tags: request.tags,
        enabled: request.enabled,
    };

    let test_case = state.test_case_service.create(service_request).await?;

    Ok(Json(to_response(test_case)))
}

/// PUT /admin/test-cases/:id
pub async fn update_test_case(
    State(state): State<AppState>,
    RequireAdmin(_): RequireAdmin,
    Path(id): Path<String>,
    Json(request): Json<UpdateTestCaseApiRequest>,
) -> Result<Json<TestCaseResponse>, ApiError> {
    info!(test_case_id = %id, "Updating test case");

    let input = if let Some(i) = request.input {
        Some(convert_input_request(i)?)
    } else {
        None
    };

    let assertions = if let Some(a) = request.assertions {
        Some(convert_assertions(a)?)
    } else {
        None
    };

    let service_request = UpdateTestCaseRequest {
        name: request.name,
        description: request.description,
        input,
        assertions,
        tags: request.tags,
        enabled: request.enabled,
    };

    let test_case = state.test_case_service.update(&id, service_request).await?;

    Ok(Json(to_response(test_case)))
}

/// DELETE /admin/test-cases/:id
pub async fn delete_test_case(
    State(state): State<AppState>,
    RequireAdmin(_): RequireAdmin,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    info!(test_case_id = %id, "Deleting test case");

    let deleted = state.test_case_service.delete(&id).await?;

    if deleted {
        Ok(Json(serde_json::json!({"deleted": true})))
    } else {
        Err(ApiError::not_found(format!(
            "Test case '{}' not found",
            id
        )))
    }
}

/// POST /admin/test-cases/:id/execute
pub async fn execute_test_case(
    State(state): State<AppState>,
    RequireAdmin(_): RequireAdmin,
    Path(id): Path<String>,
) -> Result<Json<ExecuteTestCaseApiResponse>, ApiError> {
    info!(test_case_id = %id, "Executing test case");

    let result = state.test_case_service.execute(&id).await?;

    let response = ExecuteTestCaseApiResponse {
        test_case_id: result.test_case_id,
        test_case_name: result.test_case_name,
        passed: result.passed,
        output: result.output,
        assertion_results: result
            .assertion_results
            .into_iter()
            .map(|r| AssertionResultApiResponse {
                name: r.name,
                passed: r.passed,
                operator: r.operator,
                expected: r.expected,
                actual: r.actual,
                error: r.error,
            })
            .collect(),
        execution_time_ms: result.execution_time_ms,
        tokens_used: result.tokens_used.map(|t| TokenUsageResponse {
            prompt_tokens: t.prompt_tokens,
            completion_tokens: t.completion_tokens,
            total_tokens: t.total_tokens,
        }),
        error: result.error,
    };

    Ok(Json(response))
}

/// GET /admin/test-cases/:id/results
pub async fn get_test_case_results(
    State(state): State<AppState>,
    RequireAdmin(_): RequireAdmin,
    Path(id): Path<String>,
    Query(params): Query<ListResultsQuery>,
) -> Result<Json<ListTestCaseResultsResponse>, ApiError> {
    debug!(test_case_id = %id, "Getting test case results");

    let test_case_id = crate::domain::test_case::TestCaseId::new(&id)
        .map_err(|e| ApiError::bad_request(e.to_string()))?;

    let mut query = TestCaseResultQuery::for_test_case(test_case_id);

    if let Some(passed) = params.passed {
        query = query.with_passed(passed);
    }

    if let Some(limit) = params.limit {
        query = query.with_limit(limit);
    }

    let results = state.test_case_service.get_results(&id, &query).await?;
    let total = results.len();

    let responses: Vec<TestCaseResultResponse> = results
        .into_iter()
        .map(|r| TestCaseResultResponse {
            id: r.id().to_string(),
            test_case_id: r.test_case_id().to_string(),
            passed: r.passed(),
            output: r.output().map(|s| s.to_string()),
            assertion_results: r
                .assertion_results()
                .iter()
                .map(|ar| AssertionResultApiResponse {
                    name: ar.name.clone(),
                    passed: ar.passed,
                    operator: format!("{}", ar.operator),
                    expected: ar.expected.clone(),
                    actual: ar.actual.clone(),
                    error: ar.error.clone(),
                })
                .collect(),
            execution_time_ms: r.execution_time_ms(),
            error: r.error().map(|s| s.to_string()),
            executed_at: r.executed_at().to_rfc3339(),
        })
        .collect();

    Ok(Json(ListTestCaseResultsResponse {
        results: responses,
        total,
    }))
}

/// Query parameters for listing results
#[derive(Debug, Default, Deserialize)]
pub struct ListResultsQuery {
    pub passed: Option<bool>,
    pub limit: Option<usize>,
}

/// Test case result response
#[derive(Debug, Clone, Serialize)]
pub struct TestCaseResultResponse {
    pub id: String,
    pub test_case_id: String,
    pub passed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    pub assertion_results: Vec<AssertionResultApiResponse>,
    pub execution_time_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub executed_at: String,
}

// Helper functions

fn parse_test_type(s: &str) -> Result<TestCaseType, ApiError> {
    match s.to_lowercase().as_str() {
        "model_prompt" => Ok(TestCaseType::ModelPrompt),
        "workflow" => Ok(TestCaseType::Workflow),
        _ => Err(ApiError::bad_request(format!(
            "Invalid test type: {}. Expected 'model_prompt' or 'workflow'",
            s
        ))),
    }
}

fn parse_assertion_operator(s: &str) -> Result<AssertionOperator, ApiError> {
    match s.to_lowercase().as_str() {
        "contains" => Ok(AssertionOperator::Contains),
        "not_contains" => Ok(AssertionOperator::NotContains),
        "regex" => Ok(AssertionOperator::Regex),
        "equals" => Ok(AssertionOperator::Equals),
        "not_equals" => Ok(AssertionOperator::NotEquals),
        "json_path_exists" => Ok(AssertionOperator::JsonPathExists),
        "json_path_equals" => Ok(AssertionOperator::JsonPathEquals),
        "length_greater_than" => Ok(AssertionOperator::LengthGreaterThan),
        "length_less_than" => Ok(AssertionOperator::LengthLessThan),
        _ => Err(ApiError::bad_request(format!(
            "Invalid assertion operator: {}",
            s
        ))),
    }
}

fn convert_input_request(input: TestCaseInputApiRequest) -> Result<TestCaseInputRequest, ApiError> {
    match input {
        TestCaseInputApiRequest::ModelPrompt(mp) => {
            Ok(TestCaseInputRequest::ModelPrompt(ModelPromptInput {
                model_id: mp.model_id,
                prompt_id: mp.prompt_id,
                variables: mp.variables,
                user_message: mp.user_message,
                temperature: mp.temperature,
                max_tokens: mp.max_tokens,
            }))
        }
        TestCaseInputApiRequest::Workflow(wf) => {
            Ok(TestCaseInputRequest::Workflow(WorkflowInput {
                workflow_id: wf.workflow_id,
                input: wf.input,
            }))
        }
    }
}

fn convert_assertions(assertions: Vec<AssertionApiRequest>) -> Result<Vec<AssertionCriteria>, ApiError> {
    assertions
        .into_iter()
        .map(|a| {
            let operator = parse_assertion_operator(&a.operator)?;
            Ok(AssertionCriteria {
                name: a.name,
                operator,
                expected: a.expected,
                json_path: a.json_path,
            })
        })
        .collect()
}

fn to_response(test_case: TestCase) -> TestCaseResponse {
    let input = match test_case.input() {
        TestCaseInput::ModelPrompt(mp) => TestCaseInputResponse::ModelPrompt(ModelPromptInputResponse {
            model_id: mp.model_id.clone(),
            prompt_id: mp.prompt_id.clone(),
            variables: mp.variables.clone(),
            user_message: mp.user_message.clone(),
            temperature: mp.temperature,
            max_tokens: mp.max_tokens,
        }),
        TestCaseInput::Workflow(wf) => TestCaseInputResponse::Workflow(WorkflowInputResponse {
            workflow_id: wf.workflow_id.clone(),
            input: wf.input.clone(),
        }),
    };

    let assertions: Vec<AssertionResponse> = test_case
        .assertions()
        .iter()
        .map(|a| AssertionResponse {
            name: a.name.clone(),
            operator: format!("{}", a.operator),
            expected: a.expected.clone(),
            json_path: a.json_path.clone(),
        })
        .collect();

    TestCaseResponse {
        id: test_case.id().to_string(),
        name: test_case.name().to_string(),
        description: test_case.description().map(|s| s.to_string()),
        test_type: format!("{}", test_case.test_type()),
        input,
        assertions,
        tags: test_case.tags().to_vec(),
        enabled: test_case.is_enabled(),
        created_at: test_case.created_at().to_rfc3339(),
        updated_at: test_case.updated_at().to_rfc3339(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_test_type_model_prompt() {
        let result = parse_test_type("model_prompt");
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), TestCaseType::ModelPrompt));
    }

    #[test]
    fn test_parse_test_type_workflow() {
        let result = parse_test_type("workflow");
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), TestCaseType::Workflow));
    }

    #[test]
    fn test_parse_test_type_case_insensitive() {
        assert!(parse_test_type("MODEL_PROMPT").is_ok());
        assert!(parse_test_type("Workflow").is_ok());
    }

    #[test]
    fn test_parse_test_type_invalid() {
        let result = parse_test_type("invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_assertion_operator_contains() {
        let result = parse_assertion_operator("contains");
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), AssertionOperator::Contains));
    }

    #[test]
    fn test_parse_assertion_operator_not_contains() {
        let result = parse_assertion_operator("not_contains");
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), AssertionOperator::NotContains));
    }

    #[test]
    fn test_parse_assertion_operator_regex() {
        let result = parse_assertion_operator("regex");
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), AssertionOperator::Regex));
    }

    #[test]
    fn test_parse_assertion_operator_equals() {
        let result = parse_assertion_operator("equals");
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), AssertionOperator::Equals));
    }

    #[test]
    fn test_parse_assertion_operator_json_path_exists() {
        let result = parse_assertion_operator("json_path_exists");
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), AssertionOperator::JsonPathExists));
    }

    #[test]
    fn test_parse_assertion_operator_json_path_equals() {
        let result = parse_assertion_operator("json_path_equals");
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), AssertionOperator::JsonPathEquals));
    }

    #[test]
    fn test_parse_assertion_operator_length_greater_than() {
        let result = parse_assertion_operator("length_greater_than");
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), AssertionOperator::LengthGreaterThan));
    }

    #[test]
    fn test_parse_assertion_operator_length_less_than() {
        let result = parse_assertion_operator("length_less_than");
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), AssertionOperator::LengthLessThan));
    }

    #[test]
    fn test_parse_assertion_operator_invalid() {
        let result = parse_assertion_operator("invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_default_enabled() {
        assert!(default_enabled());
    }

    #[test]
    fn test_list_test_cases_query_deserialization() {
        let json = r#"{
            "test_type": "model_prompt",
            "enabled": true,
            "tag": "regression",
            "model_id": "gpt-4",
            "limit": 50,
            "offset": 10
        }"#;

        let query: ListTestCasesQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.test_type, Some("model_prompt".to_string()));
        assert_eq!(query.enabled, Some(true));
        assert_eq!(query.tag, Some("regression".to_string()));
        assert_eq!(query.model_id, Some("gpt-4".to_string()));
        assert_eq!(query.limit, Some(50));
        assert_eq!(query.offset, Some(10));
    }

    #[test]
    fn test_list_test_cases_query_minimal() {
        let json = r#"{}"#;
        let query: ListTestCasesQuery = serde_json::from_str(json).unwrap();
        assert!(query.test_type.is_none());
        assert!(query.enabled.is_none());
    }

    #[test]
    fn test_list_results_query_deserialization() {
        let json = r#"{"passed": true, "limit": 100}"#;
        let query: ListResultsQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.passed, Some(true));
        assert_eq!(query.limit, Some(100));
    }

    #[test]
    fn test_assertion_api_request_deserialization() {
        let json = r#"{
            "name": "output-contains-hello",
            "operator": "contains",
            "expected": "hello",
            "json_path": "$.message"
        }"#;

        let assertion: AssertionApiRequest = serde_json::from_str(json).unwrap();
        assert_eq!(assertion.name, "output-contains-hello");
        assert_eq!(assertion.operator, "contains");
        assert_eq!(assertion.expected, "hello");
        assert_eq!(assertion.json_path, Some("$.message".to_string()));
    }

    #[test]
    fn test_model_prompt_input_api_request() {
        let json = r#"{
            "model_id": "gpt-4",
            "prompt_id": "greeting",
            "variables": {"name": "World"},
            "user_message": "Hello!",
            "temperature": 0.7,
            "max_tokens": 100
        }"#;

        let input: ModelPromptInputApiRequest = serde_json::from_str(json).unwrap();
        assert_eq!(input.model_id, "gpt-4");
        assert_eq!(input.prompt_id, Some("greeting".to_string()));
        assert_eq!(input.user_message, "Hello!");
        assert_eq!(input.temperature, Some(0.7));
        assert_eq!(input.max_tokens, Some(100));
    }

    #[test]
    fn test_workflow_input_api_request() {
        let json = r#"{
            "workflow_id": "my-workflow",
            "input": {"key": "value"}
        }"#;

        let input: WorkflowInputApiRequest = serde_json::from_str(json).unwrap();
        assert_eq!(input.workflow_id, "my-workflow");
        assert!(input.input.is_object());
    }

    #[test]
    fn test_test_case_input_api_request_model_prompt() {
        let json = r#"{
            "type": "model_prompt",
            "model_id": "gpt-4",
            "user_message": "Test"
        }"#;

        let input: TestCaseInputApiRequest = serde_json::from_str(json).unwrap();
        assert!(matches!(input, TestCaseInputApiRequest::ModelPrompt(_)));
    }

    #[test]
    fn test_test_case_input_api_request_workflow() {
        let json = r#"{
            "type": "workflow",
            "workflow_id": "test-workflow",
            "input": {}
        }"#;

        let input: TestCaseInputApiRequest = serde_json::from_str(json).unwrap();
        assert!(matches!(input, TestCaseInputApiRequest::Workflow(_)));
    }

    #[test]
    fn test_token_usage_response_serialization() {
        let response = TokenUsageResponse {
            prompt_tokens: 100,
            completion_tokens: 50,
            total_tokens: 150,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"prompt_tokens\":100"));
        assert!(json.contains("\"completion_tokens\":50"));
        assert!(json.contains("\"total_tokens\":150"));
    }

    #[test]
    fn test_assertion_response_serialization() {
        let response = AssertionResponse {
            name: "check-output".to_string(),
            operator: "contains".to_string(),
            expected: "hello".to_string(),
            json_path: Some("$.text".to_string()),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"name\":\"check-output\""));
        assert!(json.contains("\"operator\":\"contains\""));
        assert!(json.contains("\"expected\":\"hello\""));
        assert!(json.contains("\"json_path\":\"$.text\""));
    }

    #[test]
    fn test_assertion_response_without_json_path() {
        let response = AssertionResponse {
            name: "check".to_string(),
            operator: "equals".to_string(),
            expected: "test".to_string(),
            json_path: None,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(!json.contains("json_path"));
    }

    #[test]
    fn test_assertion_result_api_response_serialization() {
        let response = AssertionResultApiResponse {
            name: "check-output".to_string(),
            passed: true,
            operator: "contains".to_string(),
            expected: "hello".to_string(),
            actual: Some("hello world".to_string()),
            error: None,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"name\":\"check-output\""));
        assert!(json.contains("\"passed\":true"));
        assert!(json.contains("\"actual\":\"hello world\""));
    }

    #[test]
    fn test_assertion_result_api_response_with_error() {
        let response = AssertionResultApiResponse {
            name: "check".to_string(),
            passed: false,
            operator: "regex".to_string(),
            expected: "[invalid".to_string(),
            actual: None,
            error: Some("Invalid regex".to_string()),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"passed\":false"));
        assert!(json.contains("\"error\":\"Invalid regex\""));
    }

    #[test]
    fn test_model_prompt_input_response_serialization() {
        let response = ModelPromptInputResponse {
            model_id: "gpt-4".to_string(),
            prompt_id: Some("greeting".to_string()),
            variables: {
                let mut m = std::collections::HashMap::new();
                m.insert("name".to_string(), "World".to_string());
                m
            },
            user_message: "Hello!".to_string(),
            temperature: Some(0.7),
            max_tokens: Some(100),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"model_id\":\"gpt-4\""));
        assert!(json.contains("\"prompt_id\":\"greeting\""));
        assert!(json.contains("\"user_message\":\"Hello!\""));
    }

    #[test]
    fn test_workflow_input_response_serialization() {
        let response = WorkflowInputResponse {
            workflow_id: "my-workflow".to_string(),
            input: serde_json::json!({"key": "value"}),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"workflow_id\":\"my-workflow\""));
    }

    #[test]
    fn test_execute_test_case_api_response_passed() {
        let response = ExecuteTestCaseApiResponse {
            test_case_id: "tc-001".to_string(),
            test_case_name: "Test Case 1".to_string(),
            passed: true,
            output: Some("Hello World".to_string()),
            assertion_results: vec![],
            execution_time_ms: 150,
            tokens_used: Some(TokenUsageResponse {
                prompt_tokens: 50,
                completion_tokens: 25,
                total_tokens: 75,
            }),
            error: None,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"test_case_id\":\"tc-001\""));
        assert!(json.contains("\"passed\":true"));
        assert!(json.contains("\"execution_time_ms\":150"));
    }

    #[test]
    fn test_execute_test_case_api_response_failed() {
        let response = ExecuteTestCaseApiResponse {
            test_case_id: "tc-002".to_string(),
            test_case_name: "Test Case 2".to_string(),
            passed: false,
            output: None,
            assertion_results: vec![],
            execution_time_ms: 50,
            tokens_used: None,
            error: Some("Model error".to_string()),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"passed\":false"));
        assert!(json.contains("\"error\":\"Model error\""));
    }

    #[test]
    fn test_list_test_cases_response_serialization() {
        let response = ListTestCasesResponse {
            test_cases: vec![],
            total: 42,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"test_cases\":[]"));
        assert!(json.contains("\"total\":42"));
    }

    #[test]
    fn test_list_test_case_results_response_serialization() {
        let response = ListTestCaseResultsResponse {
            results: vec![],
            total: 10,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"results\":[]"));
        assert!(json.contains("\"total\":10"));
    }

    #[test]
    fn test_test_case_result_response_serialization() {
        let response = TestCaseResultResponse {
            id: "result-001".to_string(),
            test_case_id: "tc-001".to_string(),
            passed: true,
            output: Some("Output text".to_string()),
            assertion_results: vec![],
            execution_time_ms: 200,
            error: None,
            executed_at: "2024-01-01T00:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"id\":\"result-001\""));
        assert!(json.contains("\"test_case_id\":\"tc-001\""));
        assert!(json.contains("\"passed\":true"));
        assert!(json.contains("\"executed_at\":\"2024-01-01T00:00:00Z\""));
    }

    #[test]
    fn test_convert_input_request_model_prompt() {
        let input = TestCaseInputApiRequest::ModelPrompt(ModelPromptInputApiRequest {
            model_id: "gpt-4".to_string(),
            prompt_id: Some("test".to_string()),
            variables: std::collections::HashMap::new(),
            user_message: "Hello".to_string(),
            temperature: Some(0.5),
            max_tokens: Some(100),
        });

        let result = convert_input_request(input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_convert_input_request_workflow() {
        let input = TestCaseInputApiRequest::Workflow(WorkflowInputApiRequest {
            workflow_id: "wf-1".to_string(),
            input: serde_json::json!({}),
        });

        let result = convert_input_request(input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_convert_assertions_success() {
        let assertions = vec![
            AssertionApiRequest {
                name: "check".to_string(),
                operator: "contains".to_string(),
                expected: "hello".to_string(),
                json_path: None,
            },
        ];

        let result = convert_assertions(assertions);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 1);
    }

    #[test]
    fn test_convert_assertions_invalid_operator() {
        let assertions = vec![
            AssertionApiRequest {
                name: "check".to_string(),
                operator: "invalid_op".to_string(),
                expected: "test".to_string(),
                json_path: None,
            },
        ];

        let result = convert_assertions(assertions);
        assert!(result.is_err());
    }

    #[test]
    fn test_create_test_case_api_request_deserialization() {
        let json = r#"{
            "id": "tc-001",
            "name": "Test Case 1",
            "description": "A test case",
            "input": {
                "type": "model_prompt",
                "model_id": "gpt-4",
                "user_message": "Hello"
            },
            "assertions": [{
                "name": "check",
                "operator": "contains",
                "expected": "world"
            }],
            "tags": ["regression"],
            "enabled": true
        }"#;

        let request: CreateTestCaseApiRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.id, "tc-001");
        assert_eq!(request.name, "Test Case 1");
        assert_eq!(request.description, Some("A test case".to_string()));
        assert_eq!(request.tags, vec!["regression"]);
        assert!(request.enabled);
    }

    #[test]
    fn test_update_test_case_api_request_deserialization() {
        let json = r#"{
            "name": "Updated Name",
            "enabled": false
        }"#;

        let request: UpdateTestCaseApiRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.name, Some("Updated Name".to_string()));
        assert_eq!(request.enabled, Some(false));
        assert!(request.description.is_none());
        assert!(request.input.is_none());
    }
}
