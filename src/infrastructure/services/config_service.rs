//! Configuration service - Manages application configuration

use std::sync::Arc;

use crate::domain::{
    AppConfiguration, ConfigCategory, ConfigEntry, ConfigKey, ConfigRepository, ConfigValue,
    DomainError,
};

/// Configuration service for managing application settings
pub struct ConfigService {
    repository: Arc<dyn ConfigRepository>,
}

impl ConfigService {
    /// Create a new ConfigService with the given repository
    pub fn new(repository: Arc<dyn ConfigRepository>) -> Self {
        Self { repository }
    }

    /// Get the current configuration
    pub async fn get(&self) -> Result<AppConfiguration, DomainError> {
        self.repository.get().await
    }

    /// Get a specific configuration entry
    pub async fn get_entry(&self, key: &str) -> Result<Option<ConfigEntry>, DomainError> {
        let config = self.repository.get().await?;
        Ok(config.get(key).cloned())
    }

    /// Get a configuration value
    pub async fn get_value(&self, key: &str) -> Result<Option<ConfigValue>, DomainError> {
        let config = self.repository.get().await?;
        Ok(config.get_value(key).cloned())
    }

    /// Set a configuration value
    pub async fn set(&self, key: &str, value: ConfigValue) -> Result<(), DomainError> {
        let config_key = ConfigKey::new(key)
            .map_err(|e| DomainError::validation(format!("Invalid config key: {}", e)))?;
        self.repository.set(&config_key, value).await
    }

    /// Reset configuration to defaults
    pub async fn reset(&self) -> Result<(), DomainError> {
        self.repository.reset().await
    }

    /// List all configuration entries
    pub async fn list(&self) -> Result<Vec<ConfigEntry>, DomainError> {
        let config = self.repository.get().await?;
        Ok(config.list().into_iter().cloned().collect())
    }

    /// List configuration entries by category
    pub async fn list_by_category(
        &self,
        category: ConfigCategory,
    ) -> Result<Vec<ConfigEntry>, DomainError> {
        let config = self.repository.get().await?;
        Ok(config
            .list_by_category(category)
            .into_iter()
            .cloned()
            .collect())
    }

    /// Get all categories
    pub fn categories(&self) -> Vec<ConfigCategory> {
        vec![
            ConfigCategory::General,
            ConfigCategory::Persistence,
            ConfigCategory::Logging,
            ConfigCategory::Security,
            ConfigCategory::Cache,
            ConfigCategory::RateLimit,
        ]
    }

    // Convenience methods for common settings

    /// Check if persistence/execution logging is enabled
    pub async fn is_persistence_enabled(&self) -> Result<bool, DomainError> {
        let config = self.repository.get().await?;
        Ok(config.is_persistence_enabled())
    }

    /// Check if a specific model should be logged
    pub async fn should_log_model(&self, model_id: &str) -> Result<bool, DomainError> {
        let config = self.repository.get().await?;
        Ok(config.should_log_model(model_id))
    }

    /// Check if a specific workflow should be logged
    pub async fn should_log_workflow(&self, workflow_id: &str) -> Result<bool, DomainError> {
        let config = self.repository.get().await?;
        Ok(config.should_log_workflow(workflow_id))
    }

    /// Get log retention days
    pub async fn log_retention_days(&self) -> Result<i64, DomainError> {
        let config = self.repository.get().await?;
        Ok(config.log_retention_days())
    }

    /// Check if sensitive data should be logged
    pub async fn log_sensitive_data(&self) -> Result<bool, DomainError> {
        let config = self.repository.get().await?;
        Ok(config.log_sensitive_data())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::AppConfiguration;
    use crate::infrastructure::config::StorageConfigRepository;
    use crate::infrastructure::storage::InMemoryStorage;

    fn create_service() -> ConfigService {
        let storage = Arc::new(InMemoryStorage::<AppConfiguration>::new());
        let repository = Arc::new(StorageConfigRepository::new(storage));
        ConfigService::new(repository)
    }

    #[tokio::test]
    async fn test_get_default_config() {
        let service = create_service();
        let config = service.get().await.unwrap();

        assert!(!config.is_persistence_enabled());
    }

    #[tokio::test]
    async fn test_set_and_get_value() {
        let service = create_service();

        service
            .set("persistence.enabled", ConfigValue::Boolean(true))
            .await
            .unwrap();

        let value = service.get_value("persistence.enabled").await.unwrap();
        assert_eq!(value, Some(ConfigValue::Boolean(true)));
    }

    #[tokio::test]
    async fn test_reset_config() {
        let service = create_service();

        service
            .set("persistence.enabled", ConfigValue::Boolean(true))
            .await
            .unwrap();

        service.reset().await.unwrap();

        let value = service.get_value("persistence.enabled").await.unwrap();
        assert_eq!(value.and_then(|v| v.as_boolean()), Some(false));
    }

    #[tokio::test]
    async fn test_list_entries() {
        let service = create_service();

        let entries = service.list().await.unwrap();
        assert!(!entries.is_empty());
    }

    #[tokio::test]
    async fn test_list_by_category() {
        let service = create_service();

        let entries = service
            .list_by_category(ConfigCategory::Persistence)
            .await
            .unwrap();
        assert!(!entries.is_empty());

        for entry in entries {
            assert_eq!(entry.category(), ConfigCategory::Persistence);
        }
    }

    #[tokio::test]
    async fn test_convenience_methods() {
        let service = create_service();

        // Default values
        assert!(!service.is_persistence_enabled().await.unwrap());
        assert_eq!(service.log_retention_days().await.unwrap(), 30);
        assert!(!service.log_sensitive_data().await.unwrap());

        // Enable persistence
        service
            .set("persistence.enabled", ConfigValue::Boolean(true))
            .await
            .unwrap();

        assert!(service.is_persistence_enabled().await.unwrap());
        assert!(service.should_log_model("any-model").await.unwrap());
        assert!(service.should_log_workflow("any-workflow").await.unwrap());
    }
}
