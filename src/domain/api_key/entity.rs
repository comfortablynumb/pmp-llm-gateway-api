//! API Key entity and related types

use std::collections::HashSet;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::validation::{validate_api_key_id, ApiKeyValidationError};
use crate::domain::storage::{StorageEntity, StorageKey};
use crate::domain::team::TeamId;

/// API Key identifier - alphanumeric + hyphens, max 50 characters
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ApiKeyId(String);

impl ApiKeyId {
    /// Create a new ApiKeyId after validation
    pub fn new(id: impl Into<String>) -> Result<Self, ApiKeyValidationError> {
        let id = id.into();
        validate_api_key_id(&id)?;
        Ok(Self(id))
    }

    /// Get the inner string value
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for ApiKeyId {
    type Error = ApiKeyValidationError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<ApiKeyId> for String {
    fn from(id: ApiKeyId) -> Self {
        id.0
    }
}

impl std::fmt::Display for ApiKeyId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl StorageKey for ApiKeyId {
    fn as_str(&self) -> &str {
        &self.0
    }
}

impl StorageEntity for ApiKey {
    type Key = ApiKeyId;

    fn key(&self) -> &Self::Key {
        &self.id
    }
}

/// Status of an API key
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ApiKeyStatus {
    /// Key is active and can be used
    #[default]
    Active,
    /// Key is temporarily suspended
    Suspended,
    /// Key has been revoked and cannot be used
    Revoked,
    /// Key has expired
    Expired,
}

impl ApiKeyStatus {
    /// Check if the key is usable
    pub fn is_usable(&self) -> bool {
        matches!(self, Self::Active)
    }
}

/// Permission for a specific resource type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourcePermission {
    /// Access to all resources of this type
    All,
    /// Access to specific resources by ID
    Specific(HashSet<String>),
    /// No access to this resource type
    None,
}

impl Default for ResourcePermission {
    fn default() -> Self {
        Self::None
    }
}

impl ResourcePermission {
    /// Check if access to a specific resource is allowed
    pub fn allows(&self, resource_id: &str) -> bool {
        match self {
            Self::All => true,
            Self::Specific(ids) => ids.contains(resource_id),
            Self::None => false,
        }
    }

    /// Check if any access is allowed
    pub fn has_any_access(&self) -> bool {
        !matches!(self, Self::None)
    }

    /// Create permission for all resources
    pub fn all() -> Self {
        Self::All
    }

    /// Create permission for specific resources
    pub fn specific(ids: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self::Specific(ids.into_iter().map(|s| s.into()).collect())
    }

    /// Create no permission
    pub fn none() -> Self {
        Self::None
    }
}

/// Permissions for an API key
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ApiKeyPermissions {
    /// Permission for models
    #[serde(default)]
    pub models: ResourcePermission,
    /// Permission for knowledge bases
    #[serde(default)]
    pub knowledge_bases: ResourcePermission,
    /// Permission for prompts
    #[serde(default)]
    pub prompts: ResourcePermission,
    /// Permission for model chains
    #[serde(default)]
    pub chains: ResourcePermission,
    /// Whether admin operations are allowed
    #[serde(default)]
    pub admin: bool,
}

impl ApiKeyPermissions {
    /// Create new permissions with no access
    pub fn new() -> Self {
        Self::default()
    }

    /// Create permissions with full access
    pub fn full_access() -> Self {
        Self {
            models: ResourcePermission::All,
            knowledge_bases: ResourcePermission::All,
            prompts: ResourcePermission::All,
            chains: ResourcePermission::All,
            admin: true,
        }
    }

    /// Create read-only permissions for all resources
    pub fn read_only() -> Self {
        Self {
            models: ResourcePermission::All,
            knowledge_bases: ResourcePermission::All,
            prompts: ResourcePermission::All,
            chains: ResourcePermission::All,
            admin: false,
        }
    }

    /// Set model permissions
    pub fn with_models(mut self, permission: ResourcePermission) -> Self {
        self.models = permission;
        self
    }

    /// Set knowledge base permissions
    pub fn with_knowledge_bases(mut self, permission: ResourcePermission) -> Self {
        self.knowledge_bases = permission;
        self
    }

    /// Set prompt permissions
    pub fn with_prompts(mut self, permission: ResourcePermission) -> Self {
        self.prompts = permission;
        self
    }

    /// Set chain permissions
    pub fn with_chains(mut self, permission: ResourcePermission) -> Self {
        self.chains = permission;
        self
    }

