//! Webhook service for sending HTTP callbacks

use crate::domain::{
    DeliveryStatus, DomainError, Webhook, WebhookDelivery, WebhookDeliveryId,
    WebhookDeliveryRepository, WebhookEvent, WebhookId, WebhookRepository, WebhookStatus,
};
use async_trait::async_trait;
use chrono::Utc;
use hmac::{Hmac, Mac};
use reqwest::Client;
use sha2::Sha256;
use std::sync::Arc;
use std::time::Duration;
use tracing::{info, warn};

type HmacSha256 = Hmac<Sha256>;

use crate::infrastructure::usage::AlertNotification;

/// Trait for webhook service operations
#[async_trait]
pub trait WebhookServiceTrait: Send + Sync {
    /// Creates a new webhook
    async fn create(&self, webhook: Webhook) -> Result<Webhook, DomainError>;

    /// Updates an existing webhook
    async fn update(&self, id: &str, webhook: Webhook) -> Result<Webhook, DomainError>;

    /// Deletes a webhook
    async fn delete(&self, id: &str) -> Result<(), DomainError>;

    /// Gets a webhook by ID
    async fn get(&self, id: &str) -> Result<Webhook, DomainError>;

    /// Lists all webhooks
    async fn list(&self) -> Result<Vec<Webhook>, DomainError>;

    /// Sends an event to all subscribed webhooks
    async fn send_event(&self, event: WebhookEvent) -> Result<Vec<WebhookDeliveryId>, DomainError>;

    /// Retries failed deliveries
    async fn retry_failed_deliveries(&self) -> Result<u32, DomainError>;

    /// Gets delivery history for a webhook
    async fn get_deliveries(
        &self,
        webhook_id: &str,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<WebhookDelivery>, DomainError>;

    /// Resets failure count and re-enables a webhook
    async fn reset_webhook(&self, id: &str) -> Result<Webhook, DomainError>;

    /// Cleans up old completed deliveries
    async fn cleanup_deliveries(&self, retention_days: u32) -> Result<u64, DomainError>;

    /// Sends budget alert notifications as webhook events
    async fn send_budget_alerts(
        &self,
        notifications: Vec<AlertNotification>,
    ) -> Result<Vec<WebhookDeliveryId>, DomainError>;

    /// Sends a budget exceeded notification
    async fn send_budget_exceeded(
        &self,
        budget_id: &str,
        budget_name: &str,
        current_usage_micros: i64,
        limit_micros: i64,
    ) -> Result<Vec<WebhookDeliveryId>, DomainError>;
}

/// Webhook service implementation
pub struct WebhookService<W: WebhookRepository, D: WebhookDeliveryRepository> {
    webhook_repo: Arc<W>,
    delivery_repo: Arc<D>,
    http_client: Client,
}

impl<W: WebhookRepository, D: WebhookDeliveryRepository> WebhookService<W, D> {
    /// Creates a new webhook service
    pub fn new(webhook_repo: Arc<W>, delivery_repo: Arc<D>) -> Self {
        let http_client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            webhook_repo,
            delivery_repo,
            http_client,
        }
    }

    /// Generates HMAC-SHA256 signature for a payload
    fn generate_signature(secret: &str, payload: &str) -> String {
        let mut mac =
            HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC can take key of any size");
        mac.update(payload.as_bytes());
        let result = mac.finalize();
        hex::encode(result.into_bytes())
    }

    /// Sends a webhook delivery
    async fn send_delivery(
        &self,
        webhook: &Webhook,
        delivery: &mut WebhookDelivery,
    ) -> Result<(), DomainError> {
        let payload = serde_json::to_string(&delivery.payload)
            .map_err(|e| DomainError::internal(format!("Failed to serialize payload: {}", e)))?;

        let mut request = self
            .http_client
            .post(&webhook.url)
            .timeout(Duration::from_secs(webhook.timeout_secs as u64))
            .header("Content-Type", "application/json")
            .header("X-Webhook-Event", delivery.event_type.as_str())
            .header("X-Webhook-Delivery-Id", delivery.id.as_str());

        // Add HMAC signature if secret is configured
        if let Some(ref secret) = webhook.secret {
            let signature = Self::generate_signature(secret, &payload);
            request = request.header("X-Webhook-Signature", format!("sha256={}", signature));
        }

        // Add custom headers
        for (key, value) in &webhook.headers {
            request = request.header(key, value);
        }

        match request.body(payload).send().await {
            Ok(response) => {
                let status = response.status().as_u16();
                let body = response
                    .text()
                    .await
                    .ok()
                    .map(|b| b.chars().take(1000).collect());

                if (200..300).contains(&(status as i32)) {
                    delivery.record_success(status, body);
                    info!(
                        delivery_id = %delivery.id,
                        webhook_id = %webhook.id,
                        status = status,
                        "Webhook delivery succeeded"
                    );
                } else {
                    delivery.record_failure(
                        format!("HTTP status {}", status),
                        Some(status),
                        body,
                        webhook.max_retries,
                        webhook.retry_delay_secs,
                    );
                    warn!(
                        delivery_id = %delivery.id,
                        webhook_id = %webhook.id,
                        status = status,
                        "Webhook delivery failed with HTTP error"
                    );
                }
            }
            Err(e) => {
                let error_msg = if e.is_timeout() {
                    "Request timed out".to_string()
                } else if e.is_connect() {
                    "Connection failed".to_string()
                } else {
                    format!("Request failed: {}", e)
                };

                delivery.record_failure(
                    &error_msg,
                    None,
                    None,
                    webhook.max_retries,
                    webhook.retry_delay_secs,
                );
                warn!(
                    delivery_id = %delivery.id,
                    webhook_id = %webhook.id,
                    error = %error_msg,
                    "Webhook delivery failed"
                );
            }
        }

        Ok(())
    }

