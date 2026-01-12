//! Execution log management admin endpoints

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::api::middleware::RequireAdmin;
use crate::api::state::AppState;
use crate::api::types::ApiError;
use crate::domain::{ExecutionLogQuery, ExecutionStatus, ExecutionType};

/// Execution log response
#[derive(Debug, Clone, Serialize)]
pub struct ExecutionLogResponse {
    pub id: String,
    pub execution_type: String,
    pub resource_id: String,
    pub resource_name: Option<String>,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<serde_json::Value>,
    pub error: Option<String>,
    pub cost_micros: Option<i64>,
    pub token_usage: Option<TokenUsageResponse>,
    pub execution_time_ms: u64,
    pub executor: ExecutorResponse,
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workflow_steps: Option<Vec<WorkflowStepLogResponse>>,
}

/// Workflow step log response
#[derive(Debug, Clone, Serialize)]
pub struct WorkflowStepLogResponse {
    pub step_name: String,
    pub step_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub execution_time_ms: u64,
    pub status: String,
}

/// Token usage response
#[derive(Debug, Clone, Serialize)]
pub struct TokenUsageResponse {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub total_tokens: u32,
}

/// Executor response
#[derive(Debug, Clone, Serialize)]
pub struct ExecutorResponse {
    pub user_id: Option<String>,
    pub api_key_id: Option<String>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}

/// List execution logs response
#[derive(Debug, Clone, Serialize)]
pub struct ListExecutionLogsResponse {
    pub logs: Vec<ExecutionLogResponse>,
    pub total: usize,
}

/// Execution statistics response
#[derive(Debug, Clone, Serialize)]
pub struct ExecutionStatsResponse {
    pub total_executions: usize,
    pub successful_executions: usize,
    pub failed_executions: usize,
    pub success_rate: f64,
    pub total_cost_micros: i64,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub avg_execution_time_ms: f64,
    pub executions_by_type: std::collections::HashMap<String, usize>,
    pub executions_by_resource: std::collections::HashMap<String, usize>,
}