    /// Set admin permission
    pub fn with_admin(mut self, admin: bool) -> Self {
        self.admin = admin;
        self
    }

    /// Check if model access is allowed
    pub fn can_access_model(&self, model_id: &str) -> bool {
        self.models.allows(model_id)
    }

    /// Check if knowledge base access is allowed
    pub fn can_access_knowledge_base(&self, kb_id: &str) -> bool {
        self.knowledge_bases.allows(kb_id)
    }

    /// Check if prompt access is allowed
    pub fn can_access_prompt(&self, prompt_id: &str) -> bool {
        self.prompts.allows(prompt_id)
    }

    /// Check if chain access is allowed
    pub fn can_access_chain(&self, chain_id: &str) -> bool {
        self.chains.allows(chain_id)
    }
}

/// Rate limit configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Whether rate limiting is enabled (disabled by default)
    #[serde(default)]
    pub enabled: bool,
    /// Maximum requests per minute (default: 500)
    pub requests_per_minute: u32,
    /// Maximum requests per hour
    pub requests_per_hour: u32,
    /// Maximum requests per day
    pub requests_per_day: u32,
    /// Maximum tokens per minute (for LLM requests)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens_per_minute: Option<u32>,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            requests_per_minute: 500,
            requests_per_hour: 5000,
            requests_per_day: 50000,
            tokens_per_minute: None,
        }
    }
}

impl RateLimitConfig {
    /// Create a new rate limit configuration (enabled by default when explicitly created)
    pub fn new(per_minute: u32, per_hour: u32, per_day: u32) -> Self {
        Self {
            enabled: true,
            requests_per_minute: per_minute,
            requests_per_hour: per_hour,
            requests_per_day: per_day,
            tokens_per_minute: None,
        }
    }

    /// Enable rate limiting
    pub fn enabled(mut self) -> Self {
        self.enabled = true;
        self
    }

    /// Disable rate limiting
    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }

    /// Set token rate limit
    pub fn with_tokens_per_minute(mut self, tokens: u32) -> Self {
        self.tokens_per_minute = Some(tokens);
        self
    }

    /// Create unlimited rate limits (effectively disabled)
    pub fn unlimited() -> Self {
        Self {
            enabled: false,
            requests_per_minute: u32::MAX,
            requests_per_hour: u32::MAX,
            requests_per_day: u32::MAX,
            tokens_per_minute: None,
        }
    }

    /// Check if rate limiting is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

/// API Key entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    /// Unique identifier for the key
    id: ApiKeyId,
    /// Display name for the key
    name: String,
    /// Description of the key's purpose
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    /// Hashed secret (the actual key)
    /// Format: algorithm$salt$hash (e.g., "argon2$randomsalt$hashedvalue")
    /// Note: This is stored in the database but never exposed in API responses (separate DTOs used).
    secret_hash: String,
    /// Key prefix for identification (first 8 chars of the key)
    key_prefix: String,
    /// Current status of the key
    status: ApiKeyStatus,
    /// Team that owns this API key (required)
    team_id: TeamId,
    /// Permissions granted to this key
    permissions: ApiKeyPermissions,
    /// Rate limit configuration
    rate_limits: RateLimitConfig,
    /// Expiration timestamp (None = never expires)
    #[serde(skip_serializing_if = "Option::is_none")]
    expires_at: Option<DateTime<Utc>>,
    /// Last time the key was used
    #[serde(skip_serializing_if = "Option::is_none")]
    last_used_at: Option<DateTime<Utc>>,
    /// Creation timestamp
    created_at: DateTime<Utc>,
    /// Last update timestamp
    updated_at: DateTime<Utc>,
    /// Owner/creator of the key
    #[serde(skip_serializing_if = "Option::is_none")]
    created_by: Option<String>,
}

impl ApiKey {
    /// Create a new API key
    pub fn new(
        id: ApiKeyId,
        name: impl Into<String>,
        secret_hash: impl Into<String>,
        key_prefix: impl Into<String>,
        team_id: TeamId,
    ) -> Self {
        let now = Utc::now();

        Self {
            id,
            name: name.into(),
            description: None,
            secret_hash: secret_hash.into(),
            key_prefix: key_prefix.into(),
            status: ApiKeyStatus::Active,
            team_id,
            permissions: ApiKeyPermissions::default(),
            rate_limits: RateLimitConfig::default(),
            expires_at: None,
            last_used_at: None,
            created_at: now,
            updated_at: now,
            created_by: None,
        }
    }

