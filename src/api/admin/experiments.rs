//! Experiment (A/B Testing) management admin endpoints

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::api::middleware::RequireAdmin;
use crate::api::state::AppState;
use crate::api::types::ApiError;
use crate::domain::experiment::{
    Experiment, ExperimentQuery, ExperimentResult, ExperimentStatus, LatencyStats,
    StatisticalSignificance, VariantConfig, VariantMetrics,
};
use crate::infrastructure::services::{
    CreateExperimentRequest, CreateVariantRequest, UpdateExperimentRequest,
};

// ============================================================================
// Request Types
// ============================================================================

/// Request to create a new experiment
#[derive(Debug, Clone, Deserialize)]
pub struct CreateExperimentApiRequest {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub variants: Vec<CreateVariantApiRequest>,
    #[serde(default)]
    pub traffic_allocation: Vec<TrafficAllocationRequest>,
}

/// Request to update an experiment
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateExperimentApiRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub traffic_allocation: Option<Vec<TrafficAllocationRequest>>,
    pub enabled: Option<bool>,
}

/// Request to create a variant
#[derive(Debug, Clone, Deserialize)]
pub struct CreateVariantApiRequest {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub config: VariantConfigRequest,
    #[serde(default)]
    pub is_control: bool,
}

/// Variant configuration request
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum VariantConfigRequest {
    ModelReference { model_id: String },
    ConfigOverride {
        model_id: String,
        #[serde(default)]
        temperature: Option<f32>,
        #[serde(default)]
        max_tokens: Option<u32>,
        #[serde(default)]
        top_p: Option<f32>,
        #[serde(default)]
        presence_penalty: Option<f32>,
        #[serde(default)]
        frequency_penalty: Option<f32>,
    },
}

/// Traffic allocation request
#[derive(Debug, Clone, Deserialize)]
pub struct TrafficAllocationRequest {
    pub variant_id: String,
    pub percentage: u8,
}

