//! Webhook admin API endpoints

use crate::api::state::AppState;
use crate::api::types::error::ApiError;
use crate::domain::{Webhook, WebhookDelivery, WebhookEventType, WebhookId, WebhookStatus};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Request to create a webhook
#[derive(Debug, Deserialize)]
pub struct CreateWebhookRequest {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub url: String,
    #[serde(default)]
    pub secret: Option<String>,
    pub events: Vec<WebhookEventType>,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
    #[serde(default = "default_retry_delay")]
    pub retry_delay_secs: u32,
    #[serde(default = "default_timeout")]
    pub timeout_secs: u32,
}

fn default_max_retries() -> u32 {
    3
}

fn default_retry_delay() -> u32 {
    60
}

fn default_timeout() -> u32 {
    30
}

/// Request to update a webhook
#[derive(Debug, Deserialize)]
pub struct UpdateWebhookRequest {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub url: String,
    #[serde(default)]
    pub secret: Option<String>,
    pub events: Vec<WebhookEventType>,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    pub status: WebhookStatus,
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
    #[serde(default = "default_retry_delay")]
    pub retry_delay_secs: u32,
    #[serde(default = "default_timeout")]
    pub timeout_secs: u32,
}

/// Response for a single webhook
#[derive(Debug, Serialize)]
pub struct WebhookResponse {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub url: String,
    pub has_secret: bool,
    pub events: Vec<WebhookEventType>,
    pub headers: HashMap<String, String>,
    pub status: WebhookStatus,
    pub failure_count: u32,
    pub max_retries: u32,
    pub retry_delay_secs: u32,
    pub timeout_secs: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_success_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_failure_at: Option<DateTime<Utc>>,
}

impl From<Webhook> for WebhookResponse {
    fn from(w: Webhook) -> Self {
        Self {
            id: w.id.to_string(),
            name: w.name,
            description: w.description,
            url: w.url,
            has_secret: w.secret.is_some(),
            events: w.events,
            headers: w.headers,
            status: w.status,
            failure_count: w.failure_count,
            max_retries: w.max_retries,
            retry_delay_secs: w.retry_delay_secs,
            timeout_secs: w.timeout_secs,
            created_at: w.created_at,
            updated_at: w.updated_at,
            last_success_at: w.last_success_at,
            last_failure_at: w.last_failure_at,
        }
    }
}

/// Response for webhook list
#[derive(Debug, Serialize)]
pub struct WebhooksListResponse {
    pub webhooks: Vec<WebhookResponse>,
}

/// Response for webhook delivery
#[derive(Debug, Serialize)]
pub struct WebhookDeliveryResponse {
    pub id: String,
    pub webhook_id: String,
    pub event_type: WebhookEventType,
    pub status: crate::domain::DeliveryStatus,
    pub attempts: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_status: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_body: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_attempt_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_retry_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<DateTime<Utc>>,
}

impl From<WebhookDelivery> for WebhookDeliveryResponse {
    fn from(d: WebhookDelivery) -> Self {
        Self {
            id: d.id.to_string(),
            webhook_id: d.webhook_id.to_string(),
            event_type: d.event_type,
            status: d.status,
            attempts: d.attempts,
            response_status: d.response_status,
            response_body: d.response_body,
            error_message: d.error_message,
            created_at: d.created_at,
            last_attempt_at: d.last_attempt_at,
            next_retry_at: d.next_retry_at,
            completed_at: d.completed_at,
        }
    }
}

/// Response for delivery list
#[derive(Debug, Serialize)]
pub struct DeliveriesListResponse {
    pub deliveries: Vec<WebhookDeliveryResponse>,
}

/// Query parameters for deliveries
#[derive(Debug, Deserialize)]
pub struct DeliveriesQuery {
    #[serde(default = "default_limit")]
    pub limit: usize,
    #[serde(default)]
    pub offset: usize,
}

fn default_limit() -> usize {
    50
}

/// Response for event types
#[derive(Debug, Serialize)]
pub struct EventTypesResponse {
    pub event_types: Vec<EventTypeInfo>,
}

#[derive(Debug, Serialize)]
pub struct EventTypeInfo {
    pub name: WebhookEventType,
    pub description: String,
}

