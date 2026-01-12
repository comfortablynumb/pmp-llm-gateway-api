//! API Key validation utilities

use thiserror::Error;

/// Errors that can occur during API key validation
#[derive(Debug, Error, Clone, PartialEq)]
pub enum ApiKeyValidationError {
    #[error("API key ID cannot be empty")]
    EmptyId,

    #[error("API key ID exceeds maximum length of {0} characters")]
    TooLong(usize),

    #[error("API key ID must start with a letter or number")]
    InvalidStart,

    #[error("API key ID must end with a letter or number")]
    InvalidEnd,

    #[error("API key ID contains invalid character: '{0}'. Only alphanumeric characters and hyphens are allowed")]
    InvalidCharacter(char),

    #[error("API key ID cannot contain consecutive hyphens")]
    ConsecutiveHyphens,
}

const MAX_API_KEY_ID_LENGTH: usize = 50;

/// Validate an API key ID
///
/// Rules:
/// - Cannot be empty
/// - Maximum 50 characters
/// - Only alphanumeric characters and hyphens
/// - Must start and end with alphanumeric
/// - No consecutive hyphens
pub fn validate_api_key_id(id: &str) -> Result<(), ApiKeyValidationError> {
    if id.is_empty() {
        return Err(ApiKeyValidationError::EmptyId);
    }

    if id.len() > MAX_API_KEY_ID_LENGTH {
        return Err(ApiKeyValidationError::TooLong(MAX_API_KEY_ID_LENGTH));
    }

    let chars: Vec<char> = id.chars().collect();

    // Check first character
    if !chars[0].is_ascii_alphanumeric() {
        return Err(ApiKeyValidationError::InvalidStart);
    }

    // Check last character
    if !chars[chars.len() - 1].is_ascii_alphanumeric() {
        return Err(ApiKeyValidationError::InvalidEnd);
    }

    // Check all characters and consecutive hyphens
    let mut prev_hyphen = false;

    for c in &chars {
        if *c == '-' {
            if prev_hyphen {
                return Err(ApiKeyValidationError::ConsecutiveHyphens);
            }
            prev_hyphen = true;
        } else if c.is_ascii_alphanumeric() {
            prev_hyphen = false;
        } else {
            return Err(ApiKeyValidationError::InvalidCharacter(*c));
        }
    }

    Ok(())
}

/// Validate an API key secret format
///
/// The secret should be a base64-encoded string of sufficient length
#[allow(dead_code)]
pub fn validate_api_key_secret(secret: &str) -> Result<(), ApiKeyValidationError> {
    if secret.is_empty() {
        return Err(ApiKeyValidationError::EmptyId);
    }

    // Secret should be at least 32 characters for security
    if secret.len() < 32 {
        return Err(ApiKeyValidationError::TooLong(32)); // Reusing error for "too short"
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_api_key_ids() {
        assert!(validate_api_key_id("my-api-key").is_ok());
        assert!(validate_api_key_id("key123").is_ok());
        assert!(validate_api_key_id("a").is_ok());
        assert!(validate_api_key_id("test-key-1").is_ok());
        assert!(validate_api_key_id("UPPER-lower-123").is_ok());
    }

    #[test]
    fn test_empty_id() {
        assert_eq!(
            validate_api_key_id(""),
            Err(ApiKeyValidationError::EmptyId)
        );
    }

    #[test]
    fn test_too_long_id() {
        let long_id = "a".repeat(51);
        assert_eq!(
            validate_api_key_id(&long_id),
            Err(ApiKeyValidationError::TooLong(50))
        );
    }

    #[test]
    fn test_invalid_start() {
        assert_eq!(
            validate_api_key_id("-key"),
            Err(ApiKeyValidationError::InvalidStart)
        );
    }

    #[test]
    fn test_invalid_end() {
        assert_eq!(
            validate_api_key_id("key-"),
            Err(ApiKeyValidationError::InvalidEnd)
        );
    }

    #[test]
    fn test_invalid_character() {
        assert_eq!(
            validate_api_key_id("my_key"),
            Err(ApiKeyValidationError::InvalidCharacter('_'))
        );
        assert_eq!(
            validate_api_key_id("my.key"),
            Err(ApiKeyValidationError::InvalidCharacter('.'))
        );
        assert_eq!(
            validate_api_key_id("my key"),
            Err(ApiKeyValidationError::InvalidCharacter(' '))
        );
    }

    #[test]
    fn test_consecutive_hyphens() {
        assert_eq!(
            validate_api_key_id("my--key"),
            Err(ApiKeyValidationError::ConsecutiveHyphens)
        );
    }

    #[test]
    fn test_max_length_id() {
        let max_id = "a".repeat(50);
        assert!(validate_api_key_id(&max_id).is_ok());
    }
}