/// Query parameters for listing experiments
#[derive(Debug, Clone, Deserialize, Default)]
pub struct ListExperimentsQuery {
    pub status: Option<String>,
    pub model_id: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

// ============================================================================
// Response Types
// ============================================================================

/// Experiment response
#[derive(Debug, Clone, Serialize)]
pub struct ExperimentResponse {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub status: String,
    pub variants: Vec<VariantResponse>,
    pub traffic_allocation: Vec<TrafficAllocationResponse>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub enabled: bool,
}

/// Variant response
#[derive(Debug, Clone, Serialize)]
pub struct VariantResponse {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub config: VariantConfigResponse,
    pub is_control: bool,
}

/// Variant configuration response
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum VariantConfigResponse {
    ModelReference { model_id: String },
    ConfigOverride {
        model_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        temperature: Option<f32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        max_tokens: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        top_p: Option<f32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        presence_penalty: Option<f32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        frequency_penalty: Option<f32>,
    },
}

/// Traffic allocation response
#[derive(Debug, Clone, Serialize)]
pub struct TrafficAllocationResponse {
    pub variant_id: String,
    pub percentage: u8,
}

/// List experiments response
#[derive(Debug, Clone, Serialize)]
pub struct ListExperimentsResponse {
    pub experiments: Vec<ExperimentResponse>,
    pub total: usize,
}

/// Experiment results response
#[derive(Debug, Clone, Serialize)]
pub struct ExperimentResultsResponse {
    pub experiment_id: String,
    pub experiment_name: String,
    pub status: String,
    pub duration_hours: Option<f64>,
    pub total_requests: u64,
    pub variant_metrics: Vec<VariantMetricsResponse>,
    pub significance_tests: Vec<SignificanceResponse>,
    pub winner_variant_id: Option<String>,
    pub recommendation: Option<String>,
}

/// Variant metrics response
#[derive(Debug, Clone, Serialize)]
pub struct VariantMetricsResponse {
    pub variant_id: String,
    pub variant_name: String,
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub success_rate: f64,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_tokens: u64,
    pub total_cost_micros: i64,
    pub avg_cost_micros: f64,
    pub latency: LatencyStatsResponse,
}

/// Latency stats response
#[derive(Debug, Clone, Serialize)]
pub struct LatencyStatsResponse {
    pub avg_ms: f64,
    pub min_ms: u64,
    pub max_ms: u64,
    pub p50_ms: u64,
    pub p95_ms: u64,
    pub p99_ms: u64,
}

/// Statistical significance response
#[derive(Debug, Clone, Serialize)]
pub struct SignificanceResponse {
    pub metric: String,
    pub control_variant_id: String,
    pub treatment_variant_id: String,
    pub control_mean: f64,
    pub treatment_mean: f64,
    pub relative_change: f64,
    pub p_value: f64,
    pub is_significant: bool,
    pub confidence_level: f64,
}

// ============================================================================
// Conversion Implementations
// ============================================================================

fn status_to_string(status: &ExperimentStatus) -> String {
    match status {
        ExperimentStatus::Draft => "draft".to_string(),
        ExperimentStatus::Active => "active".to_string(),
        ExperimentStatus::Paused => "paused".to_string(),
        ExperimentStatus::Completed => "completed".to_string(),
    }
}

fn parse_status(s: &str) -> Result<ExperimentStatus, ApiError> {
    match s.to_lowercase().as_str() {
        "draft" => Ok(ExperimentStatus::Draft),
        "active" => Ok(ExperimentStatus::Active),
        "paused" => Ok(ExperimentStatus::Paused),
        "completed" => Ok(ExperimentStatus::Completed),
        other => Err(ApiError::bad_request(format!(
            "Invalid status '{}'. Valid values: draft, active, paused, completed",
            other
        ))),
    }
}

impl From<&VariantConfig> for VariantConfigResponse {
    fn from(config: &VariantConfig) -> Self {
        match config {
            VariantConfig::ModelReference { model_id } => VariantConfigResponse::ModelReference {
                model_id: model_id.clone(),
            },
            VariantConfig::ConfigOverride {
                model_id,
                temperature,
                max_tokens,
                top_p,
                presence_penalty,
                frequency_penalty,
            } => VariantConfigResponse::ConfigOverride {
                model_id: model_id.clone(),
                temperature: *temperature,
                max_tokens: *max_tokens,
                top_p: *top_p,
                presence_penalty: *presence_penalty,
                frequency_penalty: *frequency_penalty,
            },
        }
    }
}

impl From<&Experiment> for ExperimentResponse {
    fn from(experiment: &Experiment) -> Self {
        Self {
            id: experiment.id().as_str().to_string(),
            name: experiment.name().to_string(),
            description: experiment.description().map(|s| s.to_string()),
            status: status_to_string(&experiment.status()),
            variants: experiment
                .variants()
                .iter()
                .map(|v| VariantResponse {
                    id: v.id().as_str().to_string(),
                    name: v.name().to_string(),
                    description: v.description().map(|s| s.to_string()),
                    config: VariantConfigResponse::from(v.config()),
                    is_control: v.is_control(),
                })
                .collect(),
            traffic_allocation: experiment
                .traffic_allocation()
                .iter()
                .map(|t| TrafficAllocationResponse {
                    variant_id: t.variant_id().as_str().to_string(),
                    percentage: t.percentage(),
                })
                .collect(),
            started_at: experiment.started_at().map(|t| t.to_rfc3339()),
            completed_at: experiment.completed_at().map(|t| t.to_rfc3339()),
            created_at: experiment.created_at().to_rfc3339(),
            updated_at: experiment.updated_at().to_rfc3339(),
            enabled: experiment.is_enabled(),
        }
    }
}

impl From<&LatencyStats> for LatencyStatsResponse {
    fn from(stats: &LatencyStats) -> Self {
        Self {
            avg_ms: stats.avg_ms,
            min_ms: stats.min_ms,
            max_ms: stats.max_ms,
            p50_ms: stats.p50_ms,
            p95_ms: stats.p95_ms,
            p99_ms: stats.p99_ms,
        }
    }
}

impl From<&VariantMetrics> for VariantMetricsResponse {
    fn from(metrics: &VariantMetrics) -> Self {
        Self {
            variant_id: metrics.variant_id.clone(),
            variant_name: metrics.variant_name.clone(),
            total_requests: metrics.total_requests,
            successful_requests: metrics.successful_requests,
            failed_requests: metrics.failed_requests,
            success_rate: metrics.success_rate,
            total_input_tokens: metrics.total_input_tokens,
            total_output_tokens: metrics.total_output_tokens,
            total_tokens: metrics.total_tokens,
            total_cost_micros: metrics.total_cost_micros,
            avg_cost_micros: metrics.avg_cost_micros,
            latency: LatencyStatsResponse::from(&metrics.latency),
        }
    }
}

impl From<&StatisticalSignificance> for SignificanceResponse {
    fn from(sig: &StatisticalSignificance) -> Self {
        Self {
            metric: sig.metric.clone(),
            control_variant_id: sig.control_variant_id.clone(),
            treatment_variant_id: sig.treatment_variant_id.clone(),
            control_mean: sig.control_mean,
            treatment_mean: sig.treatment_mean,
            relative_change: sig.relative_change,
            p_value: sig.p_value,
            is_significant: sig.is_significant,
            confidence_level: sig.confidence_level,
        }
    }
}

impl From<&ExperimentResult> for ExperimentResultsResponse {
    fn from(result: &ExperimentResult) -> Self {
        Self {
            experiment_id: result.experiment_id.clone(),
            experiment_name: result.experiment_name.clone(),
            status: status_to_string(&result.status),
            duration_hours: result.duration_hours,
            total_requests: result.total_requests,
            variant_metrics: result
                .variant_metrics
                .iter()
                .map(VariantMetricsResponse::from)
                .collect(),
            significance_tests: result
                .significance_tests
                .iter()
                .map(SignificanceResponse::from)
                .collect(),
            winner_variant_id: result.winner_variant_id.clone(),
            recommendation: result.recommendation.clone(),
        }
    }
}

fn build_variant_config(request: &VariantConfigRequest) -> VariantConfig {
    match request {
        VariantConfigRequest::ModelReference { model_id } => VariantConfig::ModelReference {
            model_id: model_id.clone(),
        },
        VariantConfigRequest::ConfigOverride {
            model_id,
            temperature,
            max_tokens,
            top_p,
            presence_penalty,
            frequency_penalty,
        } => VariantConfig::ConfigOverride {
            model_id: model_id.clone(),
            temperature: *temperature,
            max_tokens: *max_tokens,
            top_p: *top_p,
            presence_penalty: *presence_penalty,
            frequency_penalty: *frequency_penalty,
        },
    }
}

// ============================================================================
// Handlers
// ============================================================================

/// GET /admin/experiments
pub async fn list_experiments(
    State(state): State<AppState>,
    RequireAdmin(_): RequireAdmin,
    Query(params): Query<ListExperimentsQuery>,
) -> Result<Json<ListExperimentsResponse>, ApiError> {
    debug!("Admin listing experiments");

    let mut query = ExperimentQuery::new();

    if let Some(ref status_str) = params.status {
        let status = parse_status(status_str)?;
        query = query.with_status(status);
    }

    if let Some(ref model_id) = params.model_id {
        query = query.with_model(model_id);
    }

    if let Some(limit) = params.limit {
        query = query.with_limit(limit);
    }

    if let Some(offset) = params.offset {
        query = query.with_offset(offset);
    }

    let experiments = state
        .experiment_service
        .list(Some(query))
        .await
        .map_err(ApiError::from)?;

    let responses: Vec<ExperimentResponse> =
        experiments.iter().map(ExperimentResponse::from).collect();
    let total = responses.len();

    Ok(Json(ListExperimentsResponse {
        experiments: responses,
        total,
    }))
}

/// POST /admin/experiments
pub async fn create_experiment(
    State(state): State<AppState>,
    RequireAdmin(_): RequireAdmin,
    Json(request): Json<CreateExperimentApiRequest>,
) -> Result<Json<ExperimentResponse>, ApiError> {
    debug!(experiment_id = %request.id, "Admin creating experiment");

    let variants: Vec<CreateVariantRequest> = request
        .variants
        .iter()
        .map(|v| CreateVariantRequest {
            id: v.id.clone(),
            name: v.name.clone(),
            description: v.description.clone(),
            config: build_variant_config(&v.config),
            control: v.is_control,
        })
        .collect();

    let traffic_allocation: Vec<(String, u8)> = request
        .traffic_allocation
        .iter()
        .map(|t| (t.variant_id.clone(), t.percentage))
        .collect();

    let create_request = CreateExperimentRequest {
        id: request.id,
        name: request.name,
        description: request.description,
        variants,
        traffic_allocation,
        enabled: true,
    };

    let experiment = state
        .experiment_service
        .create(create_request)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(ExperimentResponse::from(&experiment)))
}

