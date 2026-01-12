//! Experiment validation utilities

use thiserror::Error;

/// Maximum length for experiment IDs
pub const MAX_EXPERIMENT_ID_LENGTH: usize = 50;

/// Maximum length for variant IDs
pub const MAX_VARIANT_ID_LENGTH: usize = 50;

/// Validation errors for experiments and variants
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ExperimentValidationError {
    #[error("Experiment ID cannot be empty")]
    EmptyId,

    #[error("Experiment ID exceeds maximum length of {0} characters")]
    IdTooLong(usize),

    #[error("Experiment ID must start with a letter or number")]
    InvalidIdStart,

    #[error("Experiment ID must end with a letter or number")]
    InvalidIdEnd,

    #[error("Experiment ID contains invalid character: '{0}'")]
    InvalidIdCharacter(char),

    #[error("Experiment ID cannot contain consecutive hyphens")]
    ConsecutiveHyphens,

    #[error("Variant ID cannot be empty")]
    EmptyVariantId,

    #[error("Variant ID exceeds maximum length of {0} characters")]
    VariantIdTooLong(usize),

    #[error("Variant ID must start with a letter or number")]
    InvalidVariantIdStart,

    #[error("Variant ID must end with a letter or number")]
    InvalidVariantIdEnd,

    #[error("Variant ID contains invalid character: '{0}'")]
    InvalidVariantIdCharacter(char),

    #[error("Variant ID cannot contain consecutive hyphens")]
    VariantIdConsecutiveHyphens,

    #[error("Traffic allocations must sum to 100, got {0}")]
    InvalidTrafficSum(u8),

    #[error("Experiment must have at least 2 variants")]
    InsufficientVariants,

    #[error("Duplicate variant ID: '{0}'")]
    DuplicateVariantId(String),

    #[error("Model '{0}' not found")]
    ModelNotFound(String),

    #[error("Invalid experiment status transition from {0} to {1}")]
    InvalidStatusTransition(String, String),

    #[error("Traffic allocated to unknown variant: '{0}'")]
    UnknownVariantInAllocation(String),
}

/// Validate an experiment ID
pub fn validate_experiment_id(id: &str) -> Result<(), ExperimentValidationError> {
    if id.is_empty() {
        return Err(ExperimentValidationError::EmptyId);
    }

    if id.len() > MAX_EXPERIMENT_ID_LENGTH {
        return Err(ExperimentValidationError::IdTooLong(MAX_EXPERIMENT_ID_LENGTH));
    }

    let first_char = id.chars().next().unwrap();

    if !first_char.is_ascii_alphanumeric() {
        return Err(ExperimentValidationError::InvalidIdStart);
    }

    let last_char = id.chars().last().unwrap();

    if !last_char.is_ascii_alphanumeric() {
        return Err(ExperimentValidationError::InvalidIdEnd);
    }

    let mut prev_was_hyphen = false;

    for ch in id.chars() {
        if ch == '-' {
            if prev_was_hyphen {
                return Err(ExperimentValidationError::ConsecutiveHyphens);
            }
            prev_was_hyphen = true;
        } else if ch.is_ascii_alphanumeric() {
            prev_was_hyphen = false;
        } else {
            return Err(ExperimentValidationError::InvalidIdCharacter(ch));
        }
    }

    Ok(())
}

