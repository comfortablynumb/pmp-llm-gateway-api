//! Operation API types for async operations

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::domain::operation::{Operation, OperationStatus, OperationType};

/// Query parameters for async operation support
#[derive(Debug, Clone, Default, Deserialize)]
pub struct AsyncQueryParams {
    /// Enable async mode - returns 202 Accepted with operation ID
    #[serde(default, rename = "async")]
    pub is_async: bool,
}

/// Response when an async operation is created (HTTP 202)
#[derive(Debug, Clone, Serialize)]
pub struct AsyncOperationCreated {
    pub operation_id: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl AsyncOperationCreated {
    /// Create response for a newly created pending operation
    pub fn pending(operation_id: impl Into<String>) -> Self {
        Self {
            operation_id: operation_id.into(),
            status: "pending".to_string(),
            message: Some("Operation queued for processing".to_string()),
        }
    }
}

/// Response for a single operation
#[derive(Debug, Clone, Serialize)]
pub struct OperationResponse {
    pub operation_id: String,
    pub operation_type: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<String>,
}

impl From<&Operation> for OperationResponse {
    fn from(op: &Operation) -> Self {
        Self {
            operation_id: op.id().to_string(),
            operation_type: format_operation_type(op.operation_type()),
            status: format_status(op.status()),
            result: op.result().cloned(),
            error: op.error().map(String::from),
            created_at: op.created_at().to_rfc3339(),
            started_at: op.started_at().map(|t| t.to_rfc3339()),
            completed_at: op.completed_at().map(|t| t.to_rfc3339()),
        }
    }
}

impl From<Operation> for OperationResponse {
    fn from(op: Operation) -> Self {
        OperationResponse::from(&op)
    }
}

/// Response for listing multiple operations
#[derive(Debug, Clone, Serialize)]
pub struct OperationsListResponse {
    pub operations: Vec<OperationResponse>,
}

impl OperationsListResponse {
    /// Create from a vector of operations
    pub fn from_operations(operations: Vec<Operation>) -> Self {
        Self {
            operations: operations.iter().map(OperationResponse::from).collect(),
        }
    }
}

/// Query parameters for listing operations
#[derive(Debug, Clone, Default, Deserialize)]
pub struct OperationsQueryParams {
    /// Comma-separated list of operation IDs to fetch
    pub ids: Option<String>,
}

impl OperationsQueryParams {
    /// Parse the comma-separated IDs into a vector
    pub fn parse_ids(&self) -> Vec<String> {
        self.ids
            .as_ref()
            .map(|s| {
                s.split(',')
                    .map(|id| id.trim().to_string())
                    .filter(|id| !id.is_empty())
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// Format operation type for API response
fn format_operation_type(op_type: OperationType) -> String {
    match op_type {
        OperationType::ChatCompletion => "chat_completion".to_string(),
        OperationType::WorkflowExecution => "workflow_execution".to_string(),
    }
}

/// Format status for API response
fn format_status(status: OperationStatus) -> String {
    match status {
        OperationStatus::Pending => "pending".to_string(),
        OperationStatus::Running => "running".to_string(),
        OperationStatus::Completed => "completed".to_string(),
        OperationStatus::Failed => "failed".to_string(),
        OperationStatus::Cancelled => "cancelled".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_async_query_params_default() {
        let params = AsyncQueryParams::default();
        assert!(!params.is_async);
    }

    #[test]
    fn test_async_query_params_deserialize() {
        let json = r#"{"async": true}"#;
        let params: AsyncQueryParams = serde_json::from_str(json).unwrap();
        assert!(params.is_async);
    }

    #[test]
    fn test_async_operation_created() {
        let response = AsyncOperationCreated::pending("op-123");
        assert_eq!(response.operation_id, "op-123");
        assert_eq!(response.status, "pending");
        assert!(response.message.is_some());
    }

    #[test]
    fn test_operations_query_params_parse_ids() {
        let params = OperationsQueryParams {
            ids: Some("op-1, op-2,op-3".to_string()),
        };
        let ids = params.parse_ids();
        assert_eq!(ids, vec!["op-1", "op-2", "op-3"]);
    }

    #[test]
    fn test_operations_query_params_empty() {
        let params = OperationsQueryParams { ids: None };
        let ids = params.parse_ids();
        assert!(ids.is_empty());
    }

    #[test]
    fn test_operation_response_serialization() {
        let op = crate::domain::operation::Operation::new(
            OperationType::ChatCompletion,
            json!({"model": "gpt-4"}),
            json!({}),
        );
        let response = OperationResponse::from(&op);

        let json = serde_json::to_value(&response).unwrap();
        assert!(json.get("operation_id").is_some());
        assert_eq!(json["operation_type"], "chat_completion");
        assert_eq!(json["status"], "pending");
        assert!(json.get("result").is_none()); // Not serialized if None
        assert!(json.get("error").is_none()); // Not serialized if None
    }

    #[test]
    fn test_operations_list_response() {
        let op1 = crate::domain::operation::Operation::new(
            OperationType::ChatCompletion,
            json!({}),
            json!({}),
        );
        let op2 = crate::domain::operation::Operation::new(
            OperationType::WorkflowExecution,
            json!({}),
            json!({}),
        );

        let response = OperationsListResponse::from_operations(vec![op1, op2]);
        assert_eq!(response.operations.len(), 2);
    }
}