/// GET /admin/experiments/:id
pub async fn get_experiment(
    State(state): State<AppState>,
    RequireAdmin(_): RequireAdmin,
    Path(experiment_id): Path<String>,
) -> Result<Json<ExperimentResponse>, ApiError> {
    debug!(experiment_id = %experiment_id, "Admin getting experiment");

    let experiment = state
        .experiment_service
        .get(&experiment_id)
        .await
        .map_err(ApiError::from)?
        .ok_or_else(|| {
            ApiError::not_found(format!("Experiment '{}' not found", experiment_id))
        })?;

    Ok(Json(ExperimentResponse::from(&experiment)))
}

/// PUT /admin/experiments/:id
pub async fn update_experiment(
    State(state): State<AppState>,
    RequireAdmin(_): RequireAdmin,
    Path(experiment_id): Path<String>,
    Json(request): Json<UpdateExperimentApiRequest>,
) -> Result<Json<ExperimentResponse>, ApiError> {
    debug!(experiment_id = %experiment_id, "Admin updating experiment");

    let traffic_allocation = request.traffic_allocation.as_ref().map(|allocations| {
        allocations
            .iter()
            .map(|t| (t.variant_id.clone(), t.percentage))
            .collect()
    });

    let update_request = UpdateExperimentRequest {
        name: request.name,
        description: request.description.map(Some),
        variants: None,
        traffic_allocation,
        enabled: request.enabled,
    };

    let experiment = state
        .experiment_service
        .update(&experiment_id, update_request)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(ExperimentResponse::from(&experiment)))
}

