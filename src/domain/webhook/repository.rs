//! Webhook repository trait

use super::{
    DeliveryStatus, Webhook, WebhookDelivery, WebhookDeliveryId, WebhookEventType, WebhookId,
};
use crate::domain::error::DomainError;
use async_trait::async_trait;

#[cfg(test)]
use mockall::automock;

/// Repository for webhook persistence
#[cfg_attr(test, automock)]
#[async_trait]
pub trait WebhookRepository: Send + Sync {
    /// Creates a new webhook
    async fn create(&self, webhook: Webhook) -> Result<Webhook, DomainError>;

    /// Updates an existing webhook
    async fn update(&self, webhook: Webhook) -> Result<Webhook, DomainError>;

    /// Deletes a webhook by ID
    async fn delete(&self, id: &WebhookId) -> Result<(), DomainError>;

    /// Finds a webhook by ID
    async fn find_by_id(&self, id: &WebhookId) -> Result<Option<Webhook>, DomainError>;

    /// Lists all webhooks
    async fn list(&self) -> Result<Vec<Webhook>, DomainError>;

    /// Finds webhooks subscribed to a specific event type
    async fn find_by_event(&self, event: WebhookEventType) -> Result<Vec<Webhook>, DomainError>;

    /// Finds active webhooks subscribed to a specific event type
    async fn find_active_by_event(
        &self,
        event: WebhookEventType,
    ) -> Result<Vec<Webhook>, DomainError>;
}

/// Repository for webhook delivery persistence
#[cfg_attr(test, automock)]
#[async_trait]
pub trait WebhookDeliveryRepository: Send + Sync {
    /// Creates a new delivery record
    async fn create(&self, delivery: WebhookDelivery) -> Result<WebhookDelivery, DomainError>;

    /// Updates an existing delivery record
    async fn update(&self, delivery: WebhookDelivery) -> Result<WebhookDelivery, DomainError>;

    /// Finds a delivery by ID
    async fn find_by_id(
        &self,
        id: &WebhookDeliveryId,
    ) -> Result<Option<WebhookDelivery>, DomainError>;

    /// Lists deliveries for a webhook
    async fn find_by_webhook(
        &self,
        webhook_id: &WebhookId,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<WebhookDelivery>, DomainError>;

    /// Finds pending deliveries ready for retry
    async fn find_pending_retries(&self) -> Result<Vec<WebhookDelivery>, DomainError>;

    /// Finds deliveries by status
    async fn find_by_status(
        &self,
        status: DeliveryStatus,
        limit: usize,
    ) -> Result<Vec<WebhookDelivery>, DomainError>;

    /// Deletes old completed deliveries
    async fn cleanup_old_deliveries(&self, retention_days: u32) -> Result<u64, DomainError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_webhook_repository() {
        let mut mock = MockWebhookRepository::new();

        mock.expect_list().returning(|| Ok(vec![]));

        let result = mock.list().await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_mock_delivery_repository() {
        let mut mock = MockWebhookDeliveryRepository::new();

        mock.expect_find_pending_retries().returning(|| Ok(vec![]));

        let result = mock.find_pending_retries().await;
        assert!(result.is_ok());
    }
}
