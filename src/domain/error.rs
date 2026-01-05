use thiserror::Error;

/// Core domain errors
#[derive(Debug, Error)]
pub enum DomainError {
    #[error("Not found: {message}")]
    NotFound { message: String },

    #[error("Validation error: {message}")]
    Validation { message: String },

    #[error("Invalid ID format: {message}")]
    InvalidId { message: String },

    #[error("Credential error: {message}")]
    Credential { message: String },

    #[error("Provider error: {provider} - {message}")]
    Provider { provider: String, message: String },

    #[error("Configuration error: {message}")]
    Configuration { message: String },

    #[error("Conflict: {message}")]
    Conflict { message: String },

    #[error("Internal error: {message}")]
    Internal { message: String },

    #[error("Storage error: {message}")]
    Storage { message: String },

    #[error("Cache error: {message}")]
    Cache { message: String },

    #[error("Knowledge base error: {0}")]
    KnowledgeBase(String),
}

impl DomainError {
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::NotFound {
            message: message.into(),
        }
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self::Conflict {
            message: message.into(),
        }
    }

    pub fn validation(message: impl Into<String>) -> Self {
        Self::Validation {
            message: message.into(),
        }
    }

    pub fn invalid_id(message: impl Into<String>) -> Self {
        Self::InvalidId {
            message: message.into(),
        }
    }

    pub fn credential(message: impl Into<String>) -> Self {
        Self::Credential {
            message: message.into(),
        }
    }

    pub fn provider(provider: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Provider {
            provider: provider.into(),
            message: message.into(),
        }
    }

    pub fn configuration(message: impl Into<String>) -> Self {
        Self::Configuration {
            message: message.into(),
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal {
            message: message.into(),
        }
    }

    pub fn storage(message: impl Into<String>) -> Self {
        Self::Storage {
            message: message.into(),
        }
    }

    pub fn cache(message: impl Into<String>) -> Self {
        Self::Cache {
            message: message.into(),
        }
    }

    pub fn knowledge_base(message: impl Into<String>) -> Self {
        Self::KnowledgeBase(message.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_not_found_error() {
        let error = DomainError::not_found("Model 'test-id' not found");
        assert_eq!(error.to_string(), "Not found: Model 'test-id' not found");
    }

    #[test]
    fn test_validation_error() {
        let error = DomainError::validation("Invalid input");
        assert_eq!(error.to_string(), "Validation error: Invalid input");
    }

    #[test]
    fn test_conflict_error() {
        let error = DomainError::conflict("Resource already exists");
        assert_eq!(error.to_string(), "Conflict: Resource already exists");
    }
}