/// DELETE /admin/experiments/:id
pub async fn delete_experiment(
    State(state): State<AppState>,
    RequireAdmin(_): RequireAdmin,
    Path(experiment_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    debug!(experiment_id = %experiment_id, "Admin deleting experiment");

    state
        .experiment_service
        .delete(&experiment_id)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(serde_json::json!({
        "deleted": true,
        "id": experiment_id
    })))
}

/// POST /admin/experiments/:id/variants
pub async fn add_variant(
    State(state): State<AppState>,
    RequireAdmin(_): RequireAdmin,
    Path(experiment_id): Path<String>,
    Json(request): Json<CreateVariantApiRequest>,
) -> Result<Json<ExperimentResponse>, ApiError> {
    debug!(
        experiment_id = %experiment_id,
        variant_id = %request.id,
        "Admin adding variant to experiment"
    );

    let variant_request = CreateVariantRequest {
        id: request.id,
        name: request.name,
        description: request.description,
        config: build_variant_config(&request.config),
        control: request.is_control,
    };

    let experiment = state
        .experiment_service
        .add_variant(&experiment_id, variant_request)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(ExperimentResponse::from(&experiment)))
}

/// DELETE /admin/experiments/:id/variants/:variant_id
pub async fn remove_variant(
    State(state): State<AppState>,
    RequireAdmin(_): RequireAdmin,
    Path((experiment_id, variant_id)): Path<(String, String)>,
) -> Result<Json<ExperimentResponse>, ApiError> {
    debug!(
        experiment_id = %experiment_id,
        variant_id = %variant_id,
        "Admin removing variant from experiment"
    );

    let experiment = state
        .experiment_service
        .remove_variant(&experiment_id, &variant_id)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(ExperimentResponse::from(&experiment)))
}

/// POST /admin/experiments/:id/start
pub async fn start_experiment(
    State(state): State<AppState>,
    RequireAdmin(_): RequireAdmin,
    Path(experiment_id): Path<String>,
) -> Result<Json<ExperimentResponse>, ApiError> {
    debug!(experiment_id = %experiment_id, "Admin starting experiment");

    let experiment = state
        .experiment_service
        .start(&experiment_id)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(ExperimentResponse::from(&experiment)))
}

/// POST /admin/experiments/:id/pause
pub async fn pause_experiment(
    State(state): State<AppState>,
    RequireAdmin(_): RequireAdmin,
    Path(experiment_id): Path<String>,
) -> Result<Json<ExperimentResponse>, ApiError> {
    debug!(experiment_id = %experiment_id, "Admin pausing experiment");

    let experiment = state
        .experiment_service
        .pause(&experiment_id)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(ExperimentResponse::from(&experiment)))
}