/// Query parameters for listing execution logs
#[derive(Debug, Clone, Deserialize)]
pub struct ListExecutionLogsQuery {
    pub execution_type: Option<String>,
    pub resource_id: Option<String>,
    pub status: Option<String>,
    pub api_key_id: Option<String>,
    pub user_id: Option<String>,
    pub from_date: Option<String>,
    pub to_date: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

impl ListExecutionLogsQuery {
    fn to_domain_query(&self) -> Result<ExecutionLogQuery, ApiError> {
        let mut query = ExecutionLogQuery::new();

        if let Some(ref exec_type) = self.execution_type {
            let execution_type = match exec_type.to_lowercase().as_str() {
                "model" => ExecutionType::Model,
                "workflow" => ExecutionType::Workflow,
                "chat_completion" => ExecutionType::ChatCompletion,
                _ => return Err(ApiError::bad_request(format!(
                    "Invalid execution type: {}",
                    exec_type
                ))),
            };
            query = query.with_execution_type(execution_type);
        }

        if let Some(ref resource_id) = self.resource_id {
            query = query.with_resource_id(resource_id.clone());
        }

        if let Some(ref status) = self.status {
            let status = match status.to_lowercase().as_str() {
                "success" => ExecutionStatus::Success,
                "failed" => ExecutionStatus::Failed,
                "timeout" => ExecutionStatus::Timeout,
                "cancelled" => ExecutionStatus::Cancelled,
                _ => return Err(ApiError::bad_request(format!("Invalid status: {}", status))),
            };
            query = query.with_status(status);
        }

        if let Some(ref api_key_id) = self.api_key_id {
            query = query.with_api_key_id(api_key_id.clone());
        }

        if let Some(ref user_id) = self.user_id {
            query = query.with_user_id(user_id.clone());
        }

        if let (Some(from), Some(to)) = (&self.from_date, &self.to_date) {
            let from_date = chrono::DateTime::parse_from_rfc3339(from)
                .map_err(|e| ApiError::bad_request(format!("Invalid from_date: {}", e)))?
                .with_timezone(&chrono::Utc);
            let to_date = chrono::DateTime::parse_from_rfc3339(to)
                .map_err(|e| ApiError::bad_request(format!("Invalid to_date: {}", e)))?
                .with_timezone(&chrono::Utc);
            query = query.with_date_range(from_date, to_date);
        }

        if let Some(limit) = self.limit {
            query = query.with_limit(limit);
        }

        if let Some(offset) = self.offset {
            query = query.with_offset(offset);
        }

        Ok(query)
    }
}

/// List execution logs
pub async fn list_execution_logs(
    _admin: RequireAdmin,
    State(state): State<AppState>,
    Query(query_params): Query<ListExecutionLogsQuery>,
) -> Result<Json<ListExecutionLogsResponse>, ApiError> {
    let query = query_params.to_domain_query()?;
    let count_query = query_params.to_domain_query()?;

    let logs = state.execution_log_service.list(&query).await?;
    let total = state.execution_log_service.count(&count_query).await?;

    let logs = logs
        .into_iter()
        .map(|log| ExecutionLogResponse {
            id: log.id().to_string(),
            execution_type: log.execution_type().to_string(),
            resource_id: log.resource_id().to_string(),
            resource_name: log.resource_name().map(|s| s.to_string()),
            status: log.status().to_string(),
            input: log.input().cloned(),
            output: log.output().cloned(),
            error: log.error().map(|s| s.to_string()),
            cost_micros: log.cost_micros(),
            token_usage: log.token_usage().map(|u| TokenUsageResponse {
                input_tokens: u.input_tokens,
                output_tokens: u.output_tokens,
                total_tokens: u.total_tokens,
            }),
            execution_time_ms: log.execution_time_ms(),
            executor: ExecutorResponse {
                user_id: log.executor().user_id.clone(),
                api_key_id: log.executor().api_key_id.clone(),
                ip_address: log.executor().ip_address.clone(),
                user_agent: log.executor().user_agent.clone(),
            },
            created_at: log.created_at().to_rfc3339(),
            workflow_steps: log.workflow_steps().map(|steps| {
                steps
                    .iter()
                    .map(|step| WorkflowStepLogResponse {
                        step_name: step.step_name.clone(),
                        step_type: step.step_type.clone(),
                        input: step.input.clone(),
                        output: step.output.clone(),
                        error: step.error.clone(),
                        execution_time_ms: step.execution_time_ms,
                        status: step.status.to_string(),
                    })
                    .collect()
            }),
        })
        .collect();

    Ok(Json(ListExecutionLogsResponse { logs, total }))
}

/// Get execution log by ID
pub async fn get_execution_log(
    _admin: RequireAdmin,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ExecutionLogResponse>, ApiError> {
    let log = state
        .execution_log_service
        .get(&id)
        .await?
        .ok_or_else(|| ApiError::not_found(format!("Execution log '{}' not found", id)))?;

    Ok(Json(ExecutionLogResponse {
        id: log.id().to_string(),
        execution_type: log.execution_type().to_string(),
        resource_id: log.resource_id().to_string(),
        resource_name: log.resource_name().map(|s| s.to_string()),
        status: log.status().to_string(),
        input: log.input().cloned(),
        output: log.output().cloned(),
        error: log.error().map(|s| s.to_string()),
        cost_micros: log.cost_micros(),
        token_usage: log.token_usage().map(|u| TokenUsageResponse {
            input_tokens: u.input_tokens,
            output_tokens: u.output_tokens,
            total_tokens: u.total_tokens,
        }),
        execution_time_ms: log.execution_time_ms(),
        executor: ExecutorResponse {
            user_id: log.executor().user_id.clone(),
            api_key_id: log.executor().api_key_id.clone(),
            ip_address: log.executor().ip_address.clone(),
            user_agent: log.executor().user_agent.clone(),
        },
        created_at: log.created_at().to_rfc3339(),
        workflow_steps: log.workflow_steps().map(|steps| {
            steps
                .iter()
                .map(|step| WorkflowStepLogResponse {
                    step_name: step.step_name.clone(),
                    step_type: step.step_type.clone(),
                    input: step.input.clone(),
                    output: step.output.clone(),
                    error: step.error.clone(),
                    execution_time_ms: step.execution_time_ms,
                    status: step.status.to_string(),
                })
                .collect()
        }),
    }))
}

/// Delete execution log by ID
pub async fn delete_execution_log(
    _admin: RequireAdmin,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let deleted = state.execution_log_service.delete(&id).await?;

    if deleted {
        Ok(Json(serde_json::json!({
            "deleted": true,
            "id": id
        })))
    } else {
        Err(ApiError::not_found(format!(
            "Execution log '{}' not found",
            id
        )))
    }
}

/// Get execution statistics
pub async fn get_execution_stats(
    _admin: RequireAdmin,
    State(state): State<AppState>,
    Query(query_params): Query<ListExecutionLogsQuery>,
) -> Result<Json<ExecutionStatsResponse>, ApiError> {
    let query = query_params.to_domain_query()?;
    let stats = state.execution_log_service.stats(&query).await?;

    Ok(Json(ExecutionStatsResponse {
        total_executions: stats.total_executions,
        successful_executions: stats.successful_executions,
        failed_executions: stats.failed_executions,
        success_rate: stats.success_rate(),
        total_cost_micros: stats.total_cost_micros,
        total_input_tokens: stats.total_input_tokens,
        total_output_tokens: stats.total_output_tokens,
        avg_execution_time_ms: stats.avg_execution_time_ms,
        executions_by_type: stats.executions_by_type,
        executions_by_resource: stats.executions_by_resource,
    }))
}

/// Cleanup old execution logs
#[derive(Debug, Clone, Deserialize)]
pub struct CleanupRequest {
    pub days: Option<i64>,
}

pub async fn cleanup_execution_logs(
    _admin: RequireAdmin,
    State(state): State<AppState>,
    Json(request): Json<CleanupRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let deleted = if let Some(days) = request.days {
        state.execution_log_service.delete_older_than(days).await?
    } else {
        state.execution_log_service.cleanup_old_logs().await?
    };

    Ok(Json(serde_json::json!({
        "deleted_count": deleted
    })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_usage_response_serialization() {
        let response = TokenUsageResponse {
            input_tokens: 100,
            output_tokens: 50,
            total_tokens: 150,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"input_tokens\":100"));
        assert!(json.contains("\"output_tokens\":50"));
        assert!(json.contains("\"total_tokens\":150"));
    }

    #[test]
    fn test_executor_response_serialization() {
        let response = ExecutorResponse {
            user_id: Some("user-123".to_string()),
            api_key_id: Some("key-456".to_string()),
            ip_address: Some("192.168.1.1".to_string()),
            user_agent: Some("Mozilla/5.0".to_string()),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"user_id\":\"user-123\""));
        assert!(json.contains("\"api_key_id\":\"key-456\""));
        assert!(json.contains("\"ip_address\":\"192.168.1.1\""));
        assert!(json.contains("\"user_agent\":\"Mozilla/5.0\""));
    }

    #[test]
    fn test_executor_response_with_nulls() {
        let response = ExecutorResponse {
            user_id: None,
            api_key_id: Some("key-789".to_string()),
            ip_address: None,
            user_agent: None,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"user_id\":null"));
        assert!(json.contains("\"api_key_id\":\"key-789\""));
    }

    #[test]
    fn test_execution_log_response_serialization() {
        let response = ExecutionLogResponse {
            id: "log-001".to_string(),
            execution_type: "model".to_string(),
            resource_id: "model-gpt4".to_string(),
            resource_name: Some("GPT-4".to_string()),
            status: "success".to_string(),
            input: None,
            output: None,
            error: None,
            cost_micros: Some(1500),
            token_usage: Some(TokenUsageResponse {
                input_tokens: 100,
                output_tokens: 50,
                total_tokens: 150,
            }),
            execution_time_ms: 250,
            executor: ExecutorResponse {
                user_id: Some("user-1".to_string()),
                api_key_id: None,
                ip_address: None,
                user_agent: None,
            },
            created_at: "2024-01-01T00:00:00Z".to_string(),
            workflow_steps: None,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"id\":\"log-001\""));
        assert!(json.contains("\"execution_type\":\"model\""));
        assert!(json.contains("\"resource_id\":\"model-gpt4\""));
        assert!(json.contains("\"status\":\"success\""));
        assert!(json.contains("\"execution_time_ms\":250"));
    }

