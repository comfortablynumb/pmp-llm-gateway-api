//! Webhook domain entities

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::domain::storage::{StorageEntity, StorageKey};

/// Unique identifier for a webhook
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WebhookId(String);

impl WebhookId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for WebhookId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for WebhookId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for WebhookId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl StorageKey for WebhookId {
    fn as_str(&self) -> &str {
        &self.0
    }
}

impl StorageEntity for Webhook {
    type Key = WebhookId;

    fn key(&self) -> &Self::Key {
        &self.id
    }
}

/// Types of events that can trigger webhooks
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WebhookEventType {
    /// Budget threshold alert triggered
    BudgetAlert,
    /// Budget limit exceeded
    BudgetExceeded,
    /// Experiment completed
    ExperimentCompleted,
    /// Workflow execution failed
    WorkflowFailed,
    /// Workflow execution succeeded
    WorkflowSucceeded,
    /// Model execution failed
    ModelFailed,
    /// API key suspended
    ApiKeySuspended,
    /// API key revoked
    ApiKeyRevoked,
    /// Test case failed
    TestCaseFailed,
}

impl WebhookEventType {
    /// Returns all available event types
    pub fn all() -> Vec<Self> {
        vec![
            Self::BudgetAlert,
            Self::BudgetExceeded,
            Self::ExperimentCompleted,
            Self::WorkflowFailed,
            Self::WorkflowSucceeded,
            Self::ModelFailed,
            Self::ApiKeySuspended,
            Self::ApiKeyRevoked,
            Self::TestCaseFailed,
        ]
    }

    /// Returns the event type as a string
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::BudgetAlert => "budget_alert",
            Self::BudgetExceeded => "budget_exceeded",
            Self::ExperimentCompleted => "experiment_completed",
            Self::WorkflowFailed => "workflow_failed",
            Self::WorkflowSucceeded => "workflow_succeeded",
            Self::ModelFailed => "model_failed",
            Self::ApiKeySuspended => "api_key_suspended",
            Self::ApiKeyRevoked => "api_key_revoked",
            Self::TestCaseFailed => "test_case_failed",
        }
    }
}

impl std::fmt::Display for WebhookEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Status of a webhook
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum WebhookStatus {
    /// Webhook is active and will receive events
    #[default]
    Active,
    /// Webhook is paused and will not receive events
    Paused,
    /// Webhook is disabled due to repeated failures
    Disabled,
}

/// Webhook configuration for HTTP callbacks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Webhook {
    /// Unique identifier
    pub id: WebhookId,
    /// Display name
    pub name: String,
    /// Optional description
    pub description: Option<String>,
    /// Target URL for the webhook
    pub url: String,
    /// Secret for HMAC signature verification
    pub secret: Option<String>,
    /// Event types to subscribe to
    pub events: Vec<WebhookEventType>,
    /// Custom headers to include in requests
    pub headers: HashMap<String, String>,
    /// Current status
    pub status: WebhookStatus,
    /// Number of consecutive failures
    pub failure_count: u32,
    /// Maximum retries before marking as failed
    pub max_retries: u32,
    /// Retry delay in seconds
    pub retry_delay_secs: u32,
    /// Timeout in seconds for HTTP requests
    pub timeout_secs: u32,
    /// When the webhook was created
    pub created_at: DateTime<Utc>,
    /// When the webhook was last updated
    pub updated_at: DateTime<Utc>,
    /// When the last successful delivery occurred
    pub last_success_at: Option<DateTime<Utc>>,
    /// When the last failure occurred
    pub last_failure_at: Option<DateTime<Utc>>,
}

impl Webhook {
    /// Creates a new webhook with default settings
    pub fn new(id: impl Into<WebhookId>, name: impl Into<String>, url: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: id.into(),
            name: name.into(),
            description: None,
            url: url.into(),
            secret: None,
            events: Vec::new(),
            headers: HashMap::new(),
            status: WebhookStatus::Active,
            failure_count: 0,
            max_retries: 3,
            retry_delay_secs: 60,
            timeout_secs: 30,
            created_at: now,
            updated_at: now,
            last_success_at: None,
            last_failure_at: None,
        }
    }

    /// Sets the description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Sets the secret for HMAC signature
    pub fn with_secret(mut self, secret: impl Into<String>) -> Self {
        self.secret = Some(secret.into());
        self
    }

    /// Adds an event type to subscribe to
    pub fn with_event(mut self, event: WebhookEventType) -> Self {
        if !self.events.contains(&event) {
            self.events.push(event);
        }
        self
    }

    /// Sets multiple event types
    pub fn with_events(mut self, events: Vec<WebhookEventType>) -> Self {
        self.events = events;
        self
    }

    /// Adds a custom header
    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    /// Sets the status
    pub fn with_status(mut self, status: WebhookStatus) -> Self {
        self.status = status;
        self
    }

    /// Sets retry configuration
    pub fn with_retry_config(mut self, max_retries: u32, delay_secs: u32) -> Self {
        self.max_retries = max_retries;
        self.retry_delay_secs = delay_secs;
        self
    }

    /// Sets request timeout
    pub fn with_timeout(mut self, timeout_secs: u32) -> Self {
        self.timeout_secs = timeout_secs;
        self
    }

    /// Checks if the webhook is subscribed to an event type
    pub fn is_subscribed_to(&self, event: WebhookEventType) -> bool {
        self.events.contains(&event)
    }

    /// Checks if the webhook is active
    pub fn is_active(&self) -> bool {
        self.status == WebhookStatus::Active
    }

    /// Records a successful delivery
    pub fn record_success(&mut self) {
        self.failure_count = 0;
        self.last_success_at = Some(Utc::now());
        self.updated_at = Utc::now();

        if self.status == WebhookStatus::Disabled {
            self.status = WebhookStatus::Active;
        }
    }

    /// Records a failed delivery
    pub fn record_failure(&mut self) {
        self.failure_count += 1;
        self.last_failure_at = Some(Utc::now());
        self.updated_at = Utc::now();

        // Disable after too many consecutive failures
        if self.failure_count >= self.max_retries * 3 {
            self.status = WebhookStatus::Disabled;
        }
    }

    /// Resets failure count and re-enables the webhook
    pub fn reset_failures(&mut self) {
        self.failure_count = 0;
        self.status = WebhookStatus::Active;
        self.updated_at = Utc::now();
    }
}

