//! Storage-backed webhook repository implementations

use async_trait::async_trait;
use chrono::Utc;
use std::sync::Arc;

use crate::domain::storage::Storage;
use crate::domain::webhook::{
    DeliveryStatus, Webhook, WebhookDelivery, WebhookDeliveryId, WebhookDeliveryRepository,
    WebhookEventType, WebhookId, WebhookRepository, WebhookStatus,
};
use crate::domain::DomainError;

/// Storage-backed implementation of WebhookRepository
#[derive(Debug)]
pub struct StorageWebhookRepository {
    storage: Arc<dyn Storage<Webhook>>,
}

impl StorageWebhookRepository {
    /// Create a new storage-backed repository
    pub fn new(storage: Arc<dyn Storage<Webhook>>) -> Self {
        Self { storage }
    }
}

#[async_trait]
impl WebhookRepository for StorageWebhookRepository {
    async fn create(&self, webhook: Webhook) -> Result<Webhook, DomainError> {
        if self.storage.exists(&webhook.id).await? {
            return Err(DomainError::conflict(format!(
                "Webhook '{}' already exists",
                webhook.id
            )));
        }

        self.storage.create(webhook).await
    }

    async fn update(&self, webhook: Webhook) -> Result<Webhook, DomainError> {
        if !self.storage.exists(&webhook.id).await? {
            return Err(DomainError::not_found(format!(
                "Webhook '{}' not found",
                webhook.id
            )));
        }

        self.storage.update(webhook).await
    }

    async fn delete(&self, id: &WebhookId) -> Result<(), DomainError> {
        self.storage.delete(id).await?;
        Ok(())
    }

    async fn find_by_id(&self, id: &WebhookId) -> Result<Option<Webhook>, DomainError> {
        self.storage.get(id).await
    }

    async fn list(&self) -> Result<Vec<Webhook>, DomainError> {
        let mut webhooks = self.storage.list().await?;
        webhooks.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(webhooks)
    }

    async fn find_by_event(&self, event: WebhookEventType) -> Result<Vec<Webhook>, DomainError> {
        let all = self.storage.list().await?;
        Ok(all
            .into_iter()
            .filter(|w| w.is_subscribed_to(event))
            .collect())
    }

    async fn find_active_by_event(
        &self,
        event: WebhookEventType,
    ) -> Result<Vec<Webhook>, DomainError> {
        let all = self.storage.list().await?;
        Ok(all
            .into_iter()
            .filter(|w| w.status == WebhookStatus::Active && w.is_subscribed_to(event))
            .collect())
    }
}

/// Storage-backed implementation of WebhookDeliveryRepository
#[derive(Debug)]
pub struct StorageWebhookDeliveryRepository {
    storage: Arc<dyn Storage<WebhookDelivery>>,
}

impl StorageWebhookDeliveryRepository {
    /// Create a new storage-backed repository
    pub fn new(storage: Arc<dyn Storage<WebhookDelivery>>) -> Self {
        Self { storage }
    }
}

#[async_trait]
impl WebhookDeliveryRepository for StorageWebhookDeliveryRepository {
    async fn create(&self, delivery: WebhookDelivery) -> Result<WebhookDelivery, DomainError> {
        self.storage.create(delivery).await
    }

    async fn update(&self, delivery: WebhookDelivery) -> Result<WebhookDelivery, DomainError> {
        self.storage.update(delivery).await
    }

    async fn find_by_id(
        &self,
        id: &WebhookDeliveryId,
    ) -> Result<Option<WebhookDelivery>, DomainError> {
        self.storage.get(id).await
    }