/// Validate a variant ID
pub fn validate_variant_id(id: &str) -> Result<(), ExperimentValidationError> {
    if id.is_empty() {
        return Err(ExperimentValidationError::EmptyVariantId);
    }

    if id.len() > MAX_VARIANT_ID_LENGTH {
        return Err(ExperimentValidationError::VariantIdTooLong(
            MAX_VARIANT_ID_LENGTH,
        ));
    }

    let first_char = id.chars().next().unwrap();

    if !first_char.is_ascii_alphanumeric() {
        return Err(ExperimentValidationError::InvalidVariantIdStart);
    }

    let last_char = id.chars().last().unwrap();

    if !last_char.is_ascii_alphanumeric() {
        return Err(ExperimentValidationError::InvalidVariantIdEnd);
    }

    let mut prev_was_hyphen = false;

    for ch in id.chars() {
        if ch == '-' {
            if prev_was_hyphen {
                return Err(ExperimentValidationError::VariantIdConsecutiveHyphens);
            }
            prev_was_hyphen = true;
        } else if ch.is_ascii_alphanumeric() {
            prev_was_hyphen = false;
        } else {
            return Err(ExperimentValidationError::InvalidVariantIdCharacter(ch));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    mod experiment_id_validation {
        use super::*;

        #[test]
        fn test_valid_experiment_ids() {
            assert!(validate_experiment_id("exp-1").is_ok());
            assert!(validate_experiment_id("my-experiment").is_ok());
            assert!(validate_experiment_id("test123").is_ok());
            assert!(validate_experiment_id("a").is_ok());
            assert!(validate_experiment_id("ab-cd-ef").is_ok());
            assert!(validate_experiment_id("experiment-2024-01").is_ok());
        }

        #[test]
        fn test_empty_id() {
            assert_eq!(
                validate_experiment_id(""),
                Err(ExperimentValidationError::EmptyId)
            );
        }

        #[test]
        fn test_id_too_long() {
            let long_id = "a".repeat(51);
            assert_eq!(
                validate_experiment_id(&long_id),
                Err(ExperimentValidationError::IdTooLong(50))
            );
        }

        #[test]
        fn test_invalid_start() {
            assert_eq!(
                validate_experiment_id("-abc"),
                Err(ExperimentValidationError::InvalidIdStart)
            );
            assert_eq!(
                validate_experiment_id("_abc"),
                Err(ExperimentValidationError::InvalidIdStart)
            );
        }

        #[test]
        fn test_invalid_end() {
            assert_eq!(
                validate_experiment_id("abc-"),
                Err(ExperimentValidationError::InvalidIdEnd)
            );
        }

        #[test]
        fn test_invalid_character() {
            assert_eq!(
                validate_experiment_id("abc_def"),
                Err(ExperimentValidationError::InvalidIdCharacter('_'))
            );
            assert_eq!(
                validate_experiment_id("abc.def"),
                Err(ExperimentValidationError::InvalidIdCharacter('.'))
            );
            assert_eq!(
                validate_experiment_id("abc def"),
                Err(ExperimentValidationError::InvalidIdCharacter(' '))
            );
        }

        #[test]
        fn test_consecutive_hyphens() {
            assert_eq!(
                validate_experiment_id("abc--def"),
                Err(ExperimentValidationError::ConsecutiveHyphens)
            );
        }
    }

    mod variant_id_validation {
        use super::*;

        #[test]
        fn test_valid_variant_ids() {
            assert!(validate_variant_id("control").is_ok());
            assert!(validate_variant_id("variant-a").is_ok());
            assert!(validate_variant_id("v1").is_ok());
            assert!(validate_variant_id("treatment-group-1").is_ok());
        }

        #[test]
        fn test_empty_variant_id() {
            assert_eq!(
                validate_variant_id(""),
                Err(ExperimentValidationError::EmptyVariantId)
            );
        }

        #[test]
        fn test_variant_id_too_long() {
            let long_id = "v".repeat(51);
            assert_eq!(
                validate_variant_id(&long_id),
                Err(ExperimentValidationError::VariantIdTooLong(50))
            );
        }

        #[test]
        fn test_variant_invalid_start() {
            assert_eq!(
                validate_variant_id("-variant"),
                Err(ExperimentValidationError::InvalidVariantIdStart)
            );
        }

        #[test]
        fn test_variant_invalid_end() {
            assert_eq!(
                validate_variant_id("variant-"),
                Err(ExperimentValidationError::InvalidVariantIdEnd)
            );
        }

        #[test]
        fn test_variant_consecutive_hyphens() {
            assert_eq!(
                validate_variant_id("variant--a"),
                Err(ExperimentValidationError::VariantIdConsecutiveHyphens)
            );
        }
    }
}