    /// Set description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set permissions
    pub fn with_permissions(mut self, permissions: ApiKeyPermissions) -> Self {
        self.permissions = permissions;
        self
    }

    /// Set rate limits
    pub fn with_rate_limits(mut self, rate_limits: RateLimitConfig) -> Self {
        self.rate_limits = rate_limits;
        self
    }

    /// Set expiration
    pub fn with_expiration(mut self, expires_at: DateTime<Utc>) -> Self {
        self.expires_at = Some(expires_at);
        self
    }

    /// Set creator
    pub fn with_created_by(mut self, created_by: impl Into<String>) -> Self {
        self.created_by = Some(created_by.into());
        self
    }

    // Getters

    pub fn id(&self) -> &ApiKeyId {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    pub fn secret_hash(&self) -> &str {
        &self.secret_hash
    }

    pub fn key_prefix(&self) -> &str {
        &self.key_prefix
    }

    pub fn status(&self) -> ApiKeyStatus {
        self.status
    }

    pub fn permissions(&self) -> &ApiKeyPermissions {
        &self.permissions
    }

    pub fn rate_limits(&self) -> &RateLimitConfig {
        &self.rate_limits
    }

    pub fn expires_at(&self) -> Option<DateTime<Utc>> {
        self.expires_at
    }

    pub fn last_used_at(&self) -> Option<DateTime<Utc>> {
        self.last_used_at
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    pub fn created_by(&self) -> Option<&str> {
        self.created_by.as_deref()
    }

    pub fn team_id(&self) -> &TeamId {
        &self.team_id
    }

    // Status checks

    /// Check if the key is currently valid and usable
    pub fn is_valid(&self) -> bool {
        if !self.status.is_usable() {
            return false;
        }

        if let Some(expires_at) = self.expires_at {
            if Utc::now() >= expires_at {
                return false;
            }
        }

        true
    }

    /// Check if the key has expired
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            Utc::now() >= expires_at
        } else {
            false
        }
    }

    // Mutators

    /// Update the name
    pub fn set_name(&mut self, name: impl Into<String>) {
        self.name = name.into();
        self.touch();
    }

    /// Update the description
    pub fn set_description(&mut self, description: Option<String>) {
        self.description = description;
        self.touch();
    }

    /// Update the status
    pub fn set_status(&mut self, status: ApiKeyStatus) {
        self.status = status;
        self.touch();
    }

    /// Update permissions
    pub fn set_permissions(&mut self, permissions: ApiKeyPermissions) {
        self.permissions = permissions;
        self.touch();
    }

    /// Update rate limits
    pub fn set_rate_limits(&mut self, rate_limits: RateLimitConfig) {
        self.rate_limits = rate_limits;
        self.touch();
    }

    /// Update expiration
    pub fn set_expiration(&mut self, expires_at: Option<DateTime<Utc>>) {
        self.expires_at = expires_at;
        self.touch();
    }

    /// Record key usage
    pub fn record_usage(&mut self) {
        self.last_used_at = Some(Utc::now());
    }

    /// Suspend the key
    pub fn suspend(&mut self) {
        self.status = ApiKeyStatus::Suspended;
        self.touch();
    }

    /// Revoke the key
    pub fn revoke(&mut self) {
        self.status = ApiKeyStatus::Revoked;
        self.touch();
    }

    /// Reactivate a suspended key
    pub fn activate(&mut self) {
        if self.status == ApiKeyStatus::Suspended {
            self.status = ApiKeyStatus::Active;
            self.touch();
        }
    }

    /// Update the team ownership
    pub fn set_team_id(&mut self, team_id: TeamId) {
        self.team_id = team_id;
        self.touch();
    }