    async fn find_by_webhook(
        &self,
        webhook_id: &WebhookId,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<WebhookDelivery>, DomainError> {
        let all = self.storage.list().await?;
        let mut filtered: Vec<_> = all
            .into_iter()
            .filter(|d| &d.webhook_id == webhook_id)
            .collect();

        // Sort by created_at descending
        filtered.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        // Apply pagination
        Ok(filtered.into_iter().skip(offset).take(limit).collect())
    }

    async fn find_pending_retries(&self) -> Result<Vec<WebhookDelivery>, DomainError> {
        let all = self.storage.list().await?;
        let now = Utc::now();

        Ok(all
            .into_iter()
            .filter(|d| {
                d.status == DeliveryStatus::Failed
                    && d.next_retry_at.map_or(false, |t| t <= now)
            })
            .collect())
    }

    async fn find_by_status(
        &self,
        status: DeliveryStatus,
        limit: usize,
    ) -> Result<Vec<WebhookDelivery>, DomainError> {
        let all = self.storage.list().await?;
        Ok(all
            .into_iter()
            .filter(|d| d.status == status)
            .take(limit)
            .collect())
    }

    async fn cleanup_old_deliveries(&self, retention_days: u32) -> Result<u64, DomainError> {
        let all = self.storage.list().await?;
        let cutoff = Utc::now() - chrono::Duration::days(retention_days as i64);
        let mut deleted = 0u64;

        for delivery in all {
            if delivery.is_complete() && delivery.created_at < cutoff {
                if self.storage.delete(&delivery.id).await? {
                    deleted += 1;
                }
            }
        }

        Ok(deleted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::storage::InMemoryStorage;

    fn create_webhook_repo() -> StorageWebhookRepository {
        let storage = Arc::new(InMemoryStorage::<Webhook>::new());
        StorageWebhookRepository::new(storage)
    }

    fn create_delivery_repo() -> StorageWebhookDeliveryRepository {
        let storage = Arc::new(InMemoryStorage::<WebhookDelivery>::new());
        StorageWebhookDeliveryRepository::new(storage)
    }

    #[tokio::test]
    async fn test_webhook_crud() {
        let repo = create_webhook_repo();
        let webhook = Webhook::new("hook-1", "Test Webhook", "https://example.com/webhook")
            .with_event(WebhookEventType::BudgetAlert);

        // Create
        let created = repo.create(webhook).await.unwrap();
        assert_eq!(created.id.as_str(), "hook-1");

        // Find by ID
        let found = repo.find_by_id(&WebhookId::new("hook-1")).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "Test Webhook");

        // Update
        let mut updated = repo
            .find_by_id(&WebhookId::new("hook-1"))
            .await
            .unwrap()
            .unwrap();
        updated.name = "Updated Webhook".to_string();
        repo.update(updated).await.unwrap();

        let found = repo.find_by_id(&WebhookId::new("hook-1")).await.unwrap();
        assert_eq!(found.unwrap().name, "Updated Webhook");

        // Delete
        repo.delete(&WebhookId::new("hook-1")).await.unwrap();
        let found = repo.find_by_id(&WebhookId::new("hook-1")).await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_find_by_event() {
        let repo = create_webhook_repo();

        let webhook1 = Webhook::new("hook-1", "Hook 1", "https://example.com/1")
            .with_event(WebhookEventType::BudgetAlert);
        let webhook2 = Webhook::new("hook-2", "Hook 2", "https://example.com/2")
            .with_event(WebhookEventType::WorkflowFailed);

        repo.create(webhook1).await.unwrap();
        repo.create(webhook2).await.unwrap();

        let budget_hooks = repo
            .find_by_event(WebhookEventType::BudgetAlert)
            .await
            .unwrap();
        assert_eq!(budget_hooks.len(), 1);
        assert_eq!(budget_hooks[0].id.as_str(), "hook-1");
    }

    #[tokio::test]
    async fn test_delivery_crud() {
        let repo = create_delivery_repo();
        let delivery = WebhookDelivery::new(
            "del-1",
            WebhookId::new("hook-1"),
            WebhookEventType::BudgetAlert,
            serde_json::json!({"test": true}),
        );

        // Create
        repo.create(delivery).await.unwrap();

        // Find by ID
        let found = repo
            .find_by_id(&WebhookDeliveryId::new("del-1"))
            .await
            .unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().status, DeliveryStatus::Pending);
    }

    #[tokio::test]
    async fn test_find_by_webhook() {
        let repo = create_delivery_repo();

        for i in 0..5 {
            let delivery = WebhookDelivery::new(
                format!("del-{}", i),
                WebhookId::new("hook-1"),
                WebhookEventType::BudgetAlert,
                serde_json::json!({}),
            );
            repo.create(delivery).await.unwrap();
        }

        let deliveries = repo
            .find_by_webhook(&WebhookId::new("hook-1"), 3, 0)
            .await
            .unwrap();
        assert_eq!(deliveries.len(), 3);
    }
}
