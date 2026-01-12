//! In-memory webhook repository implementations

use crate::domain::{
    DeliveryStatus, DomainError, Webhook, WebhookDelivery, WebhookDeliveryId,
    WebhookDeliveryRepository, WebhookEventType, WebhookId, WebhookRepository, WebhookStatus,
};
use async_trait::async_trait;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::RwLock;

/// In-memory implementation of WebhookRepository
pub struct InMemoryWebhookRepository {
    webhooks: RwLock<HashMap<String, Webhook>>,
}

impl InMemoryWebhookRepository {
    /// Creates a new empty repository
    pub fn new() -> Self {
        Self {
            webhooks: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryWebhookRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl WebhookRepository for InMemoryWebhookRepository {
    async fn create(&self, webhook: Webhook) -> Result<Webhook, DomainError> {
        let mut webhooks = self
            .webhooks
            .write()
            .map_err(|_| DomainError::internal("Failed to acquire lock"))?;

        let id = webhook.id.as_str().to_string();

        if webhooks.contains_key(&id) {
            return Err(DomainError::conflict(format!(
                "Webhook with id '{}' already exists",
                id
            )));
        }

        webhooks.insert(id, webhook.clone());
        Ok(webhook)
    }

    async fn update(&self, webhook: Webhook) -> Result<Webhook, DomainError> {
        let mut webhooks = self
            .webhooks
            .write()
            .map_err(|_| DomainError::internal("Failed to acquire lock"))?;

        let id = webhook.id.as_str().to_string();

        if !webhooks.contains_key(&id) {
            return Err(DomainError::not_found(format!(
                "Webhook with id '{}' not found",
                id
            )));
        }

        webhooks.insert(id, webhook.clone());
        Ok(webhook)
    }

    async fn delete(&self, id: &WebhookId) -> Result<(), DomainError> {
        let mut webhooks = self
            .webhooks
            .write()
            .map_err(|_| DomainError::internal("Failed to acquire lock"))?;

        let key = id.as_str().to_string();

        if webhooks.remove(&key).is_none() {
            return Err(DomainError::not_found(format!(
                "Webhook with id '{}' not found",
                key
            )));
        }

        Ok(())
    }

    async fn find_by_id(&self, id: &WebhookId) -> Result<Option<Webhook>, DomainError> {
        let webhooks = self
            .webhooks
            .read()
            .map_err(|_| DomainError::internal("Failed to acquire lock"))?;

        Ok(webhooks.get(id.as_str()).cloned())
    }

    async fn list(&self) -> Result<Vec<Webhook>, DomainError> {
        let webhooks = self
            .webhooks
            .read()
            .map_err(|_| DomainError::internal("Failed to acquire lock"))?;

        let mut result: Vec<_> = webhooks.values().cloned().collect();
        result.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(result)
    }

    async fn find_by_event(&self, event: WebhookEventType) -> Result<Vec<Webhook>, DomainError> {
        let webhooks = self
            .webhooks
            .read()
            .map_err(|_| DomainError::internal("Failed to acquire lock"))?;

        let result: Vec<_> = webhooks
            .values()
            .filter(|w| w.is_subscribed_to(event))
            .cloned()
            .collect();

        Ok(result)
    }

    async fn find_active_by_event(
        &self,
        event: WebhookEventType,
    ) -> Result<Vec<Webhook>, DomainError> {
        let webhooks = self
            .webhooks
            .read()
            .map_err(|_| DomainError::internal("Failed to acquire lock"))?;

        let result: Vec<_> = webhooks
            .values()
            .filter(|w| w.status == WebhookStatus::Active && w.is_subscribed_to(event))
            .cloned()
            .collect();

        Ok(result)
    }
}

/// In-memory implementation of WebhookDeliveryRepository
pub struct InMemoryWebhookDeliveryRepository {
    deliveries: RwLock<HashMap<String, WebhookDelivery>>,
}

impl InMemoryWebhookDeliveryRepository {
    /// Creates a new empty repository
    pub fn new() -> Self {
        Self {
            deliveries: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryWebhookDeliveryRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl WebhookDeliveryRepository for InMemoryWebhookDeliveryRepository {
    async fn create(&self, delivery: WebhookDelivery) -> Result<WebhookDelivery, DomainError> {
        let mut deliveries = self
            .deliveries
            .write()
            .map_err(|_| DomainError::internal("Failed to acquire lock"))?;

        let id = delivery.id.as_str().to_string();

        if deliveries.contains_key(&id) {
            return Err(DomainError::conflict(format!(
                "Delivery with id '{}' already exists",
                id
            )));
        }

        deliveries.insert(id, delivery.clone());
        Ok(delivery)
    }

    async fn update(&self, delivery: WebhookDelivery) -> Result<WebhookDelivery, DomainError> {
        let mut deliveries = self
            .deliveries
            .write()
            .map_err(|_| DomainError::internal("Failed to acquire lock"))?;

        let id = delivery.id.as_str().to_string();

        if !deliveries.contains_key(&id) {
            return Err(DomainError::not_found(format!(
                "Delivery with id '{}' not found",
                id
            )));
        }

        deliveries.insert(id, delivery.clone());
        Ok(delivery)
    }

    async fn find_by_id(
        &self,
        id: &WebhookDeliveryId,
    ) -> Result<Option<WebhookDelivery>, DomainError> {
        let deliveries = self
            .deliveries
            .read()
            .map_err(|_| DomainError::internal("Failed to acquire lock"))?;

        Ok(deliveries.get(id.as_str()).cloned())
    }

    async fn find_by_webhook(
        &self,
        webhook_id: &WebhookId,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<WebhookDelivery>, DomainError> {
        let deliveries = self
            .deliveries
            .read()
            .map_err(|_| DomainError::internal("Failed to acquire lock"))?;

        let mut result: Vec<_> = deliveries
            .values()
            .filter(|d| d.webhook_id.as_str() == webhook_id.as_str())
            .cloned()
            .collect();

        // Sort by created_at descending (newest first)
        result.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        Ok(result.into_iter().skip(offset).take(limit).collect())
    }

    async fn find_pending_retries(&self) -> Result<Vec<WebhookDelivery>, DomainError> {
        let deliveries = self
            .deliveries
            .read()
            .map_err(|_| DomainError::internal("Failed to acquire lock"))?;

        let now = Utc::now();
        let result: Vec<_> = deliveries
            .values()
            .filter(|d| {
                d.status == DeliveryStatus::Failed && d.next_retry_at.map_or(false, |t| t <= now)
            })
            .cloned()
            .collect();

        Ok(result)
    }

    async fn find_by_status(
        &self,
        status: DeliveryStatus,
        limit: usize,
    ) -> Result<Vec<WebhookDelivery>, DomainError> {
        let deliveries = self
            .deliveries
            .read()
            .map_err(|_| DomainError::internal("Failed to acquire lock"))?;

        let mut result: Vec<_> = deliveries
            .values()
            .filter(|d| d.status == status)
            .cloned()
            .collect();

        result.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        Ok(result.into_iter().take(limit).collect())
    }

    async fn cleanup_old_deliveries(&self, retention_days: u32) -> Result<u64, DomainError> {
        let mut deliveries = self
            .deliveries
            .write()
            .map_err(|_| DomainError::internal("Failed to acquire lock"))?;

        let cutoff = Utc::now() - chrono::Duration::days(retention_days as i64);
        let initial_count = deliveries.len();

        deliveries.retain(|_, d| {
            // Keep if not complete or completed after cutoff
            !d.is_complete() || d.completed_at.map_or(true, |t| t > cutoff)
        });

        Ok((initial_count - deliveries.len()) as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_webhook() {
        let repo = InMemoryWebhookRepository::new();
        let webhook = Webhook::new("hook-1", "Test Hook", "https://example.com/webhook")
            .with_event(WebhookEventType::BudgetAlert);

        let result = repo.create(webhook).await;
        assert!(result.is_ok());

        let found = repo.find_by_id(&WebhookId::new("hook-1")).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "Test Hook");
    }

    #[tokio::test]
    async fn test_create_duplicate_webhook_fails() {
        let repo = InMemoryWebhookRepository::new();
        let webhook = Webhook::new("hook-1", "Test Hook", "https://example.com/webhook");

        repo.create(webhook.clone()).await.unwrap();
        let result = repo.create(webhook).await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DomainError::Conflict { message: _ }));
    }

    #[tokio::test]
    async fn test_update_webhook() {
        let repo = InMemoryWebhookRepository::new();
        let webhook = Webhook::new("hook-1", "Test Hook", "https://example.com/webhook");

        repo.create(webhook).await.unwrap();

        let mut updated = repo.find_by_id(&WebhookId::new("hook-1")).await.unwrap().unwrap();
        updated.name = "Updated Hook".to_string();

        let result = repo.update(updated).await;
        assert!(result.is_ok());

        let found = repo.find_by_id(&WebhookId::new("hook-1")).await.unwrap().unwrap();
        assert_eq!(found.name, "Updated Hook");
    }

    #[tokio::test]
    async fn test_delete_webhook() {
        let repo = InMemoryWebhookRepository::new();
        let webhook = Webhook::new("hook-1", "Test Hook", "https://example.com/webhook");

        repo.create(webhook).await.unwrap();
        repo.delete(&WebhookId::new("hook-1")).await.unwrap();

        let found = repo.find_by_id(&WebhookId::new("hook-1")).await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_find_by_event() {
        let repo = InMemoryWebhookRepository::new();

        let hook1 = Webhook::new("hook-1", "Budget Hook", "https://example.com/budget")
            .with_event(WebhookEventType::BudgetAlert);
        let hook2 = Webhook::new("hook-2", "Workflow Hook", "https://example.com/workflow")
            .with_event(WebhookEventType::WorkflowFailed);
        let hook3 = Webhook::new("hook-3", "Both Hook", "https://example.com/both")
            .with_event(WebhookEventType::BudgetAlert)
            .with_event(WebhookEventType::WorkflowFailed);

        repo.create(hook1).await.unwrap();
        repo.create(hook2).await.unwrap();
        repo.create(hook3).await.unwrap();

        let budget_hooks = repo.find_by_event(WebhookEventType::BudgetAlert).await.unwrap();
        assert_eq!(budget_hooks.len(), 2);

        let workflow_hooks = repo.find_by_event(WebhookEventType::WorkflowFailed).await.unwrap();
        assert_eq!(workflow_hooks.len(), 2);
    }

    #[tokio::test]
    async fn test_find_active_by_event() {
        let repo = InMemoryWebhookRepository::new();

        let hook1 = Webhook::new("hook-1", "Active Hook", "https://example.com/active")
            .with_event(WebhookEventType::BudgetAlert);
        let hook2 = Webhook::new("hook-2", "Paused Hook", "https://example.com/paused")
            .with_event(WebhookEventType::BudgetAlert)
            .with_status(WebhookStatus::Paused);

        repo.create(hook1).await.unwrap();
        repo.create(hook2).await.unwrap();

        let active_hooks = repo.find_active_by_event(WebhookEventType::BudgetAlert).await.unwrap();
        assert_eq!(active_hooks.len(), 1);
        assert_eq!(active_hooks[0].id.as_str(), "hook-1");
    }

    #[tokio::test]
    async fn test_create_delivery() {
        let repo = InMemoryWebhookDeliveryRepository::new();
        let delivery = WebhookDelivery::new(
            "del-1",
            WebhookId::new("hook-1"),
            WebhookEventType::BudgetAlert,
            serde_json::json!({"budget_id": "budget-1"}),
        );

        let result = repo.create(delivery).await;
        assert!(result.is_ok());

        let found = repo.find_by_id(&WebhookDeliveryId::new("del-1")).await.unwrap();
        assert!(found.is_some());
    }

    #[tokio::test]
    async fn test_find_deliveries_by_webhook() {
        let repo = InMemoryWebhookDeliveryRepository::new();
        let webhook_id = WebhookId::new("hook-1");

        for i in 0..5 {
            let delivery = WebhookDelivery::new(
                format!("del-{}", i),
                webhook_id.clone(),
                WebhookEventType::BudgetAlert,
                serde_json::json!({}),
            );
            repo.create(delivery).await.unwrap();
        }

        // Also create a delivery for a different webhook
        let other_delivery = WebhookDelivery::new(
            "del-other",
            WebhookId::new("hook-2"),
            WebhookEventType::BudgetAlert,
            serde_json::json!({}),
        );
        repo.create(other_delivery).await.unwrap();

        let deliveries = repo.find_by_webhook(&webhook_id, 10, 0).await.unwrap();
        assert_eq!(deliveries.len(), 5);

        // Test pagination
        let page = repo.find_by_webhook(&webhook_id, 2, 2).await.unwrap();
        assert_eq!(page.len(), 2);
    }

    #[tokio::test]
    async fn test_find_pending_retries() {
        let repo = InMemoryWebhookDeliveryRepository::new();

        // Create a delivery that should be retried
        let mut delivery1 = WebhookDelivery::new(
            "del-1",
            WebhookId::new("hook-1"),
            WebhookEventType::BudgetAlert,
            serde_json::json!({}),
        );
        delivery1.status = DeliveryStatus::Failed;
        delivery1.next_retry_at = Some(Utc::now() - chrono::Duration::minutes(1));
        repo.create(delivery1).await.unwrap();

        // Create a delivery not ready for retry
        let mut delivery2 = WebhookDelivery::new(
            "del-2",
            WebhookId::new("hook-1"),
            WebhookEventType::BudgetAlert,
            serde_json::json!({}),
        );
        delivery2.status = DeliveryStatus::Failed;
        delivery2.next_retry_at = Some(Utc::now() + chrono::Duration::hours(1));
        repo.create(delivery2).await.unwrap();

        // Create a successful delivery (should not be returned)
        let mut delivery3 = WebhookDelivery::new(
            "del-3",
            WebhookId::new("hook-1"),
            WebhookEventType::BudgetAlert,
            serde_json::json!({}),
        );
        delivery3.status = DeliveryStatus::Success;
        repo.create(delivery3).await.unwrap();

        let pending = repo.find_pending_retries().await.unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id.as_str(), "del-1");
    }

    #[tokio::test]
    async fn test_cleanup_old_deliveries() {
        let repo = InMemoryWebhookDeliveryRepository::new();

        // Create an old completed delivery
        let mut old_delivery = WebhookDelivery::new(
            "del-old",
            WebhookId::new("hook-1"),
            WebhookEventType::BudgetAlert,
            serde_json::json!({}),
        );
        old_delivery.status = DeliveryStatus::Success;
        old_delivery.completed_at = Some(Utc::now() - chrono::Duration::days(40));
        repo.create(old_delivery).await.unwrap();

        // Create a recent completed delivery
        let mut recent_delivery = WebhookDelivery::new(
            "del-recent",
            WebhookId::new("hook-1"),
            WebhookEventType::BudgetAlert,
            serde_json::json!({}),
        );
        recent_delivery.status = DeliveryStatus::Success;
        recent_delivery.completed_at = Some(Utc::now());
        repo.create(recent_delivery).await.unwrap();

        // Create a pending delivery (should not be cleaned up)
        let pending_delivery = WebhookDelivery::new(
            "del-pending",
            WebhookId::new("hook-1"),
            WebhookEventType::BudgetAlert,
            serde_json::json!({}),
        );
        repo.create(pending_delivery).await.unwrap();

        let cleaned = repo.cleanup_old_deliveries(30).await.unwrap();
        assert_eq!(cleaned, 1);

        // Verify old is gone, others remain
        assert!(repo.find_by_id(&WebhookDeliveryId::new("del-old")).await.unwrap().is_none());
        assert!(repo.find_by_id(&WebhookDeliveryId::new("del-recent")).await.unwrap().is_some());
        assert!(repo.find_by_id(&WebhookDeliveryId::new("del-pending")).await.unwrap().is_some());
    }
}
