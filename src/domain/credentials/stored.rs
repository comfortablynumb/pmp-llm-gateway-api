//! Stored credential entity with ID for persistence

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

use super::CredentialType;
use crate::domain::storage::{StorageEntity, StorageKey};
use crate::domain::DomainError;

/// Unique identifier for a stored credential
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CredentialId(String);

impl CredentialId {
    /// Create a new credential ID with validation
    pub fn new(id: impl Into<String>) -> Result<Self, DomainError> {
        let id = id.into();
        validate_credential_id(&id)?;
        Ok(Self(id))
    }

    /// Get the ID as a string slice
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for CredentialId {
    type Error = DomainError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<CredentialId> for String {
    fn from(id: CredentialId) -> Self {
        id.0
    }
}

impl std::fmt::Display for CredentialId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl StorageKey for CredentialId {
    fn as_str(&self) -> &str {
        &self.0
    }
}

fn validate_credential_id(id: &str) -> Result<(), DomainError> {
    if id.is_empty() {
        return Err(DomainError::validation("Credential ID cannot be empty"));
    }

    if id.len() > 50 {
        return Err(DomainError::validation(
            "Credential ID cannot exceed 50 characters",
        ));
    }

    if !id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(DomainError::validation(
            "Credential ID can only contain alphanumeric characters, hyphens, and underscores",
        ));
    }

    Ok(())
}

/// A stored credential with ID for persistence
///
/// For HTTP API Key credentials:
/// - `deployment` is used as the header name (e.g., "Authorization")
/// - `header_value` is the header value template (e.g., "Bearer ${api-key}")
/// - Base URL and base headers come from the associated ExternalApi
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredCredential {
    id: CredentialId,
    name: String,
    credential_type: CredentialType,
    #[serde(default)]
    api_key: String,
    /// Endpoint URL (for Azure OpenAI)
    #[serde(skip_serializing_if = "Option::is_none")]
    endpoint: Option<String>,
    /// Deployment name (Azure OpenAI) or header name (HTTP API Key, e.g., "Authorization")
    #[serde(skip_serializing_if = "Option::is_none")]
    deployment: Option<String>,
    /// Header value template for HTTP API Key credentials (e.g., "Bearer ${api-key}")
    #[serde(skip_serializing_if = "Option::is_none")]
    header_value: Option<String>,
    enabled: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl StoredCredential {
    /// Create a new stored credential
    pub fn new(
        id: CredentialId,
        name: impl Into<String>,
        credential_type: CredentialType,
        api_key: impl Into<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id,
            name: name.into(),
            credential_type,
            api_key: api_key.into(),
            endpoint: None,
            deployment: None,
            header_value: None,
            enabled: true,
            created_at: now,
            updated_at: now,
        }
    }

    /// Set endpoint (for Azure OpenAI)
    pub fn with_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint = Some(endpoint.into());
        self
    }

    /// Set deployment (for Azure OpenAI) or header name (for HTTP API Key)
    pub fn with_deployment(mut self, deployment: impl Into<String>) -> Self {
        self.deployment = Some(deployment.into());
        self
    }

    /// Set header value template for HTTP API Key credentials
    pub fn with_header_value(mut self, header_value: impl Into<String>) -> Self {
        self.header_value = Some(header_value.into());
        self
    }

