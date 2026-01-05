//! Model validation utilities

use std::fmt;

use once_cell::sync::Lazy;
use regex::Regex;

/// Maximum length for model IDs
pub const MAX_MODEL_ID_LENGTH: usize = 50;

/// Regex pattern for valid model IDs (alphanumeric + hyphens)
static MODEL_ID_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^[a-zA-Z0-9][a-zA-Z0-9-]*[a-zA-Z0-9]$|^[a-zA-Z0-9]$").unwrap());

/// Model validation errors
#[derive(Debug, Clone, PartialEq)]
pub enum ModelValidationError {
    /// Model ID is empty
    EmptyId,
    /// Model ID exceeds maximum length
    IdTooLong { length: usize, max: usize },
    /// Model ID contains invalid characters
    InvalidIdFormat { id: String },
    /// Temperature out of valid range
    InvalidTemperature { value: f32, min: f32, max: f32 },
    /// Top-p out of valid range
    InvalidTopP { value: f32, min: f32, max: f32 },
    /// Presence penalty out of valid range
    InvalidPresencePenalty { value: f32, min: f32, max: f32 },
    /// Frequency penalty out of valid range
    InvalidFrequencyPenalty { value: f32, min: f32, max: f32 },
    /// Max tokens is zero
    InvalidMaxTokens,
}

impl fmt::Display for ModelValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyId => write!(f, "Model ID cannot be empty"),
            Self::IdTooLong { length, max } => {
                write!(f, "Model ID too long: {} characters (max {})", length, max)
            }
            Self::InvalidIdFormat { id } => {
                write!(
                    f,
                    "Invalid model ID format '{}': must be alphanumeric with hyphens, cannot start or end with hyphen",
                    id
                )
            }
            Self::InvalidTemperature { value, min, max } => {
                write!(
                    f,
                    "Invalid temperature {}: must be between {} and {}",
                    value, min, max
                )
            }
            Self::InvalidTopP { value, min, max } => {
                write!(
                    f,
                    "Invalid top_p {}: must be between {} and {}",
                    value, min, max
                )
            }
            Self::InvalidPresencePenalty { value, min, max } => {
                write!(
                    f,
                    "Invalid presence_penalty {}: must be between {} and {}",
                    value, min, max
                )
            }
            Self::InvalidFrequencyPenalty { value, min, max } => {
                write!(
                    f,
                    "Invalid frequency_penalty {}: must be between {} and {}",
                    value, min, max
                )
            }
            Self::InvalidMaxTokens => write!(f, "max_tokens must be greater than 0"),
        }
    }
}

impl std::error::Error for ModelValidationError {}

/// Validate a model ID
pub fn validate_model_id(id: &str) -> Result<(), ModelValidationError> {
    if id.is_empty() {
        return Err(ModelValidationError::EmptyId);
    }

    if id.len() > MAX_MODEL_ID_LENGTH {
        return Err(ModelValidationError::IdTooLong {
            length: id.len(),
            max: MAX_MODEL_ID_LENGTH,
        });
    }

    if !MODEL_ID_PATTERN.is_match(id) {
        return Err(ModelValidationError::InvalidIdFormat { id: id.to_string() });
    }

    Ok(())
}

/// Validate temperature value
pub fn validate_temperature(temp: f32) -> Result<(), ModelValidationError> {
    const MIN: f32 = 0.0;
    const MAX: f32 = 2.0;

    if !(MIN..=MAX).contains(&temp) {
        return Err(ModelValidationError::InvalidTemperature {
            value: temp,
            min: MIN,
            max: MAX,
        });
    }

    Ok(())
}

/// Validate top_p value
pub fn validate_top_p(top_p: f32) -> Result<(), ModelValidationError> {
    const MIN: f32 = 0.0;
    const MAX: f32 = 1.0;

    if !(MIN..=MAX).contains(&top_p) {
        return Err(ModelValidationError::InvalidTopP {
            value: top_p,
            min: MIN,
            max: MAX,
        });
    }

    Ok(())
}

/// Validate presence penalty value
pub fn validate_presence_penalty(penalty: f32) -> Result<(), ModelValidationError> {
    const MIN: f32 = -2.0;
    const MAX: f32 = 2.0;

    if !(MIN..=MAX).contains(&penalty) {
        return Err(ModelValidationError::InvalidPresencePenalty {
            value: penalty,
            min: MIN,
            max: MAX,
        });
    }

    Ok(())
}

/// Validate frequency penalty value
pub fn validate_frequency_penalty(penalty: f32) -> Result<(), ModelValidationError> {
    const MIN: f32 = -2.0;
    const MAX: f32 = 2.0;

    if !(MIN..=MAX).contains(&penalty) {
        return Err(ModelValidationError::InvalidFrequencyPenalty {
            value: penalty,
            min: MIN,
            max: MAX,
        });
    }

    Ok(())
}

