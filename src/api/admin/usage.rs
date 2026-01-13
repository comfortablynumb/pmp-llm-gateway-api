//! Usage tracking and budget management admin endpoints

use axum::extract::{Path, Query, State};
use serde::{Deserialize, Serialize};

use crate::api::middleware::RequireAdmin;
use crate::api::state::AppState;
use crate::api::types::{ApiError, Json};
use crate::domain::usage::{
    Budget, BudgetId, BudgetPeriod, BudgetScope, UsageAggregate, UsageRecord, UsageSummary,
};

// ============================================================================
// Usage Query DTOs
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct UsageQueryParams {
    pub api_key_id: Option<String>,
    pub model_id: Option<String>,
    pub from_timestamp: Option<u64>,
    pub to_timestamp: Option<u64>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct UsageRecordResponse {
    pub id: String,
    pub usage_type: String,
    pub api_key_id: String,
    pub model_id: Option<String>,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub total_tokens: u32,
    pub cost_usd: f64,
    pub latency_ms: u64,
    pub success: bool,
    pub error: Option<String>,
    pub timestamp: u64,
}

impl From<UsageRecord> for UsageRecordResponse {
    fn from(record: UsageRecord) -> Self {
        let cost_usd = record.cost_usd();
        Self {
            id: record.id().to_string(),
            usage_type: record.usage_type.to_string(),
            api_key_id: record.api_key_id,
            model_id: record.model_id,
            input_tokens: record.input_tokens,
            output_tokens: record.output_tokens,
            total_tokens: record.total_tokens,
            cost_usd,
            latency_ms: record.latency_ms,
            success: record.success,
            error: record.error,
            timestamp: record.timestamp,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct UsageAggregateResponse {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_tokens: u64,
    pub total_cost_usd: f64,
    pub avg_latency_ms: f64,
    pub success_rate: f64,
}

impl From<UsageAggregate> for UsageAggregateResponse {
    fn from(agg: UsageAggregate) -> Self {
        Self {
            total_requests: agg.total_requests,
            successful_requests: agg.successful_requests,
            failed_requests: agg.failed_requests,
            total_input_tokens: agg.total_input_tokens,
            total_output_tokens: agg.total_output_tokens,
            total_tokens: agg.total_tokens,
            total_cost_usd: agg.total_cost_usd(),
            avg_latency_ms: agg.avg_latency_ms,
            success_rate: agg.success_rate(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct UsageSummaryResponse {
    pub period_start: u64,
    pub period_end: u64,
    pub aggregate: UsageAggregateResponse,
    pub daily: Vec<DailyUsageResponse>,
}

#[derive(Debug, Serialize)]
pub struct DailyUsageResponse {
    pub date: u64,
    pub requests: u64,
    pub tokens: u64,
    pub cost_usd: f64,
}

impl From<UsageSummary> for UsageSummaryResponse {
    fn from(summary: UsageSummary) -> Self {
        Self {
            period_start: summary.period_start,
            period_end: summary.period_end,
            aggregate: summary.aggregate.into(),
            daily: summary
                .daily
                .into_iter()
                .map(|d| DailyUsageResponse {
                    date: d.date,
                    requests: d.requests,
                    tokens: d.tokens,
                    cost_usd: d.cost_usd(),
                })
                .collect(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct UsageListResponse {
    pub records: Vec<UsageRecordResponse>,
    pub count: usize,
}

// ============================================================================
// Budget DTOs
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct CreateBudgetRequest {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub period: String,
    pub hard_limit_usd: f64,
    pub soft_limit_usd: Option<f64>,
    pub api_key_ids: Option<Vec<String>>,
    pub team_ids: Option<Vec<String>>,
    pub model_ids: Option<Vec<String>>,
    pub alert_thresholds: Option<Vec<u8>>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateBudgetRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub hard_limit_usd: Option<f64>,
    pub soft_limit_usd: Option<f64>,
    pub api_key_ids: Option<Vec<String>>,
    pub team_ids: Option<Vec<String>>,
    pub model_ids: Option<Vec<String>>,
    pub alert_thresholds: Option<Vec<u8>>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct BudgetResponse {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub period: String,
    pub hard_limit_usd: f64,
    pub soft_limit_usd: Option<f64>,
    pub current_usage_usd: f64,
    pub remaining_usd: f64,
    pub usage_percent: f64,
    pub status: String,
    pub scope: String,
    pub api_key_ids: Vec<String>,
    pub team_ids: Vec<String>,
    pub model_ids: Vec<String>,
    pub alerts: Vec<BudgetAlertResponse>,
    pub period_start: u64,
    pub enabled: bool,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Debug, Serialize)]
pub struct BudgetAlertResponse {
    pub threshold_percent: u8,
    pub triggered: bool,
    pub triggered_at: Option<u64>,
}

impl From<Budget> for BudgetResponse {
    fn from(budget: Budget) -> Self {
        let hard_limit_usd = budget.hard_limit_usd();
        let soft_limit_usd = budget.soft_limit_usd();
        let current_usage_usd = budget.current_usage_usd();
        let remaining_usd = budget.remaining_usd();
        let usage_percent = budget.usage_percent();
        let period = budget.period.to_string();
        let status = budget.status.to_string();
        let scope = scope_to_string(budget.scope);

        Self {
            id: budget.id().to_string(),
            name: budget.name,
            description: budget.description,
            period,
            hard_limit_usd,
            soft_limit_usd,
            current_usage_usd,
            remaining_usd,
            usage_percent,
            status,
            scope,
            api_key_ids: budget.api_key_ids,
            team_ids: budget.team_ids,
            model_ids: budget.model_ids,
            alerts: budget
                .alerts
                .into_iter()
                .map(|a| BudgetAlertResponse {
                    threshold_percent: a.threshold_percent,
                    triggered: a.triggered,
                    triggered_at: a.triggered_at,
                })
                .collect(),
            period_start: budget.period_start,
            enabled: budget.enabled,
            created_at: budget.created_at,
            updated_at: budget.updated_at,
        }
    }
}

fn scope_to_string(scope: BudgetScope) -> String {
    match scope {
        BudgetScope::AllApiKeys => "all_api_keys".to_string(),
        BudgetScope::SpecificApiKeys => "specific_api_keys".to_string(),
        BudgetScope::Teams => "teams".to_string(),
        BudgetScope::Mixed => "mixed".to_string(),
    }
}

#[derive(Debug, Serialize)]
pub struct BudgetListResponse {
    pub budgets: Vec<BudgetResponse>,
}

// ============================================================================
// Usage Endpoints
// ============================================================================

/// List usage records
pub async fn list_usage(
    RequireAdmin(_): RequireAdmin,
    State(state): State<AppState>,
    Query(params): Query<UsageQueryParams>,
) -> Result<Json<UsageListResponse>, ApiError> {
    let query = build_usage_query(&params);
    let records = state.usage_service.query(&query).await?;
    let count = records.len();

    Ok(Json(UsageListResponse {
        records: records.into_iter().map(Into::into).collect(),
        count,
    }))
}

/// Get usage aggregate
pub async fn get_usage_aggregate(
    RequireAdmin(_): RequireAdmin,
    State(state): State<AppState>,
    Query(params): Query<UsageQueryParams>,
) -> Result<Json<UsageAggregateResponse>, ApiError> {
    let query = build_usage_query(&params);
    let aggregate = state.usage_service.aggregate(&query).await?;

    Ok(Json(aggregate.into()))
}

/// Get usage summary with daily breakdown
pub async fn get_usage_summary(
    RequireAdmin(_): RequireAdmin,
    State(state): State<AppState>,
    Query(params): Query<UsageQueryParams>,
) -> Result<Json<UsageSummaryResponse>, ApiError> {
    let query = build_usage_query(&params);
    let summary = state.usage_service.summary(&query).await?;

    Ok(Json(summary.into()))
}

/// Delete old usage records
#[derive(Debug, Deserialize)]
pub struct DeleteUsageParams {
    pub before_timestamp: u64,
}

pub async fn delete_usage(
    RequireAdmin(_): RequireAdmin,
    State(state): State<AppState>,
    Query(params): Query<DeleteUsageParams>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let count = state
        .usage_service
        .delete_before(params.before_timestamp)
        .await?;

    Ok(Json(serde_json::json!({
        "deleted": count
    })))
}

fn build_usage_query(params: &UsageQueryParams) -> crate::domain::usage::UsageQuery {
    let mut query = crate::domain::usage::UsageQuery::new();

    if let Some(ref api_key_id) = params.api_key_id {
        query = query.with_api_key(api_key_id);
    }

    if let Some(ref model_id) = params.model_id {
        query = query.with_model(model_id);
    }

    if let (Some(from), Some(to)) = (params.from_timestamp, params.to_timestamp) {
        query = query.with_time_range(from, to);
    }

    if let Some(limit) = params.limit {
        query = query.with_limit(limit);
    }

    if let Some(offset) = params.offset {
        query = query.with_offset(offset);
    }

    query
}

// ============================================================================
// Budget Endpoints
// ============================================================================

/// List all budgets
pub async fn list_budgets(
    RequireAdmin(_): RequireAdmin,
    State(state): State<AppState>,
) -> Result<Json<BudgetListResponse>, ApiError> {
    let budgets = state.budget_service.list().await?;

    Ok(Json(BudgetListResponse {
        budgets: budgets.into_iter().map(Into::into).collect(),
    }))
}

/// List budgets by team
pub async fn list_budgets_by_team(
    RequireAdmin(_): RequireAdmin,
    State(state): State<AppState>,
    Path(team_id): Path<String>,
) -> Result<Json<BudgetListResponse>, ApiError> {
    let budgets = state.budget_service.list_by_team(&team_id).await?;

    Ok(Json(BudgetListResponse {
        budgets: budgets.into_iter().map(Into::into).collect(),
    }))
}

/// Create a new budget
pub async fn create_budget(
    RequireAdmin(_): RequireAdmin,
    State(state): State<AppState>,
    Json(request): Json<CreateBudgetRequest>,
) -> Result<Json<BudgetResponse>, ApiError> {
    let period = parse_budget_period(&request.period)?;

    let mut budget = Budget::new(&request.id, &request.name, period)
        .with_hard_limit(request.hard_limit_usd);

    if let Some(desc) = request.description {
        budget = budget.with_description(desc);
    }

    if let Some(soft_limit) = request.soft_limit_usd {
        budget = budget.with_soft_limit(soft_limit);
    }

    if let Some(api_key_ids) = request.api_key_ids {
        budget = budget.with_api_keys(api_key_ids);
    }

    if let Some(team_ids) = request.team_ids {
        budget = budget.with_teams(team_ids);
    }

    if let Some(model_ids) = request.model_ids {
        for model_id in model_ids {
            budget = budget.with_model(model_id);
        }
    }

    if let Some(thresholds) = request.alert_thresholds {
        for threshold in thresholds {
            budget = budget.with_alert_at(threshold);
        }
    }

    let created = state.budget_service.create(budget).await?;

    Ok(Json(created.into()))
}

/// Get a budget by ID
pub async fn get_budget(
    RequireAdmin(_): RequireAdmin,
    State(state): State<AppState>,
    Path(budget_id): Path<String>,
) -> Result<Json<BudgetResponse>, ApiError> {
    let id = BudgetId::from(&budget_id);
    let budget = state
        .budget_service
        .get(&id)
        .await?
        .ok_or_else(|| ApiError::not_found(&format!("Budget '{}' not found", budget_id)))?;

    Ok(Json(budget.into()))
}

/// Update a budget
pub async fn update_budget(
    RequireAdmin(_): RequireAdmin,
    State(state): State<AppState>,
    Path(budget_id): Path<String>,
    Json(request): Json<UpdateBudgetRequest>,
) -> Result<Json<BudgetResponse>, ApiError> {
    let id = BudgetId::from(&budget_id);
    let mut budget = state
        .budget_service
        .get(&id)
        .await?
        .ok_or_else(|| ApiError::not_found(&format!("Budget '{}' not found", budget_id)))?;

    if let Some(name) = request.name {
        budget.name = name;
    }

    if let Some(desc) = request.description {
        budget.description = Some(desc);
    }

    if let Some(hard_limit) = request.hard_limit_usd {
        budget.hard_limit_micros = (hard_limit * 1_000_000.0) as i64;
    }

    if let Some(soft_limit) = request.soft_limit_usd {
        budget.soft_limit_micros = Some((soft_limit * 1_000_000.0) as i64);
    }

    // Handle api_key_ids and team_ids updates with scope recalculation
    let api_key_ids_changed = request.api_key_ids.is_some();
    let team_ids_changed = request.team_ids.is_some();

    if let Some(api_key_ids) = request.api_key_ids {
        budget.api_key_ids = api_key_ids;
    }

    if let Some(team_ids) = request.team_ids {
        budget.team_ids = team_ids;
    }

    // Recalculate scope if api_key_ids or team_ids changed
    if api_key_ids_changed || team_ids_changed {
        budget.scope = match (!budget.api_key_ids.is_empty(), !budget.team_ids.is_empty()) {
            (false, false) => BudgetScope::AllApiKeys,
            (true, false) => BudgetScope::SpecificApiKeys,
            (false, true) => BudgetScope::Teams,
            (true, true) => BudgetScope::Mixed,
        };
    }

    if let Some(model_ids) = request.model_ids {
        budget.model_ids = model_ids;
    }

    if let Some(enabled) = request.enabled {
        budget.enabled = enabled;
    }

    if let Some(thresholds) = request.alert_thresholds {
        budget.alerts.clear();
        for threshold in thresholds {
            budget = budget.with_alert_at(threshold);
        }
    }

    let updated = state.budget_service.update(budget).await?;

    Ok(Json(updated.into()))
}

/// Delete a budget
pub async fn delete_budget(
    RequireAdmin(_): RequireAdmin,
    State(state): State<AppState>,
    Path(budget_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let id = BudgetId::from(&budget_id);
    let deleted = state.budget_service.delete(&id).await?;

    if deleted {
        Ok(Json(serde_json::json!({
            "deleted": true,
            "id": budget_id
        })))
    } else {
        Err(ApiError::not_found(&format!(
            "Budget '{}' not found",
            budget_id
        )))
    }
}

/// Reset a budget period
pub async fn reset_budget(
    RequireAdmin(_): RequireAdmin,
    State(state): State<AppState>,
    Path(budget_id): Path<String>,
) -> Result<Json<BudgetResponse>, ApiError> {
    let id = BudgetId::from(&budget_id);
    let mut budget = state
        .budget_service
        .get(&id)
        .await?
        .ok_or_else(|| ApiError::not_found(&format!("Budget '{}' not found", budget_id)))?;

    budget.reset_period();

    let updated = state.budget_service.update(budget).await?;

    Ok(Json(updated.into()))
}

/// Check budget for a request
#[derive(Debug, Deserialize)]
pub struct CheckBudgetRequest {
    pub api_key_id: String,
    pub team_id: Option<String>,
    pub model_id: Option<String>,
    pub estimated_cost_usd: f64,
}

#[derive(Debug, Serialize)]
pub struct CheckBudgetResponse {
    pub allowed: bool,
    pub exceeded_budgets: Vec<String>,
    pub warning_budgets: Vec<String>,
    pub estimated_cost_usd: f64,
}

pub async fn check_budget(
    RequireAdmin(_): RequireAdmin,
    State(state): State<AppState>,
    Json(request): Json<CheckBudgetRequest>,
) -> Result<Json<CheckBudgetResponse>, ApiError> {
    let estimated_cost_micros = (request.estimated_cost_usd * 1_000_000.0) as i64;

    let result = state
        .budget_service
        .check_budget_with_team(
            &request.api_key_id,
            request.team_id.as_deref(),
            request.model_id.as_deref(),
            estimated_cost_micros,
        )
        .await?;

    Ok(Json(CheckBudgetResponse {
        allowed: result.allowed,
        exceeded_budgets: result.exceeded_budgets.into_iter().map(|id| id.to_string()).collect(),
        warning_budgets: result.warning_budgets.into_iter().map(|id| id.to_string()).collect(),
        estimated_cost_usd: request.estimated_cost_usd,
    }))
}

fn parse_budget_period(period: &str) -> Result<BudgetPeriod, ApiError> {
    match period.to_lowercase().as_str() {
        "daily" => Ok(BudgetPeriod::Daily),
        "weekly" => Ok(BudgetPeriod::Weekly),
        "monthly" => Ok(BudgetPeriod::Monthly),
        "lifetime" => Ok(BudgetPeriod::Lifetime),
        _ => Err(ApiError::bad_request(format!(
            "Invalid budget period: {}. Must be one of: daily, weekly, monthly, lifetime",
            period
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_budget_period() {
        assert_eq!(parse_budget_period("daily").unwrap(), BudgetPeriod::Daily);
        assert_eq!(parse_budget_period("WEEKLY").unwrap(), BudgetPeriod::Weekly);
        assert_eq!(parse_budget_period("Monthly").unwrap(), BudgetPeriod::Monthly);
        assert_eq!(parse_budget_period("lifetime").unwrap(), BudgetPeriod::Lifetime);
        assert!(parse_budget_period("invalid").is_err());
    }

    #[test]
    fn test_scope_to_string() {
        assert_eq!(scope_to_string(BudgetScope::AllApiKeys), "all_api_keys");
        assert_eq!(scope_to_string(BudgetScope::SpecificApiKeys), "specific_api_keys");
        assert_eq!(scope_to_string(BudgetScope::Teams), "teams");
        assert_eq!(scope_to_string(BudgetScope::Mixed), "mixed");
    }

    #[test]
    fn test_create_budget_request_deserialization() {
        let json = r#"{
            "id": "monthly-budget",
            "name": "Monthly Budget",
            "description": "Team monthly spending limit",
            "period": "monthly",
            "hard_limit_usd": 1000.0,
            "soft_limit_usd": 800.0,
            "api_key_ids": ["key-1", "key-2"],
            "team_ids": ["team-1"],
            "model_ids": ["gpt-4"],
            "alert_thresholds": [50, 75, 90]
        }"#;

        let request: CreateBudgetRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.id, "monthly-budget");
        assert_eq!(request.period, "monthly");
        assert_eq!(request.hard_limit_usd, 1000.0);
        assert_eq!(request.soft_limit_usd, Some(800.0));
        assert_eq!(request.api_key_ids.as_ref().unwrap().len(), 2);
        assert_eq!(request.alert_thresholds.as_ref().unwrap().len(), 3);
    }

    #[test]
    fn test_create_budget_request_minimal() {
        let json = r#"{
            "id": "simple-budget",
            "name": "Simple",
            "period": "daily",
            "hard_limit_usd": 100.0
        }"#;

        let request: CreateBudgetRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.id, "simple-budget");
        assert!(request.description.is_none());
        assert!(request.soft_limit_usd.is_none());
        assert!(request.api_key_ids.is_none());
    }

    #[test]
    fn test_update_budget_request_full() {
        let json = r#"{
            "name": "Updated Budget",
            "description": "New description",
            "hard_limit_usd": 2000.0,
            "soft_limit_usd": 1500.0,
            "enabled": false
        }"#;

        let request: UpdateBudgetRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.name, Some("Updated Budget".to_string()));
        assert_eq!(request.hard_limit_usd, Some(2000.0));
        assert_eq!(request.enabled, Some(false));
    }

    #[test]
    fn test_update_budget_request_empty() {
        let json = r#"{}"#;

        let request: UpdateBudgetRequest = serde_json::from_str(json).unwrap();
        assert!(request.name.is_none());
        assert!(request.hard_limit_usd.is_none());
        assert!(request.enabled.is_none());
    }

    #[test]
    fn test_usage_query_params_deserialization() {
        let json = r#"{
            "api_key_id": "key-1",
            "model_id": "gpt-4",
            "from_timestamp": 1704067200,
            "to_timestamp": 1704153600,
            "limit": 100,
            "offset": 50
        }"#;

        let params: UsageQueryParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.api_key_id, Some("key-1".to_string()));
        assert_eq!(params.model_id, Some("gpt-4".to_string()));
        assert_eq!(params.limit, Some(100));
        assert_eq!(params.offset, Some(50));
    }

    #[test]
    fn test_delete_usage_params() {
        let json = r#"{"before_timestamp": 1704067200}"#;

        let params: DeleteUsageParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.before_timestamp, 1704067200);
    }

    #[test]
    fn test_check_budget_request() {
        let json = r#"{
            "api_key_id": "key-1",
            "team_id": "team-1",
            "model_id": "gpt-4",
            "estimated_cost_usd": 0.05
        }"#;

        let request: CheckBudgetRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.api_key_id, "key-1");
        assert_eq!(request.team_id, Some("team-1".to_string()));
        assert_eq!(request.estimated_cost_usd, 0.05);
    }

    #[test]
    fn test_check_budget_request_minimal() {
        let json = r#"{
            "api_key_id": "key-1",
            "estimated_cost_usd": 0.01
        }"#;

        let request: CheckBudgetRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.api_key_id, "key-1");
        assert!(request.team_id.is_none());
        assert!(request.model_id.is_none());
    }

    #[test]
    fn test_usage_record_response_serialization() {
        let response = UsageRecordResponse {
            id: "rec-1".to_string(),
            usage_type: "chat".to_string(),
            api_key_id: "key-1".to_string(),
            model_id: Some("gpt-4".to_string()),
            input_tokens: 100,
            output_tokens: 50,
            total_tokens: 150,
            cost_usd: 0.003,
            latency_ms: 500,
            success: true,
            error: None,
            timestamp: 1704067200,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"id\":\"rec-1\""));
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"total_tokens\":150"));
    }

    #[test]
    fn test_usage_aggregate_response_serialization() {
        let response = UsageAggregateResponse {
            total_requests: 1000,
            successful_requests: 990,
            failed_requests: 10,
            total_input_tokens: 100000,
            total_output_tokens: 50000,
            total_tokens: 150000,
            total_cost_usd: 3.0,
            avg_latency_ms: 250.5,
            success_rate: 0.99,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"total_requests\":1000"));
        assert!(json.contains("\"success_rate\":0.99"));
    }

    #[test]
    fn test_usage_summary_response_serialization() {
        let response = UsageSummaryResponse {
            period_start: 1704067200,
            period_end: 1704153600,
            aggregate: UsageAggregateResponse {
                total_requests: 500,
                successful_requests: 495,
                failed_requests: 5,
                total_input_tokens: 50000,
                total_output_tokens: 25000,
                total_tokens: 75000,
                total_cost_usd: 1.5,
                avg_latency_ms: 200.0,
                success_rate: 0.99,
            },
            daily: vec![],
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"period_start\":1704067200"));
        assert!(json.contains("\"aggregate\":"));
    }

    #[test]
    fn test_daily_usage_response_serialization() {
        let response = DailyUsageResponse {
            date: 1704067200,
            requests: 100,
            tokens: 15000,
            cost_usd: 0.3,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"date\":1704067200"));
        assert!(json.contains("\"requests\":100"));
    }

    #[test]
    fn test_usage_list_response_serialization() {
        let response = UsageListResponse {
            records: vec![],
            count: 0,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"records\":[]"));
        assert!(json.contains("\"count\":0"));
    }

    #[test]
    fn test_budget_response_serialization() {
        let response = BudgetResponse {
            id: "budget-1".to_string(),
            name: "Monthly Limit".to_string(),
            description: Some("Monthly spending cap".to_string()),
            period: "monthly".to_string(),
            hard_limit_usd: 1000.0,
            soft_limit_usd: Some(800.0),
            current_usage_usd: 250.0,
            remaining_usd: 750.0,
            usage_percent: 25.0,
            status: "normal".to_string(),
            scope: "all_api_keys".to_string(),
            api_key_ids: vec![],
            team_ids: vec![],
            model_ids: vec![],
            alerts: vec![],
            period_start: 1704067200,
            enabled: true,
            created_at: 1704067200,
            updated_at: 1704067200,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"id\":\"budget-1\""));
        assert!(json.contains("\"hard_limit_usd\":1000"));
        assert!(json.contains("\"usage_percent\":25"));
    }

    #[test]
    fn test_budget_alert_response_serialization() {
        let response = BudgetAlertResponse {
            threshold_percent: 80,
            triggered: true,
            triggered_at: Some(1704100000),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"threshold_percent\":80"));
        assert!(json.contains("\"triggered\":true"));
    }

    #[test]
    fn test_budget_list_response_serialization() {
        let response = BudgetListResponse { budgets: vec![] };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"budgets\":[]"));
    }

    #[test]
    fn test_check_budget_response_serialization() {
        let response = CheckBudgetResponse {
            allowed: true,
            exceeded_budgets: vec![],
            warning_budgets: vec!["budget-1".to_string()],
            estimated_cost_usd: 0.05,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"allowed\":true"));
        assert!(json.contains("\"warning_budgets\":[\"budget-1\"]"));
    }

    #[test]
    fn test_check_budget_response_exceeded() {
        let response = CheckBudgetResponse {
            allowed: false,
            exceeded_budgets: vec!["budget-1".to_string(), "budget-2".to_string()],
            warning_budgets: vec![],
            estimated_cost_usd: 100.0,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"allowed\":false"));
        assert!(json.contains("\"exceeded_budgets\":[\"budget-1\",\"budget-2\"]"));
    }
}
