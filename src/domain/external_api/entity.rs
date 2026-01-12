//! External API entity

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::error::DomainError;
use crate::domain::storage::StorageEntity;

/// External API identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ExternalApiId(String);

impl ExternalApiId {
    /// Create a new external API ID with validation
    pub fn new(id: impl Into<String>) -> Result<Self, DomainError> {
        let id = id.into();

        if id.is_empty() {
            return Err(DomainError::validation("External API ID cannot be empty"));
        }

        if id.len() > 64 {
            return Err(DomainError::validation(
                "External API ID cannot exceed 64 characters",
            ));
        }

        if !id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            return Err(DomainError::validation(
                "External API ID can only contain alphanumeric characters, hyphens, and underscores",
            ));
        }

        Ok(Self(id))
    }

    /// Get the ID as a string slice
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ExternalApiId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for ExternalApiId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl crate::domain::storage::StorageKey for ExternalApiId {
    fn as_str(&self) -> &str {
        &self.0
    }
}

/// External API configuration
///
/// Represents an external HTTP API with its base URL and default headers.
/// Used by HTTP Request workflow steps to define the target API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalApi {
    /// Unique identifier
    id: ExternalApiId,

    /// Human-readable name
    name: String,

    /// Optional description
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,

    /// Base URL for the API (e.g., "https://api.example.com/v1")
    base_url: String,

    /// Default headers to include in all requests
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    base_headers: HashMap<String, String>,

    /// Whether this API is enabled
    #[serde(default = "default_enabled")]
    enabled: bool,

    /// Creation timestamp
    created_at: DateTime<Utc>,

    /// Last update timestamp
    updated_at: DateTime<Utc>,
}

fn default_enabled() -> bool {
    true
}

impl ExternalApi {
    /// Create a new external API
    pub fn new(
        id: ExternalApiId,
        name: impl Into<String>,
        base_url: impl Into<String>,
    ) -> Result<Self, DomainError> {
        let name = name.into();
        let base_url = base_url.into();

        if name.is_empty() {
            return Err(DomainError::validation("Name cannot be empty"));
        }

        if name.len() > 128 {
            return Err(DomainError::validation("Name cannot exceed 128 characters"));
        }

        if base_url.is_empty() {
            return Err(DomainError::validation("Base URL cannot be empty"));
        }

        // Basic URL validation
        if !base_url.starts_with("http://") && !base_url.starts_with("https://") {
            return Err(DomainError::validation(
                "Base URL must start with http:// or https://",
            ));
        }

        let now = Utc::now();

        Ok(Self {
            id,
            name,
            description: None,
            base_url,
            base_headers: HashMap::new(),
            enabled: true,
            created_at: now,
            updated_at: now,
        })
    }

    /// Create with a builder pattern
    pub fn builder(
        id: ExternalApiId,
        name: impl Into<String>,
        base_url: impl Into<String>,
    ) -> Result<ExternalApiBuilder, DomainError> {
        let api = Self::new(id, name, base_url)?;
        Ok(ExternalApiBuilder(api))
    }

    // Getters

