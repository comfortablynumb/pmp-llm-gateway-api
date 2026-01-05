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
    use super::*;
    use crate::api::types::OperationsQueryParams;

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
}