/// List all webhooks
pub async fn list_webhooks(State(state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
    let webhooks = state.webhook_service().list().await?;

    Ok(Json(WebhooksListResponse {
        webhooks: webhooks.into_iter().map(WebhookResponse::from).collect(),
    }))
}

/// Get a webhook by ID
pub async fn get_webhook(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let webhook = state.webhook_service().get(&id).await?;
    Ok(Json(WebhookResponse::from(webhook)))
}

/// Create a new webhook
pub async fn create_webhook(
    State(state): State<AppState>,
    Json(req): Json<CreateWebhookRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let mut webhook = Webhook::new(WebhookId::new(&req.id), &req.name, &req.url)
        .with_events(req.events)
        .with_retry_config(req.max_retries, req.retry_delay_secs)
        .with_timeout(req.timeout_secs);

    if let Some(desc) = req.description {
        webhook = webhook.with_description(desc);
    }

    if let Some(secret) = req.secret {
        webhook = webhook.with_secret(secret);
    }

    for (key, value) in req.headers {
        webhook = webhook.with_header(key, value);
    }

    let created = state.webhook_service().create(webhook).await?;
    Ok((StatusCode::CREATED, Json(WebhookResponse::from(created))))
}

/// Update an existing webhook
pub async fn update_webhook(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateWebhookRequest>,
) -> Result<impl IntoResponse, ApiError> {
    // Get existing to preserve certain fields
    let existing = state.webhook_service().get(&id).await?;

    let mut webhook = Webhook::new(existing.id, &req.name, &req.url)
        .with_events(req.events)
        .with_status(req.status)
        .with_retry_config(req.max_retries, req.retry_delay_secs)
        .with_timeout(req.timeout_secs);

    if let Some(desc) = req.description {
        webhook = webhook.with_description(desc);
    }

    // Preserve secret if not provided in update
    if let Some(secret) = req.secret {
        webhook = webhook.with_secret(secret);
    } else if existing.secret.is_some() {
        webhook.secret = existing.secret;
    }

    for (key, value) in req.headers {
        webhook = webhook.with_header(key, value);
    }

    // Preserve failure tracking
    webhook.failure_count = existing.failure_count;
    webhook.last_success_at = existing.last_success_at;
    webhook.last_failure_at = existing.last_failure_at;

    let updated = state.webhook_service().update(&id, webhook).await?;
    Ok(Json(WebhookResponse::from(updated)))
}

/// Delete a webhook
pub async fn delete_webhook(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    state.webhook_service().delete(&id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Reset a webhook's failure count and re-enable it
pub async fn reset_webhook(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let webhook = state.webhook_service().reset_webhook(&id).await?;
    Ok(Json(WebhookResponse::from(webhook)))
}

/// Get delivery history for a webhook
pub async fn get_deliveries(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<DeliveriesQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let deliveries = state
        .webhook_service()
        .get_deliveries(&id, query.limit, query.offset)
        .await?;

    Ok(Json(DeliveriesListResponse {
        deliveries: deliveries
            .into_iter()
            .map(WebhookDeliveryResponse::from)
            .collect(),
    }))
}

/// List available event types
pub async fn list_event_types() -> impl IntoResponse {
    let event_types: Vec<EventTypeInfo> = WebhookEventType::all()
        .into_iter()
        .map(|e| EventTypeInfo {
            name: e,
            description: match e {
                WebhookEventType::BudgetAlert => {
                    "Triggered when a budget threshold is reached".to_string()
                }
                WebhookEventType::BudgetExceeded => {
                    "Triggered when a budget limit is exceeded".to_string()
                }
                WebhookEventType::ExperimentCompleted => {
                    "Triggered when an A/B experiment is completed".to_string()
                }
                WebhookEventType::WorkflowFailed => {
                    "Triggered when a workflow execution fails".to_string()
                }
                WebhookEventType::WorkflowSucceeded => {
                    "Triggered when a workflow execution succeeds".to_string()
                }
                WebhookEventType::ModelFailed => {
                    "Triggered when a model execution fails".to_string()
                }
                WebhookEventType::ApiKeySuspended => {
                    "Triggered when an API key is suspended".to_string()
                }
                WebhookEventType::ApiKeyRevoked => {
                    "Triggered when an API key is revoked".to_string()
                }
                WebhookEventType::TestCaseFailed => {
                    "Triggered when a test case execution fails".to_string()
                }
            },
        })
        .collect();

    Json(EventTypesResponse { event_types })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_webhook_response_from() {
        let webhook = Webhook::new("hook-1", "Test Hook", "https://example.com/webhook")
            .with_event(WebhookEventType::BudgetAlert)
            .with_secret("secret123");

        let response = WebhookResponse::from(webhook);

        assert_eq!(response.id, "hook-1");
        assert_eq!(response.name, "Test Hook");
        assert!(response.has_secret);
        assert_eq!(response.events.len(), 1);
    }

    #[test]
    fn test_webhook_response_without_secret() {
        let webhook = Webhook::new("hook-2", "No Secret", "https://example.com/hook");

        let response = WebhookResponse::from(webhook);

        assert_eq!(response.id, "hook-2");
        assert!(!response.has_secret);
    }

    #[test]
    fn test_event_type_descriptions() {
        let all = WebhookEventType::all();
        assert!(!all.is_empty());

        for event in all {
            let _ = event.as_str();
        }
    }

    #[test]
    fn test_default_max_retries() {
        assert_eq!(default_max_retries(), 3);
    }

    #[test]
    fn test_default_retry_delay() {
        assert_eq!(default_retry_delay(), 60);
    }

    #[test]
    fn test_default_timeout() {
        assert_eq!(default_timeout(), 30);
    }

    #[test]
    fn test_default_limit() {
        assert_eq!(default_limit(), 50);
    }

    #[test]
    fn test_create_webhook_request_deserialization_minimal() {
        let json = r#"{
            "id": "test-hook",
            "name": "Test",
            "url": "https://example.com",
            "events": ["budget_alert"]
        }"#;

        let request: CreateWebhookRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.id, "test-hook");
        assert_eq!(request.name, "Test");
        assert_eq!(request.url, "https://example.com");
        assert_eq!(request.events.len(), 1);
        assert_eq!(request.max_retries, 3);
        assert_eq!(request.retry_delay_secs, 60);
        assert_eq!(request.timeout_secs, 30);
    }

    #[test]
    fn test_create_webhook_request_deserialization_full() {
        let json = r#"{
            "id": "full-hook",
            "name": "Full Hook",
            "description": "A webhook with all fields",
            "url": "https://example.com/webhook",
            "secret": "mysecret",
            "events": ["budget_alert", "budget_exceeded"],
            "headers": {"X-Custom": "value"},
            "max_retries": 5,
            "retry_delay_secs": 120,
            "timeout_secs": 60
        }"#;

        let request: CreateWebhookRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.id, "full-hook");
        assert_eq!(request.description, Some("A webhook with all fields".to_string()));
        assert_eq!(request.secret, Some("mysecret".to_string()));
        assert_eq!(request.events.len(), 2);
        assert_eq!(request.headers.get("X-Custom"), Some(&"value".to_string()));
        assert_eq!(request.max_retries, 5);
    }

    #[test]
    fn test_update_webhook_request_deserialization() {
        let json = r#"{
            "name": "Updated",
            "url": "https://new-url.com",
            "events": ["workflow_failed"],
            "status": "active"
        }"#;

        let request: UpdateWebhookRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.name, "Updated");
        assert_eq!(request.url, "https://new-url.com");
        assert_eq!(request.events.len(), 1);
        assert_eq!(request.status, WebhookStatus::Active);
    }

    #[test]
    fn test_deliveries_query_defaults() {
        let json = r#"{}"#;

        let query: DeliveriesQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.limit, 50);
        assert_eq!(query.offset, 0);
    }

    #[test]
    fn test_deliveries_query_custom() {
        let json = r#"{"limit": 10, "offset": 20}"#;

        let query: DeliveriesQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.limit, 10);
        assert_eq!(query.offset, 20);
    }

    #[test]
    fn test_webhook_response_serialization() {
        let webhook = Webhook::new("hook-1", "Test", "https://example.com")
            .with_event(WebhookEventType::BudgetAlert);

        let response = WebhookResponse::from(webhook);
        let json = serde_json::to_string(&response).unwrap();

        assert!(json.contains("\"id\":\"hook-1\""));
        assert!(json.contains("\"name\":\"Test\""));
        assert!(json.contains("\"url\":\"https://example.com\""));
        assert!(json.contains("\"has_secret\":false"));
    }

    #[test]
    fn test_webhooks_list_response_serialization() {
        let list_response = WebhooksListResponse { webhooks: vec![] };

        let json = serde_json::to_string(&list_response).unwrap();
        assert!(json.contains("\"webhooks\":[]"));
    }

    #[test]
    fn test_event_types_response_serialization() {
        let response = EventTypesResponse {
            event_types: vec![EventTypeInfo {
                name: WebhookEventType::BudgetAlert,
                description: "Test description".to_string(),
            }],
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"event_types\""));
        assert!(json.contains("\"name\":\"budget_alert\""));
        assert!(json.contains("\"description\":\"Test description\""));
    }
}
