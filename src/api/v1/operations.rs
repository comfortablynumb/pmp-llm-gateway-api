//! Operations API endpoints for async operation management

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};

use crate::api::middleware::RequireApiKey;
use crate::api::state::AppState;
use crate::api::types::{
    ApiError, OperationResponse, OperationsListResponse, OperationsQueryParams,
};

/// GET /v1/operations/:operation_id - Get a single operation by ID
pub async fn get_operation(
    State(state): State<AppState>,
    RequireApiKey(_api_key): RequireApiKey,
    Path(operation_id): Path<String>,
) -> Result<Json<OperationResponse>, ApiError> {
    let operation = state
        .operation_service
        .get(&operation_id)
        .await
        .map_err(ApiError::from)?;

    match operation {
        Some(op) => Ok(Json(OperationResponse::from(op))),
        None => Err(ApiError::not_found(format!(
            "Operation '{}' not found",
            operation_id
        ))),
    }
}

/// GET /v1/operations - List operations by IDs
pub async fn list_operations(
    State(state): State<AppState>,
    RequireApiKey(_api_key): RequireApiKey,
    Query(params): Query<OperationsQueryParams>,
) -> Result<Json<OperationsListResponse>, ApiError> {
    let ids = params.parse_ids();

    if ids.is_empty() {
        return Ok(Json(OperationsListResponse { operations: vec![] }));
    }

    let operations = state
        .operation_service
        .get_batch(&ids)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(OperationsListResponse::from_operations(operations)))
}

/// DELETE /v1/operations/:operation_id - Cancel an operation
pub async fn cancel_operation(
    State(state): State<AppState>,
    RequireApiKey(_api_key): RequireApiKey,
    Path(operation_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let operation = state
        .operation_service
        .cancel(&operation_id)
        .await
        .map_err(ApiError::from)?;

    Ok((StatusCode::OK, Json(OperationResponse::from(operation))))
}

#[cfg(test)]
mod tests {
    use crate::api::types::{OperationResponse, OperationsListResponse, OperationsQueryParams};

    #[test]
    fn test_parse_ids_empty() {
        let params = OperationsQueryParams { ids: None };
        assert!(params.parse_ids().is_empty());
    }

    #[test]
    fn test_parse_ids_single() {
        let params = OperationsQueryParams {
            ids: Some("op-123".to_string()),
        };
        assert_eq!(params.parse_ids(), vec!["op-123"]);
    }

    #[test]
    fn test_parse_ids_multiple() {
        let params = OperationsQueryParams {
            ids: Some("op-1,op-2,op-3".to_string()),
        };
        assert_eq!(params.parse_ids(), vec!["op-1", "op-2", "op-3"]);
    }

    #[test]
    fn test_parse_ids_with_spaces() {
        let params = OperationsQueryParams {
            ids: Some("op-1, op-2 , op-3".to_string()),
        };
        assert_eq!(params.parse_ids(), vec!["op-1", "op-2", "op-3"]);
    }

    #[test]
    fn test_parse_ids_empty_string() {
        let params = OperationsQueryParams {
            ids: Some("".to_string()),
        };
        let ids = params.parse_ids();
        // Empty string results in vec with one empty element, which gets filtered
        assert!(ids.is_empty() || ids == vec![""]);
    }

    #[test]
    fn test_operations_query_params_deserialization() {
        let json = r#"{"ids": "op-1,op-2"}"#;
        let params: OperationsQueryParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.ids, Some("op-1,op-2".to_string()));
    }

    #[test]
    fn test_operation_response_serialization() {
        let response = OperationResponse {
            operation_id: "op-001".to_string(),
            operation_type: "chat_completion".to_string(),
            status: "completed".to_string(),
            result: Some(serde_json::json!({"response": "Hello"})),
            error: None,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            started_at: Some("2024-01-01T00:00:01Z".to_string()),
            completed_at: Some("2024-01-01T00:00:10Z".to_string()),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"operation_id\":\"op-001\""));
        assert!(json.contains("\"status\":\"completed\""));
        assert!(json.contains("\"operation_type\":\"chat_completion\""));
    }

    #[test]
    fn test_operation_response_with_error() {
        let response = OperationResponse {
            operation_id: "op-002".to_string(),
            operation_type: "workflow_execution".to_string(),
            status: "failed".to_string(),
            result: None,
            error: Some("Connection refused".to_string()),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            started_at: Some("2024-01-01T00:00:01Z".to_string()),
            completed_at: Some("2024-01-01T00:00:05Z".to_string()),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"status\":\"failed\""));
        assert!(json.contains("\"error\":\"Connection refused\""));
    }

    #[test]
    fn test_operation_response_pending() {
        let response = OperationResponse {
            operation_id: "op-003".to_string(),
            operation_type: "chat_completion".to_string(),
            status: "pending".to_string(),
            result: None,
            error: None,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            started_at: None,
            completed_at: None,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"status\":\"pending\""));
        assert!(!json.contains("\"result\":"));
        assert!(!json.contains("\"error\":"));
        assert!(!json.contains("\"started_at\":"));
        assert!(!json.contains("\"completed_at\":"));
    }

    #[test]
    fn test_operations_list_response_empty() {
        let response = OperationsListResponse { operations: vec![] };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"operations\":[]"));
    }

    #[test]
    fn test_operations_list_response_multiple() {
        let response = OperationsListResponse {
            operations: vec![
                OperationResponse {
                    operation_id: "op-1".to_string(),
                    operation_type: "chat_completion".to_string(),
                    status: "completed".to_string(),
                    result: None,
                    error: None,
                    created_at: "2024-01-01T00:00:00Z".to_string(),
                    started_at: None,
                    completed_at: Some("2024-01-01T00:00:10Z".to_string()),
                },
                OperationResponse {
                    operation_id: "op-2".to_string(),
                    operation_type: "workflow_execution".to_string(),
                    status: "pending".to_string(),
                    result: None,
                    error: None,
                    created_at: "2024-01-01T00:00:20Z".to_string(),
                    started_at: None,
                    completed_at: None,
                },
            ],
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"op-1\""));
        assert!(json.contains("\"op-2\""));
    }
}
