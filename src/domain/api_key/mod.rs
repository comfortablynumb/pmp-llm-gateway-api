//! API Key domain
//!
//! This module provides domain types and traits for API key management,
//! including key generation, validation, permissions, and rate limiting.

mod entity;
mod repository;
mod validation;

pub use entity::{
    ApiKey, ApiKeyId, ApiKeyPermissions, ApiKeyStatus, RateLimitConfig, ResourcePermission,
};
pub use repository::ApiKeyRepository;
pub use validation::{validate_api_key_id, ApiKeyValidationError};
