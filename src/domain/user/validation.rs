//! User validation utilities

use thiserror::Error;

/// Errors that can occur during user validation
#[derive(Debug, Error, Clone, PartialEq)]
pub enum UserValidationError {
    #[error("User ID cannot be empty")]
    EmptyId,

    #[error("User ID exceeds maximum length of {0} characters")]
    IdTooLong(usize),

    #[error("User ID must start with a letter or number")]
    InvalidIdStart,

    #[error("User ID must end with a letter or number")]
    InvalidIdEnd,

    #[error("User ID contains invalid character: '{0}'. Only alphanumeric characters and hyphens are allowed")]
    InvalidIdCharacter(char),

    #[error("User ID cannot contain consecutive hyphens")]
    ConsecutiveHyphens,

    #[error("Username cannot be empty")]
    EmptyUsername,

    #[error("Username exceeds maximum length of {0} characters")]
    UsernameTooLong(usize),

    #[error("Username is too short. Minimum length is {0} characters")]
    UsernameTooShort(usize),

    #[error("Username contains invalid character: '{0}'. Only alphanumeric characters, underscores, and hyphens are allowed")]
    InvalidUsernameCharacter(char),

    #[error("Password is too short. Minimum length is {0} characters")]
    PasswordTooShort(usize),

    #[error("Password exceeds maximum length of {0} characters")]
    PasswordTooLong(usize),
}

const MAX_USER_ID_LENGTH: usize = 50;
const MIN_USERNAME_LENGTH: usize = 3;
const MAX_USERNAME_LENGTH: usize = 50;
const MIN_PASSWORD_LENGTH: usize = 8;
const MAX_PASSWORD_LENGTH: usize = 128;

/// Validate a user ID
///
/// Rules:
/// - Cannot be empty
/// - Maximum 50 characters
/// - Only alphanumeric characters and hyphens
/// - Must start and end with alphanumeric
/// - No consecutive hyphens
pub fn validate_user_id(id: &str) -> Result<(), UserValidationError> {
    if id.is_empty() {
        return Err(UserValidationError::EmptyId);
    }

    if id.len() > MAX_USER_ID_LENGTH {
        return Err(UserValidationError::IdTooLong(MAX_USER_ID_LENGTH));
    }

    let chars: Vec<char> = id.chars().collect();

    if !chars[0].is_ascii_alphanumeric() {
        return Err(UserValidationError::InvalidIdStart);
    }

    if !chars[chars.len() - 1].is_ascii_alphanumeric() {
        return Err(UserValidationError::InvalidIdEnd);
    }

    let mut prev_hyphen = false;

    for c in &chars {
        if *c == '-' {
            if prev_hyphen {
                return Err(UserValidationError::ConsecutiveHyphens);
            }
            prev_hyphen = true;
        } else if c.is_ascii_alphanumeric() {
            prev_hyphen = false;
        } else {
            return Err(UserValidationError::InvalidIdCharacter(*c));
        }
    }

    Ok(())
}

/// Validate a username
///
/// Rules:
/// - Cannot be empty
/// - Minimum 3 characters
/// - Maximum 50 characters
/// - Only alphanumeric characters, underscores, and hyphens
pub fn validate_username(username: &str) -> Result<(), UserValidationError> {
    if username.is_empty() {
        return Err(UserValidationError::EmptyUsername);
    }

    if username.len() < MIN_USERNAME_LENGTH {
        return Err(UserValidationError::UsernameTooShort(MIN_USERNAME_LENGTH));
    }

    if username.len() > MAX_USERNAME_LENGTH {
        return Err(UserValidationError::UsernameTooLong(MAX_USERNAME_LENGTH));
    }

    for c in username.chars() {
        if !c.is_ascii_alphanumeric() && c != '_' && c != '-' {
            return Err(UserValidationError::InvalidUsernameCharacter(c));
        }
    }

    Ok(())
}

/// Validate a password
///
/// Rules:
/// - Minimum 8 characters
/// - Maximum 128 characters
pub fn validate_password(password: &str) -> Result<(), UserValidationError> {
    if password.len() < MIN_PASSWORD_LENGTH {
        return Err(UserValidationError::PasswordTooShort(MIN_PASSWORD_LENGTH));
    }

    if password.len() > MAX_PASSWORD_LENGTH {
        return Err(UserValidationError::PasswordTooLong(MAX_PASSWORD_LENGTH));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // User ID tests
    #[test]
    fn test_valid_user_ids() {
        assert!(validate_user_id("admin").is_ok());
        assert!(validate_user_id("user-1").is_ok());
        assert!(validate_user_id("a").is_ok());
        assert!(validate_user_id("test-user-123").is_ok());
    }

    #[test]
    fn test_empty_user_id() {
        assert_eq!(validate_user_id(""), Err(UserValidationError::EmptyId));
    }

    #[test]
    fn test_user_id_too_long() {
        let long_id = "a".repeat(51);
        assert_eq!(
            validate_user_id(&long_id),
            Err(UserValidationError::IdTooLong(50))
        );
    }

    #[test]
    fn test_user_id_invalid_start() {
        assert_eq!(
            validate_user_id("-user"),
            Err(UserValidationError::InvalidIdStart)
        );
    }

    #[test]
    fn test_user_id_invalid_end() {
        assert_eq!(
            validate_user_id("user-"),
            Err(UserValidationError::InvalidIdEnd)
        );
    }

    #[test]
    fn test_user_id_invalid_character() {
        assert_eq!(
            validate_user_id("user_name"),
            Err(UserValidationError::InvalidIdCharacter('_'))
        );
    }

    #[test]
    fn test_user_id_consecutive_hyphens() {
        assert_eq!(
            validate_user_id("user--name"),
            Err(UserValidationError::ConsecutiveHyphens)
        );
    }

    // Username tests
    #[test]
    fn test_valid_usernames() {
        assert!(validate_username("admin").is_ok());
        assert!(validate_username("user_name").is_ok());
        assert!(validate_username("user-name").is_ok());
        assert!(validate_username("User123").is_ok());
    }

    #[test]
    fn test_empty_username() {
        assert_eq!(
            validate_username(""),
            Err(UserValidationError::EmptyUsername)
        );
    }

    #[test]
    fn test_username_too_short() {
        assert_eq!(
            validate_username("ab"),
            Err(UserValidationError::UsernameTooShort(3))
        );
    }

    #[test]
    fn test_username_too_long() {
        let long_username = "a".repeat(51);
        assert_eq!(
            validate_username(&long_username),
            Err(UserValidationError::UsernameTooLong(50))
        );
    }

    #[test]
    fn test_username_invalid_character() {
        assert_eq!(
            validate_username("user@name"),
            Err(UserValidationError::InvalidUsernameCharacter('@'))
        );
    }

    // Password tests
    #[test]
    fn test_valid_passwords() {
        assert!(validate_password("password123").is_ok());
        assert!(validate_password("P@ssw0rd!").is_ok());
        assert!(validate_password("12345678").is_ok());
    }

    #[test]
    fn test_password_too_short() {
        assert_eq!(
            validate_password("1234567"),
            Err(UserValidationError::PasswordTooShort(8))
        );
    }

    #[test]
    fn test_password_too_long() {
        let long_password = "a".repeat(129);
        assert_eq!(
            validate_password(&long_password),
            Err(UserValidationError::PasswordTooLong(128))
        );
    }
}