/// Unique identifier for a webhook delivery
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WebhookDeliveryId(String);

impl WebhookDeliveryId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for WebhookDeliveryId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for WebhookDeliveryId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for WebhookDeliveryId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl StorageKey for WebhookDeliveryId {
    fn as_str(&self) -> &str {
        &self.0
    }
}

impl StorageEntity for WebhookDelivery {
    type Key = WebhookDeliveryId;

    fn key(&self) -> &Self::Key {
        &self.id
    }
}

/// Status of a webhook delivery attempt
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeliveryStatus {
    /// Delivery is pending
    Pending,
    /// Delivery succeeded
    Success,
    /// Delivery failed, may retry
    Failed,
    /// All retries exhausted
    Exhausted,
}

/// Record of a webhook delivery attempt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookDelivery {
    /// Unique identifier
    pub id: WebhookDeliveryId,
    /// Webhook this delivery belongs to
    pub webhook_id: WebhookId,
    /// Event type that triggered the delivery
    pub event_type: WebhookEventType,
    /// Event payload (JSON)
    pub payload: serde_json::Value,
    /// Delivery status
    pub status: DeliveryStatus,
    /// Number of attempts made
    pub attempts: u32,
    /// HTTP response status code (if received)
    pub response_status: Option<u16>,
    /// HTTP response body (truncated if large)
    pub response_body: Option<String>,
    /// Error message (if failed)
    pub error_message: Option<String>,
    /// When the delivery was created
    pub created_at: DateTime<Utc>,
    /// When the last attempt was made
    pub last_attempt_at: Option<DateTime<Utc>>,
    /// When the next retry is scheduled
    pub next_retry_at: Option<DateTime<Utc>>,
    /// When the delivery completed (success or exhausted)
    pub completed_at: Option<DateTime<Utc>>,
}

impl WebhookDelivery {
    /// Creates a new pending delivery
    pub fn new(
        id: impl Into<WebhookDeliveryId>,
        webhook_id: WebhookId,
        event_type: WebhookEventType,
        payload: serde_json::Value,
    ) -> Self {
        Self {
            id: id.into(),
            webhook_id,
            event_type,
            payload,
            status: DeliveryStatus::Pending,
            attempts: 0,
            response_status: None,
            response_body: None,
            error_message: None,
            created_at: Utc::now(),
            last_attempt_at: None,
            next_retry_at: None,
            completed_at: None,
        }
    }

    /// Records a successful attempt
    pub fn record_success(&mut self, status: u16, body: Option<String>) {
        self.attempts += 1;
        self.status = DeliveryStatus::Success;
        self.response_status = Some(status);
        self.response_body = body;
        self.last_attempt_at = Some(Utc::now());
        self.completed_at = Some(Utc::now());
        self.next_retry_at = None;
    }

    /// Records a failed attempt
    pub fn record_failure(
        &mut self,
        error: impl Into<String>,
        status: Option<u16>,
        body: Option<String>,
        max_retries: u32,
        retry_delay_secs: u32,
    ) {
        self.attempts += 1;
        self.response_status = status;
        self.response_body = body;
        self.error_message = Some(error.into());
        self.last_attempt_at = Some(Utc::now());

        if self.attempts >= max_retries {
            self.status = DeliveryStatus::Exhausted;
            self.completed_at = Some(Utc::now());
            self.next_retry_at = None;
        } else {
            self.status = DeliveryStatus::Failed;
            // Exponential backoff: delay * 2^(attempts-1)
            let backoff = retry_delay_secs * 2u32.pow(self.attempts.saturating_sub(1));
            self.next_retry_at =
                Some(Utc::now() + chrono::Duration::seconds(backoff as i64));
        }
    }