/// POST /admin/experiments/:id/resume
pub async fn resume_experiment(
    State(state): State<AppState>,
    RequireAdmin(_): RequireAdmin,
    Path(experiment_id): Path<String>,
) -> Result<Json<ExperimentResponse>, ApiError> {
    debug!(experiment_id = %experiment_id, "Admin resuming experiment");

    let experiment = state
        .experiment_service
        .resume(&experiment_id)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(ExperimentResponse::from(&experiment)))
}

/// POST /admin/experiments/:id/complete
pub async fn complete_experiment(
    State(state): State<AppState>,
    RequireAdmin(_): RequireAdmin,
    Path(experiment_id): Path<String>,
) -> Result<Json<ExperimentResponse>, ApiError> {
    debug!(experiment_id = %experiment_id, "Admin completing experiment");

    let experiment = state
        .experiment_service
        .complete(&experiment_id)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(ExperimentResponse::from(&experiment)))
}

/// GET /admin/experiments/:id/results
pub async fn get_experiment_results(
    State(state): State<AppState>,
    RequireAdmin(_): RequireAdmin,
    Path(experiment_id): Path<String>,
) -> Result<Json<ExperimentResultsResponse>, ApiError> {
    debug!(experiment_id = %experiment_id, "Admin getting experiment results");

    let results = state
        .experiment_service
        .get_results(&experiment_id)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(ExperimentResultsResponse::from(&results)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_experiment_request_deserialization() {
        let json = r#"{
            "id": "model-comparison",
            "name": "GPT-4 vs Claude 3.5",
            "description": "Compare GPT-4 and Claude 3.5 Sonnet performance",
            "variants": [
                {
                    "id": "control",
                    "name": "GPT-4 (Control)",
                    "config": {
                        "type": "model_reference",
                        "model_id": "gpt-4"
                    },
                    "is_control": true
                },
                {
                    "id": "treatment",
                    "name": "Claude 3.5 Sonnet",
                    "config": {
                        "type": "model_reference",
                        "model_id": "claude-3-5-sonnet"
                    },
                    "is_control": false
                }
            ],
            "traffic_allocation": [
                {"variant_id": "control", "percentage": 50},
                {"variant_id": "treatment", "percentage": 50}
            ]
        }"#;

        let request: CreateExperimentApiRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.id, "model-comparison");
        assert_eq!(request.name, "GPT-4 vs Claude 3.5");
        assert_eq!(request.variants.len(), 2);
        assert_eq!(request.traffic_allocation.len(), 2);
    }

    #[test]
    fn test_create_experiment_request_minimal() {
        let json = r#"{
            "id": "test-exp",
            "name": "Test Experiment"
        }"#;

        let request: CreateExperimentApiRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.id, "test-exp");
        assert!(request.description.is_none());
        assert!(request.variants.is_empty());
        assert!(request.traffic_allocation.is_empty());
    }

    #[test]
    fn test_update_experiment_request_full() {
        let json = r#"{
            "name": "Updated Name",
            "description": "New description",
            "traffic_allocation": [
                {"variant_id": "v1", "percentage": 70},
                {"variant_id": "v2", "percentage": 30}
            ],
            "enabled": false
        }"#;

        let request: UpdateExperimentApiRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.name, Some("Updated Name".to_string()));
        assert_eq!(request.description, Some("New description".to_string()));
        assert!(request.traffic_allocation.is_some());
        assert_eq!(request.enabled, Some(false));
    }

    #[test]
    fn test_update_experiment_request_empty() {
        let json = r#"{}"#;

        let request: UpdateExperimentApiRequest = serde_json::from_str(json).unwrap();
        assert!(request.name.is_none());
        assert!(request.description.is_none());
        assert!(request.traffic_allocation.is_none());
        assert!(request.enabled.is_none());
    }

    #[test]
    fn test_variant_config_override_deserialization() {
        let json = r#"{
            "id": "high-temp",
            "name": "High Temperature",
            "config": {
                "type": "config_override",
                "model_id": "gpt-4",
                "temperature": 0.9,
                "max_tokens": 2000
            },
            "is_control": false
        }"#;

        let request: CreateVariantApiRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.id, "high-temp");

        match request.config {
            VariantConfigRequest::ConfigOverride {
                model_id,
                temperature,
                max_tokens,
                ..
            } => {
                assert_eq!(model_id, "gpt-4");
                assert_eq!(temperature, Some(0.9));
                assert_eq!(max_tokens, Some(2000));
            }
            _ => panic!("Expected ConfigOverride"),
        }
    }

    #[test]
    fn test_variant_config_model_reference() {
        let json = r#"{
            "id": "simple",
            "name": "Simple Variant",
            "config": {
                "type": "model_reference",
                "model_id": "gpt-4"
            }
        }"#;

        let request: CreateVariantApiRequest = serde_json::from_str(json).unwrap();
        assert!(!request.is_control);

        match request.config {
            VariantConfigRequest::ModelReference { model_id } => {
                assert_eq!(model_id, "gpt-4");
            }
            _ => panic!("Expected ModelReference"),
        }
    }

    #[test]
    fn test_traffic_allocation_request() {
        let json = r#"{"variant_id": "v1", "percentage": 60}"#;

        let request: TrafficAllocationRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.variant_id, "v1");
        assert_eq!(request.percentage, 60);
    }

    #[test]
    fn test_list_experiments_query_defaults() {
        let query = ListExperimentsQuery::default();
        assert!(query.status.is_none());
        assert!(query.model_id.is_none());
        assert!(query.limit.is_none());
        assert!(query.offset.is_none());
    }

    #[test]
    fn test_parse_status() {
        assert!(matches!(parse_status("draft").unwrap(), ExperimentStatus::Draft));
        assert!(matches!(parse_status("active").unwrap(), ExperimentStatus::Active));
        assert!(matches!(parse_status("paused").unwrap(), ExperimentStatus::Paused));
        assert!(matches!(parse_status("completed").unwrap(), ExperimentStatus::Completed));
        assert!(parse_status("invalid").is_err());
    }

    #[test]
    fn test_parse_status_case_insensitive() {
        assert!(matches!(parse_status("DRAFT").unwrap(), ExperimentStatus::Draft));
        assert!(matches!(parse_status("Active").unwrap(), ExperimentStatus::Active));
        assert!(matches!(parse_status("PAUSED").unwrap(), ExperimentStatus::Paused));
    }

    #[test]
    fn test_status_to_string() {
        assert_eq!(status_to_string(&ExperimentStatus::Draft), "draft");
        assert_eq!(status_to_string(&ExperimentStatus::Active), "active");
        assert_eq!(status_to_string(&ExperimentStatus::Paused), "paused");
        assert_eq!(status_to_string(&ExperimentStatus::Completed), "completed");
    }

    #[test]
    fn test_build_variant_config_model_reference() {
        let request = VariantConfigRequest::ModelReference {
            model_id: "gpt-4".to_string(),
        };

        let config = build_variant_config(&request);

        if let VariantConfig::ModelReference { model_id } = config {
            assert_eq!(model_id, "gpt-4");
        } else {
            panic!("Expected ModelReference");
        }
    }

    #[test]
    fn test_build_variant_config_override() {
        let request = VariantConfigRequest::ConfigOverride {
            model_id: "gpt-4".to_string(),
            temperature: Some(0.8),
            max_tokens: Some(1000),
            top_p: None,
            presence_penalty: None,
            frequency_penalty: None,
        };

        let config = build_variant_config(&request);

        if let VariantConfig::ConfigOverride { model_id, temperature, max_tokens, .. } = config {
            assert_eq!(model_id, "gpt-4");
            assert_eq!(temperature, Some(0.8));
            assert_eq!(max_tokens, Some(1000));
        } else {
            panic!("Expected ConfigOverride");
        }
    }

    #[test]
    fn test_variant_config_response_from_model_reference() {
        let config = VariantConfig::ModelReference {
            model_id: "claude-3".to_string(),
        };

        let response = VariantConfigResponse::from(&config);

        if let VariantConfigResponse::ModelReference { model_id } = response {
            assert_eq!(model_id, "claude-3");
        } else {
            panic!("Expected ModelReference");
        }
    }

    #[test]
    fn test_variant_config_response_from_override() {
        let config = VariantConfig::ConfigOverride {
            model_id: "gpt-4".to_string(),
            temperature: Some(0.7),
            max_tokens: Some(500),
            top_p: None,
            presence_penalty: None,
            frequency_penalty: None,
        };

        let response = VariantConfigResponse::from(&config);

        if let VariantConfigResponse::ConfigOverride { model_id, temperature, .. } = response {
            assert_eq!(model_id, "gpt-4");
            assert_eq!(temperature, Some(0.7));
        } else {
            panic!("Expected ConfigOverride");
        }
    }

    #[test]
    fn test_latency_stats_response_from() {
        let stats = LatencyStats {
            avg_ms: 150.5,
            min_ms: 100,
            max_ms: 300,
            p50_ms: 140,
            p95_ms: 250,
            p99_ms: 290,
        };

        let response = LatencyStatsResponse::from(&stats);

        assert_eq!(response.avg_ms, 150.5);
        assert_eq!(response.min_ms, 100);
        assert_eq!(response.max_ms, 300);
        assert_eq!(response.p50_ms, 140);
        assert_eq!(response.p95_ms, 250);
        assert_eq!(response.p99_ms, 290);
    }

    #[test]
    fn test_experiment_response_serialization() {
        let response = ExperimentResponse {
            id: "exp-1".to_string(),
            name: "Test Experiment".to_string(),
            description: Some("A test".to_string()),
            status: "active".to_string(),
            variants: vec![],
            traffic_allocation: vec![],
            started_at: None,
            completed_at: None,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
            enabled: true,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"id\":\"exp-1\""));
        assert!(json.contains("\"status\":\"active\""));
        assert!(json.contains("\"enabled\":true"));
    }

    #[test]
    fn test_list_experiments_response_serialization() {
        let response = ListExperimentsResponse {
            experiments: vec![],
            total: 0,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"experiments\":[]"));
        assert!(json.contains("\"total\":0"));
    }

    #[test]
    fn test_variant_response_serialization() {
        let response = VariantResponse {
            id: "v1".to_string(),
            name: "Control".to_string(),
            description: None,
            config: VariantConfigResponse::ModelReference {
                model_id: "gpt-4".to_string(),
            },
            is_control: true,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"id\":\"v1\""));
        assert!(json.contains("\"is_control\":true"));
        assert!(json.contains("\"type\":\"model_reference\""));
    }

    #[test]
    fn test_traffic_allocation_response_serialization() {
        let response = TrafficAllocationResponse {
            variant_id: "v1".to_string(),
            percentage: 50,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"variant_id\":\"v1\""));
        assert!(json.contains("\"percentage\":50"));
    }

    #[test]
    fn test_significance_response_serialization() {
        let response = SignificanceResponse {
            metric: "latency".to_string(),
            control_variant_id: "control".to_string(),
            treatment_variant_id: "treatment".to_string(),
            control_mean: 150.0,
            treatment_mean: 120.0,
            relative_change: -20.0,
            p_value: 0.01,
            is_significant: true,
            confidence_level: 0.95,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"metric\":\"latency\""));
        assert!(json.contains("\"is_significant\":true"));
        assert!(json.contains("\"p_value\":0.01"));
    }

    #[test]
    fn test_experiment_results_response_serialization() {
        let response = ExperimentResultsResponse {
            experiment_id: "exp-1".to_string(),
            experiment_name: "Test".to_string(),
            status: "completed".to_string(),
            duration_hours: Some(24.5),
            total_requests: 1000,
            variant_metrics: vec![],
            significance_tests: vec![],
            winner_variant_id: Some("treatment".to_string()),
            recommendation: Some("Deploy treatment variant".to_string()),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"experiment_id\":\"exp-1\""));
        assert!(json.contains("\"total_requests\":1000"));
        assert!(json.contains("\"winner_variant_id\":\"treatment\""));
    }

    #[test]
    fn test_variant_metrics_response_serialization() {
        let response = VariantMetricsResponse {
            variant_id: "v1".to_string(),
            variant_name: "Control".to_string(),
            total_requests: 500,
            successful_requests: 490,
            failed_requests: 10,
            success_rate: 0.98,
            total_input_tokens: 10000,
            total_output_tokens: 5000,
            total_tokens: 15000,
            total_cost_micros: 50000,
            avg_cost_micros: 100.0,
            latency: LatencyStatsResponse {
                avg_ms: 150.0,
                min_ms: 50,
                max_ms: 500,
                p50_ms: 140,
                p95_ms: 300,
                p99_ms: 450,
            },
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"variant_id\":\"v1\""));
        assert!(json.contains("\"success_rate\":0.98"));
        assert!(json.contains("\"total_tokens\":15000"));
    }
}