    /// Set enabled status
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    // Getters
    pub fn id(&self) -> &CredentialId {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn credential_type(&self) -> &CredentialType {
        &self.credential_type
    }

    pub fn api_key(&self) -> &str {
        &self.api_key
    }

    pub fn endpoint(&self) -> Option<&str> {
        self.endpoint.as_deref()
    }

    pub fn deployment(&self) -> Option<&str> {
        self.deployment.as_deref()
    }

    pub fn header_value(&self) -> Option<&str> {
        self.header_value.as_deref()
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

    /// Update credential fields
    pub fn update(
        &mut self,
        name: Option<String>,
        api_key: Option<String>,
        endpoint: Option<Option<String>>,
        deployment: Option<Option<String>>,
        header_value: Option<Option<String>>,
        enabled: Option<bool>,
    ) {
        if let Some(n) = name {
            self.name = n;
        }

        if let Some(k) = api_key {
            self.api_key = k;
        }

        if let Some(e) = endpoint {
            self.endpoint = e;
        }

        if let Some(d) = deployment {
            self.deployment = d;
        }

        if let Some(hv) = header_value {
            self.header_value = hv;
        }

        if let Some(en) = enabled {
            self.enabled = en;
        }
        self.updated_at = Utc::now();
    }

    /// Convert to a Credential domain object for use with providers
    pub fn to_credential(&self) -> super::Credential {
        let mut cred = super::Credential::new(self.credential_type.clone(), self.api_key.clone());

        // Add endpoint as base_url parameter (for Azure OpenAI)
        if let Some(endpoint) = &self.endpoint {
            cred = cred.with_param("base_url", endpoint);
        }

        // Add deployment parameter (deployment for Azure, header_name for HTTP API Key)
        if let Some(deployment) = &self.deployment {
            cred = cred.with_param("header_name", deployment);
        }

        // Add header_value for HTTP API Key credentials
        if let Some(header_value) = &self.header_value {
            cred = cred.with_param("header_value", header_value);
        }

        cred
    }
}

impl StorageEntity for StoredCredential {
    type Key = CredentialId;

    fn key(&self) -> &Self::Key {
        &self.id
    }
}

/// Repository trait for stored credentials
#[async_trait]
pub trait StoredCredentialRepository: Send + Sync + Debug {
    /// Get a credential by ID
    async fn get(&self, id: &CredentialId) -> Result<Option<StoredCredential>, DomainError>;

    /// List all credentials
    async fn list(&self) -> Result<Vec<StoredCredential>, DomainError>;

    /// List credentials by provider type
    async fn list_by_type(
        &self,
        credential_type: &CredentialType,
    ) -> Result<Vec<StoredCredential>, DomainError>;

    /// Create a new credential
    async fn create(&self, credential: StoredCredential) -> Result<StoredCredential, DomainError>;

    /// Update a credential
    async fn update(&self, credential: StoredCredential) -> Result<StoredCredential, DomainError>;

    /// Delete a credential
    async fn delete(&self, id: &CredentialId) -> Result<(), DomainError>;

    /// Check if a credential exists
    async fn exists(&self, id: &CredentialId) -> Result<bool, DomainError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_credential_id_valid() {
        assert!(CredentialId::new("openai-prod").is_ok());
        assert!(CredentialId::new("azure_openai_1").is_ok());
        assert!(CredentialId::new("my-api-key").is_ok());
    }

    #[test]
    fn test_credential_id_invalid() {
        assert!(CredentialId::new("").is_err());
        assert!(CredentialId::new("a".repeat(51)).is_err());
        assert!(CredentialId::new("has spaces").is_err());
        assert!(CredentialId::new("has.dots").is_err());
    }

    #[test]
    fn test_stored_credential_creation() {
        let id = CredentialId::new("openai-prod").unwrap();
        let cred = StoredCredential::new(
            id.clone(),
            "Production OpenAI",
            CredentialType::OpenAi,
            "sk-test-key",
        );

        assert_eq!(cred.id().as_str(), "openai-prod");
        assert_eq!(cred.name(), "Production OpenAI");
        assert_eq!(cred.credential_type(), &CredentialType::OpenAi);
        assert_eq!(cred.api_key(), "sk-test-key");
        assert!(cred.is_enabled());
    }

    #[test]
    fn test_stored_credential_azure() {
        let id = CredentialId::new("azure-prod").unwrap();
        let cred = StoredCredential::new(
            id,
            "Azure Production",
            CredentialType::AzureOpenAi,
            "azure-key",
        )
        .with_endpoint("https://myinstance.openai.azure.com")
        .with_deployment("gpt-4");

        assert_eq!(
            cred.endpoint(),
            Some("https://myinstance.openai.azure.com")
        );
        assert_eq!(cred.deployment(), Some("gpt-4"));
    }

    #[test]
    fn test_stored_credential_update() {
        let id = CredentialId::new("test").unwrap();
        let mut cred = StoredCredential::new(id, "Test", CredentialType::OpenAi, "key");

        cred.update(
            Some("Updated Name".to_string()),
            Some("new-key".to_string()),
            None,
            None,
            None,
            Some(false),
        );

        assert_eq!(cred.name(), "Updated Name");
        assert_eq!(cred.api_key(), "new-key");
        assert!(!cred.is_enabled());
    }

    #[test]
    fn test_stored_credential_http_api_key() {
        let id = CredentialId::new("my-api").unwrap();

        let cred = StoredCredential::new(id, "My HTTP API", CredentialType::HttpApiKey, "secret-key")
            .with_deployment("Authorization")
            .with_header_value("Bearer ${api-key}");

        assert_eq!(cred.deployment(), Some("Authorization"));
        assert_eq!(cred.header_value(), Some("Bearer ${api-key}"));
    }
}