    /// Checks if the delivery should be retried
    pub fn should_retry(&self) -> bool {
        self.status == DeliveryStatus::Failed
            && self.next_retry_at.map_or(false, |t| t <= Utc::now())
    }

    /// Checks if the delivery is complete
    pub fn is_complete(&self) -> bool {
        matches!(self.status, DeliveryStatus::Success | DeliveryStatus::Exhausted)
    }
}

/// Event payload sent to webhooks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookEvent {
    /// Unique event ID
    pub id: String,
    /// Event type
    pub event_type: WebhookEventType,
    /// When the event occurred
    pub timestamp: DateTime<Utc>,
    /// Event-specific data
    pub data: serde_json::Value,
}

impl WebhookEvent {
    /// Creates a new webhook event
    pub fn new(event_type: WebhookEventType, data: serde_json::Value) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            event_type,
            timestamp: Utc::now(),
            data,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_webhook_creation() {
        let webhook = Webhook::new("hook-1", "My Webhook", "https://example.com/webhook");

        assert_eq!(webhook.id.as_str(), "hook-1");
        assert_eq!(webhook.name, "My Webhook");
        assert_eq!(webhook.url, "https://example.com/webhook");
        assert!(webhook.is_active());
        assert_eq!(webhook.failure_count, 0);
    }

    #[test]
    fn test_webhook_with_events() {
        let webhook = Webhook::new("hook-1", "My Webhook", "https://example.com/webhook")
            .with_event(WebhookEventType::BudgetAlert)
            .with_event(WebhookEventType::WorkflowFailed);

        assert!(webhook.is_subscribed_to(WebhookEventType::BudgetAlert));
        assert!(webhook.is_subscribed_to(WebhookEventType::WorkflowFailed));
        assert!(!webhook.is_subscribed_to(WebhookEventType::ExperimentCompleted));
    }

    #[test]
    fn test_webhook_failure_tracking() {
        let mut webhook = Webhook::new("hook-1", "My Webhook", "https://example.com/webhook");

        webhook.record_failure();
        assert_eq!(webhook.failure_count, 1);
        assert!(webhook.is_active());

        // Record many failures to trigger disable
        for _ in 0..8 {
            webhook.record_failure();
        }
        assert_eq!(webhook.status, WebhookStatus::Disabled);
    }

    #[test]
    fn test_webhook_success_resets_failures() {
        let mut webhook = Webhook::new("hook-1", "My Webhook", "https://example.com/webhook");

        webhook.record_failure();
        webhook.record_failure();
        assert_eq!(webhook.failure_count, 2);

        webhook.record_success();
        assert_eq!(webhook.failure_count, 0);
        assert!(webhook.last_success_at.is_some());
    }

    #[test]
    fn test_delivery_creation() {
        let delivery = WebhookDelivery::new(
            "del-1",
            WebhookId::new("hook-1"),
            WebhookEventType::BudgetAlert,
            serde_json::json!({"budget_id": "budget-1"}),
        );

        assert_eq!(delivery.id.as_str(), "del-1");
        assert_eq!(delivery.status, DeliveryStatus::Pending);
        assert_eq!(delivery.attempts, 0);
    }

    #[test]
    fn test_delivery_success() {
        let mut delivery = WebhookDelivery::new(
            "del-1",
            WebhookId::new("hook-1"),
            WebhookEventType::BudgetAlert,
            serde_json::json!({}),
        );

        delivery.record_success(200, Some("OK".to_string()));

        assert_eq!(delivery.status, DeliveryStatus::Success);
        assert_eq!(delivery.attempts, 1);
        assert!(delivery.is_complete());
    }

    #[test]
    fn test_delivery_failure_with_retry() {
        let mut delivery = WebhookDelivery::new(
            "del-1",
            WebhookId::new("hook-1"),
            WebhookEventType::BudgetAlert,
            serde_json::json!({}),
        );

        delivery.record_failure("Connection refused", None, None, 3, 60);

        assert_eq!(delivery.status, DeliveryStatus::Failed);
        assert_eq!(delivery.attempts, 1);
        assert!(!delivery.is_complete());
        assert!(delivery.next_retry_at.is_some());
    }

    #[test]
    fn test_delivery_exhausted_after_max_retries() {
        let mut delivery = WebhookDelivery::new(
            "del-1",
            WebhookId::new("hook-1"),
            WebhookEventType::BudgetAlert,
            serde_json::json!({}),
        );

        for _ in 0..3 {
            delivery.record_failure("Error", None, None, 3, 60);
        }

        assert_eq!(delivery.status, DeliveryStatus::Exhausted);
        assert!(delivery.is_complete());
    }

    #[test]
    fn test_webhook_event_type_all() {
        let all = WebhookEventType::all();
        assert_eq!(all.len(), 9);
    }

    #[test]
    fn test_webhook_event_creation() {
        let event = WebhookEvent::new(
            WebhookEventType::BudgetAlert,
            serde_json::json!({"budget_id": "budget-1", "usage_percent": 85.5}),
        );

        assert_eq!(event.event_type, WebhookEventType::BudgetAlert);
        assert!(!event.id.is_empty());
    }
}
