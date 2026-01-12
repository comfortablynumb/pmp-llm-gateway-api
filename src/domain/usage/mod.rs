//! Usage tracking and cost management domain
//!
//! Provides entities and traits for tracking LLM usage, calculating costs,
//! and enforcing budgets.

mod budget;
mod pricing;
mod record;
mod repository;

pub use budget::{Budget, BudgetAlert, BudgetId, BudgetPeriod, BudgetScope, BudgetStatus};
pub use pricing::{default_model_pricing, ModelPricing, PricingTier};
pub use record::{DailyUsage, UsageAggregate, UsageRecord, UsageRecordId, UsageSummary, UsageType};
pub use repository::{BudgetRepository, UsageQuery, UsageRepository};

/// Validate a budget ID
pub fn validate_budget_id(id: &str) -> Result<(), BudgetValidationError> {
    if id.is_empty() {
        return Err(BudgetValidationError::EmptyId);
    }

    if id.len() > 50 {
        return Err(BudgetValidationError::IdTooLong(id.len()));
    }

    if !id
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        return Err(BudgetValidationError::InvalidIdFormat);
    }

    Ok(())
}

/// Budget validation errors
#[derive(Debug, Clone, PartialEq)]
pub enum BudgetValidationError {
    EmptyId,
    IdTooLong(usize),
    InvalidIdFormat,
    InvalidLimit,
    InvalidPeriod,
}

impl std::fmt::Display for BudgetValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyId => write!(f, "Budget ID cannot be empty"),
            Self::IdTooLong(len) => write!(f, "Budget ID too long: {} chars (max 50)", len),
            Self::InvalidIdFormat => {
                write!(f, "Budget ID must be alphanumeric with hyphens/underscores")
            }
            Self::InvalidLimit => write!(f, "Budget limit must be positive"),
            Self::InvalidPeriod => write!(f, "Invalid budget period"),
        }
    }
}

impl std::error::Error for BudgetValidationError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_budget_id_valid() {
        assert!(validate_budget_id("my-budget").is_ok());
        assert!(validate_budget_id("budget_123").is_ok());
        assert!(validate_budget_id("Budget-2024").is_ok());
    }

    #[test]
    fn test_validate_budget_id_empty() {
        assert!(matches!(
            validate_budget_id(""),
            Err(BudgetValidationError::EmptyId)
        ));
    }

    #[test]
    fn test_validate_budget_id_too_long() {
        let long_id = "a".repeat(51);
        assert!(matches!(
            validate_budget_id(&long_id),
            Err(BudgetValidationError::IdTooLong(51))
        ));
    }

    #[test]
    fn test_validate_budget_id_invalid_chars() {
        assert!(matches!(
            validate_budget_id("budget/test"),
            Err(BudgetValidationError::InvalidIdFormat)
        ));
    }
}
