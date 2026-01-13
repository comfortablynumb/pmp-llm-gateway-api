//! Health check endpoints for Kubernetes probes

use std::time::Instant;

use axum::{extract::State, http::StatusCode, response::IntoResponse};

use crate::api::types::Json;
use serde::Serialize;

use super::state::AppState;

/// Detailed health response with component status
#[derive(Serialize)]
pub struct HealthResponse {
    pub status: HealthStatus,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checks: Option<Vec<HealthCheck>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u64>,
}

/// Health check status
#[derive(Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

/// Individual component health check
#[derive(Serialize)]
pub struct HealthCheck {
    pub name: String,
    pub status: HealthStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u64>,
}

/// Simple health check - returns 200 if the service is running
/// Used for basic liveness probes
pub async fn health_check() -> impl IntoResponse {
    let response = HealthResponse {
        status: HealthStatus::Healthy,
        version: env!("CARGO_PKG_VERSION").to_string(),
        checks: None,
        latency_ms: None,
    };

    (StatusCode::OK, Json(response))
}

/// Readiness check with dependency verification
/// Checks if the service can handle requests
pub async fn ready_check(State(state): State<AppState>) -> impl IntoResponse {
    let start = Instant::now();
    let mut checks = Vec::new();
    let mut overall_status = HealthStatus::Healthy;

    // Check model service connectivity
    let model_check = check_model_service(&state).await;

    if model_check.status != HealthStatus::Healthy {
        overall_status = HealthStatus::Degraded;
    }
    checks.push(model_check);

    // Check API key service connectivity
    let api_key_check = check_api_key_service(&state).await;

    if api_key_check.status != HealthStatus::Healthy {
        overall_status = HealthStatus::Degraded;
    }
    checks.push(api_key_check);

    let latency = start.elapsed().as_millis() as u64;
    let response = HealthResponse {
        status: overall_status,
        version: env!("CARGO_PKG_VERSION").to_string(),
        checks: Some(checks),
        latency_ms: Some(latency),
    };

    let status_code = match overall_status {
        HealthStatus::Healthy => StatusCode::OK,
        HealthStatus::Degraded => StatusCode::OK, // Still accept requests
        HealthStatus::Unhealthy => StatusCode::SERVICE_UNAVAILABLE,
    };

    (status_code, Json(response))
}

/// Liveness check - simple check to verify the service is running
/// Used for Kubernetes liveness probes to detect crashes
pub async fn live_check() -> impl IntoResponse {
    StatusCode::OK
}

async fn check_model_service(state: &AppState) -> HealthCheck {
    let start = Instant::now();

    match state.model_service.list().await {
        Ok(_) => HealthCheck {
            name: "model_service".to_string(),
            status: HealthStatus::Healthy,
            message: None,
            latency_ms: Some(start.elapsed().as_millis() as u64),
        },
        Err(e) => HealthCheck {
            name: "model_service".to_string(),
            status: HealthStatus::Unhealthy,
            message: Some(e.to_string()),
            latency_ms: Some(start.elapsed().as_millis() as u64),
        },
    }
}

async fn check_api_key_service(state: &AppState) -> HealthCheck {
    let start = Instant::now();

    match state.api_key_service.list().await {
        Ok(_) => HealthCheck {
            name: "api_key_service".to_string(),
            status: HealthStatus::Healthy,
            message: None,
            latency_ms: Some(start.elapsed().as_millis() as u64),
        },
        Err(e) => HealthCheck {
            name: "api_key_service".to_string(),
            status: HealthStatus::Unhealthy,
            message: Some(e.to_string()),
            latency_ms: Some(start.elapsed().as_millis() as u64),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_status_serialization() {
        assert_eq!(
            serde_json::to_string(&HealthStatus::Healthy).unwrap(),
            "\"healthy\""
        );
        assert_eq!(
            serde_json::to_string(&HealthStatus::Degraded).unwrap(),
            "\"degraded\""
        );
        assert_eq!(
            serde_json::to_string(&HealthStatus::Unhealthy).unwrap(),
            "\"unhealthy\""
        );
    }

    #[test]
    fn test_health_response_serialization() {
        let response = HealthResponse {
            status: HealthStatus::Healthy,
            version: "1.0.0".to_string(),
            checks: None,
            latency_ms: None,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"status\":\"healthy\""));
        assert!(json.contains("\"version\":\"1.0.0\""));
        assert!(!json.contains("checks"));
    }

    #[test]
    fn test_health_response_with_checks() {
        let response = HealthResponse {
            status: HealthStatus::Degraded,
            version: "1.0.0".to_string(),
            checks: Some(vec![
                HealthCheck {
                    name: "database".to_string(),
                    status: HealthStatus::Healthy,
                    message: None,
                    latency_ms: Some(5),
                },
                HealthCheck {
                    name: "cache".to_string(),
                    status: HealthStatus::Unhealthy,
                    message: Some("Connection refused".to_string()),
                    latency_ms: Some(100),
                },
            ]),
            latency_ms: Some(105),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"status\":\"degraded\""));
        assert!(json.contains("\"database\""));
        assert!(json.contains("\"cache\""));
        assert!(json.contains("\"Connection refused\""));
    }
}