    /// Updates webhook failure tracking based on delivery result
    async fn update_webhook_status(
        &self,
        webhook_id: &WebhookId,
        success: bool,
    ) -> Result<(), DomainError> {
        if let Some(mut webhook) = self.webhook_repo.find_by_id(webhook_id).await? {
            if success {
                webhook.record_success();
            } else {
                webhook.record_failure();
            }
            self.webhook_repo.update(webhook).await?;
        }
        Ok(())
    }
}

#[async_trait]
impl<W: WebhookRepository, D: WebhookDeliveryRepository> WebhookServiceTrait
    for WebhookService<W, D>
{
    async fn create(&self, webhook: Webhook) -> Result<Webhook, DomainError> {
        // Validate URL
        if webhook.url.is_empty() {
            return Err(DomainError::validation("URL is required"));
        }

        if !webhook.url.starts_with("http://") && !webhook.url.starts_with("https://") {
            return Err(DomainError::validation(
                "URL must start with http:// or https://",
            ));
        }

        // Validate events
        if webhook.events.is_empty() {
            return Err(DomainError::validation(
                "At least one event must be subscribed",
            ));
        }

        self.webhook_repo.create(webhook).await
    }

    async fn update(&self, id: &str, mut webhook: Webhook) -> Result<Webhook, DomainError> {
        let existing = self
            .webhook_repo
            .find_by_id(&WebhookId::new(id))
            .await?
            .ok_or_else(|| DomainError::not_found(format!("Webhook '{}' not found", id)))?;

        // Preserve immutable fields
        webhook.id = existing.id;
        webhook.created_at = existing.created_at;
        webhook.updated_at = Utc::now();

        // Validate
        if webhook.url.is_empty() {
            return Err(DomainError::validation("URL is required"));
        }

        if webhook.events.is_empty() {
            return Err(DomainError::validation(
                "At least one event must be subscribed",
            ));
        }

        self.webhook_repo.update(webhook).await
    }

    async fn delete(&self, id: &str) -> Result<(), DomainError> {
        self.webhook_repo.delete(&WebhookId::new(id)).await
    }

    async fn get(&self, id: &str) -> Result<Webhook, DomainError> {
        self.webhook_repo
            .find_by_id(&WebhookId::new(id))
            .await?
            .ok_or_else(|| DomainError::not_found(format!("Webhook '{}' not found", id)))
    }

    async fn list(&self) -> Result<Vec<Webhook>, DomainError> {
        self.webhook_repo.list().await
    }

    async fn send_event(&self, event: WebhookEvent) -> Result<Vec<WebhookDeliveryId>, DomainError> {
        let webhooks = self
            .webhook_repo
            .find_active_by_event(event.event_type)
            .await?;

        if webhooks.is_empty() {
            return Ok(vec![]);
        }

        let mut delivery_ids = Vec::new();

        for webhook in webhooks {
            let delivery_id = WebhookDeliveryId::new(uuid::Uuid::new_v4().to_string());
            let mut delivery = WebhookDelivery::new(
                delivery_id.clone(),
                webhook.id.clone(),
                event.event_type,
                serde_json::to_value(&event).unwrap_or_default(),
            );

            // Create delivery record first
            self.delivery_repo.create(delivery.clone()).await?;

            // Send the delivery
            self.send_delivery(&webhook, &mut delivery).await?;

            // Update delivery status
            self.delivery_repo.update(delivery.clone()).await?;

            // Update webhook status
            let success = delivery.status == DeliveryStatus::Success;
            self.update_webhook_status(&webhook.id, success).await?;

            delivery_ids.push(delivery_id);
        }

        info!(
            event_type = %event.event_type,
            deliveries = delivery_ids.len(),
            "Webhook event dispatched"
        );

        Ok(delivery_ids)
    }

    async fn retry_failed_deliveries(&self) -> Result<u32, DomainError> {
        let pending = self.delivery_repo.find_pending_retries().await?;
        let mut retried = 0;

        for mut delivery in pending {
            let webhook = match self.webhook_repo.find_by_id(&delivery.webhook_id).await? {
                Some(w) if w.status == WebhookStatus::Active => w,
                _ => continue,
            };

            self.send_delivery(&webhook, &mut delivery).await?;
            self.delivery_repo.update(delivery.clone()).await?;

            let success = delivery.status == DeliveryStatus::Success;
            self.update_webhook_status(&webhook.id, success).await?;

            retried += 1;
        }

        if retried > 0 {
            info!(retried = retried, "Retried failed webhook deliveries");
        }

        Ok(retried)
    }

    async fn get_deliveries(
        &self,
        webhook_id: &str,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<WebhookDelivery>, DomainError> {
        // Verify webhook exists
        self.webhook_repo
            .find_by_id(&WebhookId::new(webhook_id))
            .await?
            .ok_or_else(|| DomainError::not_found(format!("Webhook '{}' not found", webhook_id)))?;

        self.delivery_repo
            .find_by_webhook(&WebhookId::new(webhook_id), limit, offset)
            .await
    }

    async fn reset_webhook(&self, id: &str) -> Result<Webhook, DomainError> {
        let mut webhook = self
            .webhook_repo
            .find_by_id(&WebhookId::new(id))
            .await?
            .ok_or_else(|| DomainError::not_found(format!("Webhook '{}' not found", id)))?;

        webhook.reset_failures();
        self.webhook_repo.update(webhook).await
    }

    async fn cleanup_deliveries(&self, retention_days: u32) -> Result<u64, DomainError> {
        let cleaned = self
            .delivery_repo
            .cleanup_old_deliveries(retention_days)
            .await?;

        if cleaned > 0 {
            info!(
                cleaned = cleaned,
                retention_days = retention_days,
                "Cleaned up old deliveries"
            );
        }

        Ok(cleaned)
    }

    async fn send_budget_alerts(
        &self,
        notifications: Vec<AlertNotification>,
    ) -> Result<Vec<WebhookDeliveryId>, DomainError> {
        use crate::domain::WebhookEventType;

        let mut all_deliveries = Vec::new();

        for notification in notifications {
            let data = serde_json::json!({
                "budget_id": notification.budget_id,
                "budget_name": notification.budget_name,
                "threshold_percent": notification.alert.threshold_percent,
                "current_usage_micros": notification.current_usage_micros,
                "current_usage_dollars": notification.current_usage_micros as f64 / 1_000_000.0,
                "limit_micros": notification.limit_micros,
                "limit_dollars": notification.limit_micros as f64 / 1_000_000.0,
                "usage_percent": (notification.current_usage_micros as f64 / notification.limit_micros as f64) * 100.0
            });

            let event = WebhookEvent::new(WebhookEventType::BudgetAlert, data);
            let deliveries = self.send_event(event).await?;
            all_deliveries.extend(deliveries);
        }

        Ok(all_deliveries)
    }

    async fn send_budget_exceeded(
        &self,
        budget_id: &str,
        budget_name: &str,
        current_usage_micros: i64,
        limit_micros: i64,
    ) -> Result<Vec<WebhookDeliveryId>, DomainError> {
        use crate::domain::WebhookEventType;

        let data = serde_json::json!({
            "budget_id": budget_id,
            "budget_name": budget_name,
            "current_usage_micros": current_usage_micros,
            "current_usage_dollars": current_usage_micros as f64 / 1_000_000.0,
            "limit_micros": limit_micros,
            "limit_dollars": limit_micros as f64 / 1_000_000.0,
            "exceeded_by_micros": current_usage_micros - limit_micros,
            "exceeded_by_dollars": (current_usage_micros - limit_micros) as f64 / 1_000_000.0
        });

        let event = WebhookEvent::new(WebhookEventType::BudgetExceeded, data);
        self.send_event(event).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::webhook::{
        InMemoryWebhookDeliveryRepository, InMemoryWebhookRepository,
    };

    fn create_service(
    ) -> WebhookService<InMemoryWebhookRepository, InMemoryWebhookDeliveryRepository> {
        WebhookService::new(
            Arc::new(InMemoryWebhookRepository::new()),
            Arc::new(InMemoryWebhookDeliveryRepository::new()),
        )
    }

    #[tokio::test]
    async fn test_create_webhook() {
        let service = create_service();
        let webhook = Webhook::new("hook-1", "Test Hook", "https://example.com/webhook")
            .with_event(crate::domain::WebhookEventType::BudgetAlert);

        let result = service.create(webhook).await;
        assert!(result.is_ok());

        let found = service.get("hook-1").await.unwrap();
        assert_eq!(found.name, "Test Hook");
    }

    #[tokio::test]
    async fn test_create_webhook_validation() {
        let service = create_service();

        // No URL
        let webhook = Webhook::new("hook-1", "Test Hook", "")
            .with_event(crate::domain::WebhookEventType::BudgetAlert);
        let result = service.create(webhook).await;
        assert!(matches!(
            result,
            Err(DomainError::Validation { message: _ })
        ));

        // Invalid URL scheme
        let webhook = Webhook::new("hook-1", "Test Hook", "ftp://example.com")
            .with_event(crate::domain::WebhookEventType::BudgetAlert);
        let result = service.create(webhook).await;
        assert!(matches!(
            result,
            Err(DomainError::Validation { message: _ })
        ));

        // No events
        let webhook = Webhook::new("hook-1", "Test Hook", "https://example.com/webhook");
        let result = service.create(webhook).await;
        assert!(matches!(
            result,
            Err(DomainError::Validation { message: _ })
        ));
    }

    #[tokio::test]
    async fn test_update_webhook() {
        let service = create_service();
        let webhook = Webhook::new("hook-1", "Test Hook", "https://example.com/webhook")
            .with_event(crate::domain::WebhookEventType::BudgetAlert);

        service.create(webhook).await.unwrap();

        let mut updated = service.get("hook-1").await.unwrap();
        updated.name = "Updated Hook".to_string();

        let result = service.update("hook-1", updated).await;
        assert!(result.is_ok());

        let found = service.get("hook-1").await.unwrap();
        assert_eq!(found.name, "Updated Hook");
    }

    #[tokio::test]
    async fn test_delete_webhook() {
        let service = create_service();
        let webhook = Webhook::new("hook-1", "Test Hook", "https://example.com/webhook")
            .with_event(crate::domain::WebhookEventType::BudgetAlert);

        service.create(webhook).await.unwrap();
        service.delete("hook-1").await.unwrap();

        let result = service.get("hook-1").await;
        assert!(matches!(result, Err(DomainError::NotFound { message: _ })));
    }

    #[tokio::test]
    async fn test_list_webhooks() {
        let service = create_service();

        for i in 1..=3 {
            let webhook = Webhook::new(
                format!("hook-{}", i),
                format!("Hook {}", i),
                format!("https://example.com/hook{}", i),
            )
            .with_event(crate::domain::WebhookEventType::BudgetAlert);
            service.create(webhook).await.unwrap();
        }

        let webhooks = service.list().await.unwrap();
        assert_eq!(webhooks.len(), 3);
    }

    #[tokio::test]
    async fn test_reset_webhook() {
        let service = create_service();
        let mut webhook = Webhook::new("hook-1", "Test Hook", "https://example.com/webhook")
            .with_event(crate::domain::WebhookEventType::BudgetAlert);
        webhook.failure_count = 5;
        webhook.status = WebhookStatus::Disabled;

        // Create directly in repo since status was modified
        service.webhook_repo.create(webhook).await.unwrap();

        let result = service.reset_webhook("hook-1").await;
        assert!(result.is_ok());

        let found = service.get("hook-1").await.unwrap();
        assert_eq!(found.failure_count, 0);
        assert_eq!(found.status, WebhookStatus::Active);
    }

    #[test]
    fn test_generate_signature() {
        let secret = "my-secret";
        let payload = r#"{"event":"budget_alert"}"#;

        let signature = WebhookService::<
            InMemoryWebhookRepository,
            InMemoryWebhookDeliveryRepository,
        >::generate_signature(secret, payload);

        // Signature should be consistent
        let signature2 = WebhookService::<
            InMemoryWebhookRepository,
            InMemoryWebhookDeliveryRepository,
        >::generate_signature(secret, payload);

        assert_eq!(signature, signature2);
        assert!(!signature.is_empty());

        // Different secret should produce different signature
        let signature3 = WebhookService::<
            InMemoryWebhookRepository,
            InMemoryWebhookDeliveryRepository,
        >::generate_signature("other-secret", payload);

        assert_ne!(signature, signature3);
    }
}
