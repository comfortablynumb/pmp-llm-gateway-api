//! Operation-specific errors

use std::fmt;

/// Errors that can occur in operation handling
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OperationError {
    /// Invalid operation ID format
    InvalidId(String),

    /// Invalid state transition
    InvalidStateTransition {
        from: String,
        to: String,
        reason: String,
    },

    /// Operation not found
    NotFound(String),

    /// Operation already exists
    AlreadyExists(String),

    /// Operation cannot be cancelled
    CannotCancel(String),

    /// Validation error
    Validation(String),
}

impl OperationError {
    pub fn invalid_id(message: impl Into<String>) -> Self {
        Self::InvalidId(message.into())
    }

    pub fn invalid_transition(from: &str, to: &str, reason: impl Into<String>) -> Self {
        Self::InvalidStateTransition {
            from: from.to_string(),
            to: to.to_string(),
            reason: reason.into(),
        }
    }

    pub fn not_found(id: impl Into<String>) -> Self {
        Self::NotFound(id.into())
    }

    pub fn already_exists(id: impl Into<String>) -> Self {
        Self::AlreadyExists(id.into())
    }

    pub fn cannot_cancel(reason: impl Into<String>) -> Self {
        Self::CannotCancel(reason.into())
    }

    pub fn validation(message: impl Into<String>) -> Self {
        Self::Validation(message.into())
    }
}

impl fmt::Display for OperationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidId(msg) => write!(f, "Invalid operation ID: {}", msg),
            Self::InvalidStateTransition { from, to, reason } => {
                write!(
                    f,
                    "Invalid state transition from '{}' to '{}': {}",
                    from, to, reason
                )
            }
            Self::NotFound(id) => write!(f, "Operation '{}' not found", id),
            Self::AlreadyExists(id) => write!(f, "Operation '{}' already exists", id),
            Self::CannotCancel(reason) => write!(f, "Cannot cancel operation: {}", reason),
            Self::Validation(msg) => write!(f, "Validation error: {}", msg),
        }
    }
}

impl std::error::Error for OperationError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = OperationError::invalid_id("bad-format");
        assert!(err.to_string().contains("Invalid operation ID"));

        let err = OperationError::invalid_transition("pending", "completed", "must run first");
        assert!(err.to_string().contains("Invalid state transition"));

        let err = OperationError::not_found("op-123");
        assert!(err.to_string().contains("not found"));

        let err = OperationError::cannot_cancel("already completed");
        assert!(err.to_string().contains("Cannot cancel"));
    }

    #[test]
    fn test_error_equality() {
        let err1 = OperationError::not_found("op-123");
        let err2 = OperationError::not_found("op-123");
        assert_eq!(err1, err2);

        let err3 = OperationError::not_found("op-456");
        assert_ne!(err1, err3);
    }
}
