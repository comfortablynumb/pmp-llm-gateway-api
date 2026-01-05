//! Knowledge base validation utilities

use std::fmt;

use once_cell::sync::Lazy;
use regex::Regex;

/// Maximum length for knowledge base IDs
pub const MAX_KB_ID_LENGTH: usize = 50;

/// Regex pattern for valid knowledge base IDs (alphanumeric + hyphens)
static KB_ID_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^[a-zA-Z0-9][a-zA-Z0-9-]*[a-zA-Z0-9]$|^[a-zA-Z0-9]$").unwrap());

/// Knowledge base validation errors
#[derive(Debug, Clone, PartialEq)]
pub enum KnowledgeBaseValidationError {
    /// ID is empty
    EmptyId,
    /// ID exceeds maximum length
    IdTooLong { length: usize, max: usize },
    /// ID contains invalid characters
    InvalidIdFormat { id: String },
    /// Invalid embedding dimensions
    InvalidDimensions { value: u32, min: u32, max: u32 },
    /// Invalid top_k value
    InvalidTopK { value: u32, min: u32, max: u32 },
    /// Invalid similarity threshold
    InvalidSimilarityThreshold { value: f32 },
}

impl fmt::Display for KnowledgeBaseValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyId => write!(f, "Knowledge base ID cannot be empty"),
            Self::IdTooLong { length, max } => {
                write!(
                    f,
                    "Knowledge base ID too long: {} characters (max {})",
                    length, max
                )
            }
            Self::InvalidIdFormat { id } => {
                write!(
                    f,
                    "Invalid knowledge base ID format '{}': must be alphanumeric with hyphens",
                    id
                )
            }
            Self::InvalidDimensions { value, min, max } => {
                write!(
                    f,
                    "Invalid embedding dimensions {}: must be between {} and {}",
                    value, min, max
                )
            }
            Self::InvalidTopK { value, min, max } => {
                write!(
                    f,
                    "Invalid top_k {}: must be between {} and {}",
                    value, min, max
                )
            }
            Self::InvalidSimilarityThreshold { value } => {
                write!(
                    f,
                    "Invalid similarity threshold {}: must be between 0.0 and 1.0",
                    value
                )
            }
        }
    }
}

impl std::error::Error for KnowledgeBaseValidationError {}

/// Validate a knowledge base ID
pub fn validate_knowledge_base_id(id: &str) -> Result<(), KnowledgeBaseValidationError> {
    if id.is_empty() {
        return Err(KnowledgeBaseValidationError::EmptyId);
    }

    if id.len() > MAX_KB_ID_LENGTH {
        return Err(KnowledgeBaseValidationError::IdTooLong {
            length: id.len(),
            max: MAX_KB_ID_LENGTH,
        });
    }

    if !KB_ID_PATTERN.is_match(id) {
        return Err(KnowledgeBaseValidationError::InvalidIdFormat { id: id.to_string() });
    }

    Ok(())
}

/// Validate embedding dimensions
pub fn validate_dimensions(dims: u32) -> Result<(), KnowledgeBaseValidationError> {
    const MIN: u32 = 1;
    const MAX: u32 = 8192; // Most models have <= 4096 dims

    if !(MIN..=MAX).contains(&dims) {
        return Err(KnowledgeBaseValidationError::InvalidDimensions {
            value: dims,
            min: MIN,
            max: MAX,
        });
    }

    Ok(())
}

/// Validate top_k value
pub fn validate_top_k(top_k: u32) -> Result<(), KnowledgeBaseValidationError> {
    const MIN: u32 = 1;
    const MAX: u32 = 1000;

    if !(MIN..=MAX).contains(&top_k) {
        return Err(KnowledgeBaseValidationError::InvalidTopK {
            value: top_k,
            min: MIN,
            max: MAX,
        });
    }

    Ok(())
}

/// Validate similarity threshold
pub fn validate_similarity_threshold(threshold: f32) -> Result<(), KnowledgeBaseValidationError> {
    if !(0.0..=1.0).contains(&threshold) {
        return Err(KnowledgeBaseValidationError::InvalidSimilarityThreshold {
            value: threshold,
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_kb_ids() {
        assert!(validate_knowledge_base_id("a").is_ok());
        assert!(validate_knowledge_base_id("my-kb").is_ok());
        assert!(validate_knowledge_base_id("product-docs-v1").is_ok());
        assert!(validate_knowledge_base_id("KB123").is_ok());
    }

    #[test]
    fn test_invalid_kb_ids() {
        assert!(matches!(
            validate_knowledge_base_id(""),
            Err(KnowledgeBaseValidationError::EmptyId)
        ));

        let long_id = "a".repeat(51);
        assert!(matches!(
            validate_knowledge_base_id(&long_id),
            Err(KnowledgeBaseValidationError::IdTooLong { .. })
        ));

        assert!(matches!(
            validate_knowledge_base_id("my_kb"),
            Err(KnowledgeBaseValidationError::InvalidIdFormat { .. })
        ));

        assert!(matches!(
            validate_knowledge_base_id("-kb"),
            Err(KnowledgeBaseValidationError::InvalidIdFormat { .. })
        ));
    }

    #[test]
    fn test_dimensions_validation() {
        assert!(validate_dimensions(1).is_ok());
        assert!(validate_dimensions(1536).is_ok());
        assert!(validate_dimensions(8192).is_ok());

        assert!(validate_dimensions(0).is_err());
        assert!(validate_dimensions(10000).is_err());
    }

    #[test]
    fn test_top_k_validation() {
        assert!(validate_top_k(1).is_ok());
        assert!(validate_top_k(10).is_ok());
        assert!(validate_top_k(1000).is_ok());

        assert!(validate_top_k(0).is_err());
        assert!(validate_top_k(1001).is_err());
    }

    #[test]
    fn test_similarity_threshold_validation() {
        assert!(validate_similarity_threshold(0.0).is_ok());
        assert!(validate_similarity_threshold(0.5).is_ok());
        assert!(validate_similarity_threshold(1.0).is_ok());

        assert!(validate_similarity_threshold(-0.1).is_err());
        assert!(validate_similarity_threshold(1.1).is_err());
    }
}
