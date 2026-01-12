//! Team validation

use thiserror::Error;

/// Errors that can occur during team validation
#[derive(Debug, Error, Clone, PartialEq)]
pub enum TeamValidationError {
    #[error("Team ID cannot be empty")]
    EmptyId,

    #[error("Team ID cannot exceed {0} characters")]
    IdTooLong(usize),

    #[error("Team ID can only contain alphanumeric characters and hyphens")]
    InvalidIdCharacters,

    #[error("Team ID cannot start or end with a hyphen")]
    InvalidIdFormat,

    #[error("Team name cannot be empty")]
    EmptyName,

    #[error("Team name cannot exceed {0} characters")]
    NameTooLong(usize),
}

const MAX_TEAM_ID_LENGTH: usize = 50;
const MAX_TEAM_NAME_LENGTH: usize = 100;

/// Validate a team ID
pub fn validate_team_id(id: &str) -> Result<(), TeamValidationError> {
    if id.is_empty() {
        return Err(TeamValidationError::EmptyId);
    }

    if id.len() > MAX_TEAM_ID_LENGTH {
        return Err(TeamValidationError::IdTooLong(MAX_TEAM_ID_LENGTH));
    }

    if !id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-')
    {
        return Err(TeamValidationError::InvalidIdCharacters);
    }

    if id.starts_with('-') || id.ends_with('-') {
        return Err(TeamValidationError::InvalidIdFormat);
    }

    Ok(())
}

/// Validate a team name
pub fn validate_team_name(name: &str) -> Result<(), TeamValidationError> {
    if name.is_empty() {
        return Err(TeamValidationError::EmptyName);
    }

    if name.len() > MAX_TEAM_NAME_LENGTH {
        return Err(TeamValidationError::NameTooLong(MAX_TEAM_NAME_LENGTH));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_team_id() {
        assert!(validate_team_id("my-team").is_ok());
        assert!(validate_team_id("team123").is_ok());
        assert!(validate_team_id("Team-123").is_ok());
        assert!(validate_team_id("administrators").is_ok());
    }

    #[test]
    fn test_empty_team_id() {
        assert_eq!(validate_team_id(""), Err(TeamValidationError::EmptyId));
    }

    #[test]
    fn test_team_id_too_long() {
        let long_id = "a".repeat(51);
        assert_eq!(
            validate_team_id(&long_id),
            Err(TeamValidationError::IdTooLong(50))
        );
    }

    #[test]
    fn test_invalid_team_id_characters() {
        assert_eq!(
            validate_team_id("team_name"),
            Err(TeamValidationError::InvalidIdCharacters)
        );
        assert_eq!(
            validate_team_id("team.name"),
            Err(TeamValidationError::InvalidIdCharacters)
        );
    }

    #[test]
    fn test_invalid_team_id_format() {
        assert_eq!(
            validate_team_id("-team"),
            Err(TeamValidationError::InvalidIdFormat)
        );
        assert_eq!(
            validate_team_id("team-"),
            Err(TeamValidationError::InvalidIdFormat)
        );
    }

    #[test]
    fn test_valid_team_name() {
        assert!(validate_team_name("My Team").is_ok());
        assert!(validate_team_name("Administrators").is_ok());
        assert!(validate_team_name("Team with spaces & symbols!").is_ok());
    }

    #[test]
    fn test_empty_team_name() {
        assert_eq!(validate_team_name(""), Err(TeamValidationError::EmptyName));
    }

    #[test]
    fn test_team_name_too_long() {
        let long_name = "a".repeat(101);
        assert_eq!(
            validate_team_name(&long_name),
            Err(TeamValidationError::NameTooLong(100))
        );
    }
}
