//! Experiment domain module for A/B testing
//!
//! This module provides types and traits for managing A/B testing experiments
//! that compare different LLM models and configurations.

mod assignment;
mod entity;
mod record;
mod repository;
mod result;
mod validation;

// Re-export all public types
pub use assignment::{AssignmentResult, ConfigOverrides};
pub use entity::{
    Experiment, ExperimentId, ExperimentStatus, TrafficAllocation, Variant, VariantConfig,
    VariantId,
};
pub use record::{ExperimentRecord, ExperimentRecordId};
pub use repository::{
    ExperimentQuery, ExperimentRecordQuery, ExperimentRecordRepository, ExperimentRepository,
};
pub use result::{ExperimentResult, LatencyStats, StatisticalSignificance, VariantMetrics};
pub use validation::ExperimentValidationError;

#[cfg(test)]
pub use repository::mock::{MockExperimentRecordRepository, MockExperimentRepository};