    fn touch(&mut self) {
        self.updated_at = Utc::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_api_key(id: &str, name: &str) -> ApiKey {
        let key_id = ApiKeyId::new(id).unwrap();
        let team_id = TeamId::administrators();
        ApiKey::new(key_id, name, "hashed_secret", "pk_test_", team_id)
    }

    #[test]
    fn test_api_key_id_valid() {
        let id = ApiKeyId::new("my-api-key-1").unwrap();
        assert_eq!(id.as_str(), "my-api-key-1");
    }

    #[test]
    fn test_api_key_id_invalid() {
        assert!(ApiKeyId::new("").is_err());
        assert!(ApiKeyId::new("my_key").is_err());
        assert!(ApiKeyId::new("-key").is_err());
    }

    #[test]
    fn test_api_key_status() {
        assert!(ApiKeyStatus::Active.is_usable());
        assert!(!ApiKeyStatus::Suspended.is_usable());
        assert!(!ApiKeyStatus::Revoked.is_usable());
        assert!(!ApiKeyStatus::Expired.is_usable());
    }

    #[test]
    fn test_resource_permission_all() {
        let perm = ResourcePermission::All;
        assert!(perm.allows("any-resource"));
        assert!(perm.has_any_access());
    }

    #[test]
    fn test_resource_permission_specific() {
        let perm = ResourcePermission::specific(vec!["res-1", "res-2"]);
        assert!(perm.allows("res-1"));
        assert!(perm.allows("res-2"));
        assert!(!perm.allows("res-3"));
        assert!(perm.has_any_access());
    }

    #[test]
    fn test_resource_permission_none() {
        let perm = ResourcePermission::None;
        assert!(!perm.allows("any-resource"));
        assert!(!perm.has_any_access());
    }

    #[test]
    fn test_api_key_permissions() {
        let perms = ApiKeyPermissions::new()
            .with_models(ResourcePermission::all())
            .with_knowledge_bases(ResourcePermission::specific(vec!["kb-1"]));

        assert!(perms.can_access_model("any-model"));
        assert!(perms.can_access_knowledge_base("kb-1"));
        assert!(!perms.can_access_knowledge_base("kb-2"));
        assert!(!perms.can_access_prompt("any-prompt"));
    }

    #[test]
    fn test_api_key_permissions_full_access() {
        let perms = ApiKeyPermissions::full_access();

        assert!(perms.can_access_model("any"));
        assert!(perms.can_access_knowledge_base("any"));
        assert!(perms.can_access_prompt("any"));
        assert!(perms.can_access_chain("any"));
        assert!(perms.admin);
    }

    #[test]
    fn test_rate_limit_config() {
        let config = RateLimitConfig::new(60, 1000, 10000).with_tokens_per_minute(100000);

        assert_eq!(config.requests_per_minute, 60);
        assert_eq!(config.requests_per_hour, 1000);
        assert_eq!(config.requests_per_day, 10000);
        assert_eq!(config.tokens_per_minute, Some(100000));
    }

    #[test]
    fn test_api_key_creation() {
        let key = create_test_api_key("test-key", "Test Key")
            .with_description("A test API key")
            .with_permissions(ApiKeyPermissions::read_only());

        assert_eq!(key.name(), "Test Key");
        assert_eq!(key.description(), Some("A test API key"));
        assert_eq!(key.key_prefix(), "pk_test_");
        assert_eq!(key.team_id().as_str(), "administrators");
        assert!(key.is_valid());
        assert!(!key.is_expired());
    }

    #[test]
    fn test_api_key_with_team() {
        let key_id = ApiKeyId::new("test-key").unwrap();
        let team_id = TeamId::new("my-team").unwrap();
        let key = ApiKey::new(key_id, "Test Key", "hash", "pk_", team_id);

        assert_eq!(key.team_id().as_str(), "my-team");
    }

    #[test]
    fn test_api_key_expiration() {
        let past = Utc::now() - chrono::Duration::hours(1);
        let key = create_test_api_key("test-key", "Test Key").with_expiration(past);

        assert!(key.is_expired());
        assert!(!key.is_valid());
    }

    #[test]
    fn test_api_key_status_changes() {
        let mut key = create_test_api_key("test-key", "Test Key");

        assert!(key.is_valid());

        key.suspend();
        assert!(!key.is_valid());
        assert_eq!(key.status(), ApiKeyStatus::Suspended);

        key.activate();
        assert!(key.is_valid());
        assert_eq!(key.status(), ApiKeyStatus::Active);

        key.revoke();
        assert!(!key.is_valid());
        assert_eq!(key.status(), ApiKeyStatus::Revoked);

        // Can't reactivate a revoked key
        key.activate();
        assert_eq!(key.status(), ApiKeyStatus::Revoked);
    }

    #[test]
    fn test_api_key_record_usage() {
        let mut key = create_test_api_key("test-key", "Test Key");

        assert!(key.last_used_at().is_none());

        key.record_usage();
        assert!(key.last_used_at().is_some());
    }

    #[test]
    fn test_api_key_set_team_id() {
        let mut key = create_test_api_key("test-key", "Test Key");
        let new_team_id = TeamId::new("new-team").unwrap();

        key.set_team_id(new_team_id);
        assert_eq!(key.team_id().as_str(), "new-team");
    }
}