    pub fn id(&self) -> &ExternalApiId {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn base_headers(&self) -> &HashMap<String, String> {
        &self.base_headers
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    // Setters

    pub fn set_name(&mut self, name: impl Into<String>) -> Result<(), DomainError> {
        let name = name.into();

        if name.is_empty() {
            return Err(DomainError::validation("Name cannot be empty"));
        }

        if name.len() > 128 {
            return Err(DomainError::validation("Name cannot exceed 128 characters"));
        }

        self.name = name;
        self.updated_at = Utc::now();
        Ok(())
    }

    pub fn set_description(&mut self, description: Option<String>) {
        self.description = description;
        self.updated_at = Utc::now();
    }

    pub fn set_base_url(&mut self, base_url: impl Into<String>) -> Result<(), DomainError> {
        let base_url = base_url.into();

        if base_url.is_empty() {
            return Err(DomainError::validation("Base URL cannot be empty"));
        }

        if !base_url.starts_with("http://") && !base_url.starts_with("https://") {
            return Err(DomainError::validation(
                "Base URL must start with http:// or https://",
            ));
        }

        self.base_url = base_url;
        self.updated_at = Utc::now();
        Ok(())
    }

    pub fn set_base_headers(&mut self, headers: HashMap<String, String>) {
        self.base_headers = headers;
        self.updated_at = Utc::now();
    }

    pub fn add_header(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.base_headers.insert(key.into(), value.into());
        self.updated_at = Utc::now();
    }

    pub fn remove_header(&mut self, key: &str) {
        self.base_headers.remove(key);
        self.updated_at = Utc::now();
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        self.updated_at = Utc::now();
    }

    /// Update multiple fields at once
    pub fn update(
        &mut self,
        name: Option<String>,
        description: Option<Option<String>>,
        base_url: Option<String>,
        base_headers: Option<HashMap<String, String>>,
        enabled: Option<bool>,
    ) -> Result<(), DomainError> {
        if let Some(name) = name {
            self.set_name(name)?;
        }

        if let Some(description) = description {
            self.set_description(description);
        }

        if let Some(base_url) = base_url {
            self.set_base_url(base_url)?;
        }

        if let Some(headers) = base_headers {
            self.set_base_headers(headers);
        }

        if let Some(enabled) = enabled {
            self.set_enabled(enabled);
        }

        self.updated_at = Utc::now();
        Ok(())
    }
}

impl StorageEntity for ExternalApi {
    type Key = ExternalApiId;

    fn key(&self) -> &Self::Key {
        &self.id
    }
}

/// Builder for ExternalApi
pub struct ExternalApiBuilder(ExternalApi);

impl ExternalApiBuilder {
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.0.description = Some(description.into());
        self
    }

    pub fn base_headers(mut self, headers: HashMap<String, String>) -> Self {
        self.0.base_headers = headers;
        self
    }

    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.0.base_headers.insert(key.into(), value.into());
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.0.enabled = enabled;
        self
    }

    pub fn build(self) -> ExternalApi {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_external_api_id_validation() {
        assert!(ExternalApiId::new("valid-id").is_ok());
        assert!(ExternalApiId::new("valid_id_123").is_ok());
        assert!(ExternalApiId::new("").is_err());
        assert!(ExternalApiId::new("invalid id").is_err());
        assert!(ExternalApiId::new("a".repeat(65)).is_err());
    }

    #[test]
    fn test_external_api_creation() {
        let id = ExternalApiId::new("test-api").unwrap();
        let api = ExternalApi::new(id, "Test API", "https://api.example.com").unwrap();

        assert_eq!(api.name(), "Test API");
        assert_eq!(api.base_url(), "https://api.example.com");
        assert!(api.is_enabled());
        assert!(api.base_headers().is_empty());
    }

    #[test]
    fn test_external_api_builder() {
        let id = ExternalApiId::new("test-api").unwrap();
        let api = ExternalApi::builder(id, "Test API", "https://api.example.com")
            .unwrap()
            .description("A test API")
            .header("X-Custom-Header", "value")
            .enabled(false)
            .build();

        assert_eq!(api.description(), Some("A test API"));
        assert_eq!(api.base_headers().get("X-Custom-Header"), Some(&"value".to_string()));
        assert!(!api.is_enabled());
    }

    #[test]
    fn test_external_api_validation_errors() {
        let id = ExternalApiId::new("test").unwrap();

        // Empty name
        assert!(ExternalApi::new(id.clone(), "", "https://api.example.com").is_err());

        // Empty URL
        assert!(ExternalApi::new(id.clone(), "Test", "").is_err());

        // Invalid URL scheme
        assert!(ExternalApi::new(id.clone(), "Test", "ftp://api.example.com").is_err());
    }

    #[test]
    fn test_external_api_update() {
        let id = ExternalApiId::new("test-api").unwrap();
        let mut api = ExternalApi::new(id, "Test API", "https://api.example.com").unwrap();

        api.update(
            Some("Updated API".to_string()),
            Some(Some("New description".to_string())),
            Some("https://new-api.example.com".to_string()),
            None,
            Some(false),
        )
        .unwrap();

        assert_eq!(api.name(), "Updated API");
        assert_eq!(api.description(), Some("New description"));
        assert_eq!(api.base_url(), "https://new-api.example.com");
        assert!(!api.is_enabled());
    }
}