    #[test]
    fn test_execution_log_response_with_error() {
        let response = ExecutionLogResponse {
            id: "log-002".to_string(),
            execution_type: "workflow".to_string(),
            resource_id: "workflow-1".to_string(),
            resource_name: None,
            status: "failed".to_string(),
            input: None,
            output: None,
            error: Some("Connection timeout".to_string()),
            cost_micros: None,
            token_usage: None,
            execution_time_ms: 5000,
            executor: ExecutorResponse {
                user_id: None,
                api_key_id: Some("key-test".to_string()),
                ip_address: Some("10.0.0.1".to_string()),
                user_agent: None,
            },
            created_at: "2024-01-01T00:00:00Z".to_string(),
            workflow_steps: None,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"status\":\"failed\""));
        assert!(json.contains("\"error\":\"Connection timeout\""));
        assert!(json.contains("\"cost_micros\":null"));
    }

    #[test]
    fn test_list_execution_logs_response_serialization() {
        let response = ListExecutionLogsResponse {
            logs: vec![
                ExecutionLogResponse {
                    id: "log-1".to_string(),
                    execution_type: "model".to_string(),
                    resource_id: "model-1".to_string(),
                    resource_name: None,
                    status: "success".to_string(),
                    input: None,
                    output: None,
                    error: None,
                    cost_micros: Some(100),
                    token_usage: None,
                    execution_time_ms: 100,
                    executor: ExecutorResponse {
                        user_id: None,
                        api_key_id: None,
                        ip_address: None,
                        user_agent: None,
                    },
                    created_at: "2024-01-01T00:00:00Z".to_string(),
                    workflow_steps: None,
                },
            ],
            total: 50,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"logs\":"));
        assert!(json.contains("\"total\":50"));
    }

    #[test]
    fn test_execution_stats_response_serialization() {
        let mut by_type = std::collections::HashMap::new();
        by_type.insert("model".to_string(), 100);
        by_type.insert("workflow".to_string(), 50);

        let mut by_resource = std::collections::HashMap::new();
        by_resource.insert("gpt-4".to_string(), 75);

        let response = ExecutionStatsResponse {
            total_executions: 150,
            successful_executions: 140,
            failed_executions: 10,
            success_rate: 0.933,
            total_cost_micros: 50000,
            total_input_tokens: 100000,
            total_output_tokens: 50000,
            avg_execution_time_ms: 245.5,
            executions_by_type: by_type,
            executions_by_resource: by_resource,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"total_executions\":150"));
        assert!(json.contains("\"successful_executions\":140"));
        assert!(json.contains("\"failed_executions\":10"));
        assert!(json.contains("\"total_cost_micros\":50000"));
    }

    #[test]
    fn test_list_execution_logs_query_deserialization() {
        let json = r#"{
            "execution_type": "model",
            "resource_id": "model-1",
            "status": "success",
            "limit": 100,
            "offset": 0
        }"#;

        let query: ListExecutionLogsQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.execution_type, Some("model".to_string()));
        assert_eq!(query.resource_id, Some("model-1".to_string()));
        assert_eq!(query.status, Some("success".to_string()));
        assert_eq!(query.limit, Some(100));
        assert_eq!(query.offset, Some(0));
    }

    #[test]
    fn test_list_execution_logs_query_minimal() {
        let json = r#"{}"#;
        let query: ListExecutionLogsQuery = serde_json::from_str(json).unwrap();
        assert!(query.execution_type.is_none());
        assert!(query.resource_id.is_none());
        assert!(query.status.is_none());
    }

    #[test]
    fn test_list_execution_logs_query_with_dates() {
        let json = r#"{
            "from_date": "2024-01-01T00:00:00Z",
            "to_date": "2024-01-31T23:59:59Z"
        }"#;

        let query: ListExecutionLogsQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.from_date, Some("2024-01-01T00:00:00Z".to_string()));
        assert_eq!(query.to_date, Some("2024-01-31T23:59:59Z".to_string()));
    }

    #[test]
    fn test_list_execution_logs_query_with_api_key() {
        let json = r#"{"api_key_id": "key-123"}"#;
        let query: ListExecutionLogsQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.api_key_id, Some("key-123".to_string()));
    }

    #[test]
    fn test_list_execution_logs_query_with_user_id() {
        let json = r#"{"user_id": "user-456"}"#;
        let query: ListExecutionLogsQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.user_id, Some("user-456".to_string()));
    }

    #[test]
    fn test_to_domain_query_model_type() {
        let query = ListExecutionLogsQuery {
            execution_type: Some("model".to_string()),
            resource_id: None,
            status: None,
            api_key_id: None,
            user_id: None,
            from_date: None,
            to_date: None,
            limit: None,
            offset: None,
        };

        let result = query.to_domain_query();
        assert!(result.is_ok());
    }

    #[test]
    fn test_to_domain_query_workflow_type() {
        let query = ListExecutionLogsQuery {
            execution_type: Some("workflow".to_string()),
            resource_id: None,
            status: None,
            api_key_id: None,
            user_id: None,
            from_date: None,
            to_date: None,
            limit: None,
            offset: None,
        };

        let result = query.to_domain_query();
        assert!(result.is_ok());
    }

    #[test]
    fn test_to_domain_query_chat_completion_type() {
        let query = ListExecutionLogsQuery {
            execution_type: Some("chat_completion".to_string()),
            resource_id: None,
            status: None,
            api_key_id: None,
            user_id: None,
            from_date: None,
            to_date: None,
            limit: None,
            offset: None,
        };

        let result = query.to_domain_query();
        assert!(result.is_ok());
    }

    #[test]
    fn test_to_domain_query_invalid_type() {
        let query = ListExecutionLogsQuery {
            execution_type: Some("invalid".to_string()),
            resource_id: None,
            status: None,
            api_key_id: None,
            user_id: None,
            from_date: None,
            to_date: None,
            limit: None,
            offset: None,
        };

        let result = query.to_domain_query();
        assert!(result.is_err());
    }

    #[test]
    fn test_to_domain_query_success_status() {
        let query = ListExecutionLogsQuery {
            execution_type: None,
            resource_id: None,
            status: Some("success".to_string()),
            api_key_id: None,
            user_id: None,
            from_date: None,
            to_date: None,
            limit: None,
            offset: None,
        };

        let result = query.to_domain_query();
        assert!(result.is_ok());
    }

    #[test]
    fn test_to_domain_query_failed_status() {
        let query = ListExecutionLogsQuery {
            execution_type: None,
            resource_id: None,
            status: Some("failed".to_string()),
            api_key_id: None,
            user_id: None,
            from_date: None,
            to_date: None,
            limit: None,
            offset: None,
        };

        let result = query.to_domain_query();
        assert!(result.is_ok());
    }

    #[test]
    fn test_to_domain_query_timeout_status() {
        let query = ListExecutionLogsQuery {
            execution_type: None,
            resource_id: None,
            status: Some("timeout".to_string()),
            api_key_id: None,
            user_id: None,
            from_date: None,
            to_date: None,
            limit: None,
            offset: None,
        };

        let result = query.to_domain_query();
        assert!(result.is_ok());
    }

    #[test]
    fn test_to_domain_query_cancelled_status() {
        let query = ListExecutionLogsQuery {
            execution_type: None,
            resource_id: None,
            status: Some("cancelled".to_string()),
            api_key_id: None,
            user_id: None,
            from_date: None,
            to_date: None,
            limit: None,
            offset: None,
        };

        let result = query.to_domain_query();
        assert!(result.is_ok());
    }

    #[test]
    fn test_to_domain_query_invalid_status() {
        let query = ListExecutionLogsQuery {
            execution_type: None,
            resource_id: None,
            status: Some("unknown".to_string()),
            api_key_id: None,
            user_id: None,
            from_date: None,
            to_date: None,
            limit: None,
            offset: None,
        };

        let result = query.to_domain_query();
        assert!(result.is_err());
    }

    #[test]
    fn test_to_domain_query_with_dates() {
        let query = ListExecutionLogsQuery {
            execution_type: None,
            resource_id: None,
            status: None,
            api_key_id: None,
            user_id: None,
            from_date: Some("2024-01-01T00:00:00Z".to_string()),
            to_date: Some("2024-01-31T23:59:59Z".to_string()),
            limit: None,
            offset: None,
        };

        let result = query.to_domain_query();
        assert!(result.is_ok());
    }

    #[test]
    fn test_to_domain_query_invalid_from_date() {
        let query = ListExecutionLogsQuery {
            execution_type: None,
            resource_id: None,
            status: None,
            api_key_id: None,
            user_id: None,
            from_date: Some("not-a-date".to_string()),
            to_date: Some("2024-01-31T23:59:59Z".to_string()),
            limit: None,
            offset: None,
        };

        let result = query.to_domain_query();
        assert!(result.is_err());
    }

    #[test]
    fn test_to_domain_query_with_limit_offset() {
        let query = ListExecutionLogsQuery {
            execution_type: None,
            resource_id: None,
            status: None,
            api_key_id: None,
            user_id: None,
            from_date: None,
            to_date: None,
            limit: Some(50),
            offset: Some(100),
        };

        let result = query.to_domain_query();
        assert!(result.is_ok());
    }

    #[test]
    fn test_cleanup_request_deserialization() {
        let json = r#"{"days": 30}"#;
        let request: CleanupRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.days, Some(30));
    }

    #[test]
    fn test_cleanup_request_without_days() {
        let json = r#"{}"#;
        let request: CleanupRequest = serde_json::from_str(json).unwrap();
        assert!(request.days.is_none());
    }
}