/// Validate max_tokens value
pub fn validate_max_tokens(max_tokens: u32) -> Result<(), ModelValidationError> {
    if max_tokens == 0 {
        return Err(ModelValidationError::InvalidMaxTokens);
    }

    Ok(())
}

use super::ModelConfig;

/// Validate a complete ModelConfig
pub fn validate_model_config(config: &ModelConfig) -> Result<(), ModelValidationError> {
    if let Some(temp) = config.temperature {
        validate_temperature(temp)?;
    }

    if let Some(top_p) = config.top_p {
        validate_top_p(top_p)?;
    }

    if let Some(penalty) = config.presence_penalty {
        validate_presence_penalty(penalty)?;
    }

    if let Some(penalty) = config.frequency_penalty {
        validate_frequency_penalty(penalty)?;
    }

    if let Some(max_tokens) = config.max_tokens {
        validate_max_tokens(max_tokens)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_model_ids() {
        assert!(validate_model_id("a").is_ok());
        assert!(validate_model_id("my-model").is_ok());
        assert!(validate_model_id("my-model-123").is_ok());
        assert!(validate_model_id("GPT4-Production").is_ok());
        assert!(validate_model_id("claude-3-5-sonnet").is_ok());
        assert!(validate_model_id("a1").is_ok());
        assert!(validate_model_id("1a").is_ok());
    }

    #[test]
    fn test_invalid_model_ids() {
        // Empty
        assert!(matches!(
            validate_model_id(""),
            Err(ModelValidationError::EmptyId)
        ));

        // Too long
        let long_id = "a".repeat(51);
        assert!(matches!(
            validate_model_id(&long_id),
            Err(ModelValidationError::IdTooLong { .. })
        ));

        // Invalid characters
        assert!(matches!(
            validate_model_id("my_model"),
            Err(ModelValidationError::InvalidIdFormat { .. })
        ));
        assert!(matches!(
            validate_model_id("my model"),
            Err(ModelValidationError::InvalidIdFormat { .. })
        ));
        assert!(matches!(
            validate_model_id("my.model"),
            Err(ModelValidationError::InvalidIdFormat { .. })
        ));

        // Starts or ends with hyphen
        assert!(matches!(
            validate_model_id("-model"),
            Err(ModelValidationError::InvalidIdFormat { .. })
        ));
        assert!(matches!(
            validate_model_id("model-"),
            Err(ModelValidationError::InvalidIdFormat { .. })
        ));
    }

    #[test]
    fn test_max_length_model_id() {
        let max_id = "a".repeat(50);
        assert!(validate_model_id(&max_id).is_ok());
    }

    #[test]
    fn test_temperature_validation() {
        assert!(validate_temperature(0.0).is_ok());
        assert!(validate_temperature(1.0).is_ok());
        assert!(validate_temperature(2.0).is_ok());
        assert!(validate_temperature(0.7).is_ok());

        assert!(validate_temperature(-0.1).is_err());
        assert!(validate_temperature(2.1).is_err());
    }

    #[test]
    fn test_top_p_validation() {
        assert!(validate_top_p(0.0).is_ok());
        assert!(validate_top_p(0.5).is_ok());
        assert!(validate_top_p(1.0).is_ok());

        assert!(validate_top_p(-0.1).is_err());
        assert!(validate_top_p(1.1).is_err());
    }

    #[test]
    fn test_penalty_validation() {
        assert!(validate_presence_penalty(-2.0).is_ok());
        assert!(validate_presence_penalty(0.0).is_ok());
        assert!(validate_presence_penalty(2.0).is_ok());

        assert!(validate_presence_penalty(-2.1).is_err());
        assert!(validate_presence_penalty(2.1).is_err());

        assert!(validate_frequency_penalty(-2.0).is_ok());
        assert!(validate_frequency_penalty(2.0).is_ok());
        assert!(validate_frequency_penalty(-2.1).is_err());
    }

    #[test]
    fn test_max_tokens_validation() {
        assert!(validate_max_tokens(1).is_ok());
        assert!(validate_max_tokens(4096).is_ok());

        assert!(validate_max_tokens(0).is_err());
    }

    #[test]
    fn test_model_config_validation() {
        let valid_config = ModelConfig::new()
            .with_temperature(0.7)
            .with_max_tokens(4096)
            .with_top_p(0.9);
        assert!(validate_model_config(&valid_config).is_ok());

        let invalid_config = ModelConfig::new().with_temperature(3.0);
        assert!(validate_model_config(&invalid_config).is_err());
    }
}
