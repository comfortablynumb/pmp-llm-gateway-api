//! Experiment domain entities

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

use super::validation::{
    validate_experiment_id, validate_variant_id, ExperimentValidationError,
};
use crate::domain::storage::{StorageEntity, StorageKey};

// ============================================================================
// ExperimentId
// ============================================================================

/// Unique identifier for an experiment
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ExperimentId(String);

impl ExperimentId {
    /// Create a new experiment ID with validation
    pub fn new(id: impl Into<String>) -> Result<Self, ExperimentValidationError> {
        let id = id.into();
        validate_experiment_id(&id)?;
        Ok(Self(id))
    }

    /// Get the ID as a string slice
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for ExperimentId {
    type Error = ExperimentValidationError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<ExperimentId> for String {
    fn from(id: ExperimentId) -> Self {
        id.0
    }
}

impl fmt::Display for ExperimentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for ExperimentId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl StorageKey for ExperimentId {
    fn as_str(&self) -> &str {
        &self.0
    }
}

// ============================================================================
// VariantId
// ============================================================================

/// Unique identifier for a variant within an experiment
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct VariantId(String);

impl VariantId {
    /// Create a new variant ID with validation
    pub fn new(id: impl Into<String>) -> Result<Self, ExperimentValidationError> {
        let id = id.into();
        validate_variant_id(&id)?;
        Ok(Self(id))
    }

    /// Get the ID as a string slice
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for VariantId {
    type Error = ExperimentValidationError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<VariantId> for String {
    fn from(id: VariantId) -> Self {
        id.0
    }
}

impl fmt::Display for VariantId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for VariantId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

// ============================================================================
// ExperimentStatus
// ============================================================================

/// Status of an experiment
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ExperimentStatus {
    /// Experiment is being configured, not yet running
    #[default]
    Draft,
    /// Experiment is actively running and routing traffic
    Active,
    /// Experiment is temporarily paused
    Paused,
    /// Experiment has finished
    Completed,
}

impl ExperimentStatus {
    /// Check if the experiment is currently running
    pub fn is_running(&self) -> bool {
        matches!(self, Self::Active)
    }

    /// Check if the experiment can accept new configuration changes
    pub fn is_editable(&self) -> bool {
        matches!(self, Self::Draft)
    }

    /// Check if a transition to the target status is valid
    pub fn can_transition_to(&self, target: ExperimentStatus) -> bool {
        match (self, target) {
            // Draft -> Active (start)
            (Self::Draft, Self::Active) => true,
            // Active -> Paused (pause)
            (Self::Active, Self::Paused) => true,
            // Active -> Completed (complete)
            (Self::Active, Self::Completed) => true,
            // Paused -> Active (resume)
            (Self::Paused, Self::Active) => true,
            // Paused -> Completed (complete)
            (Self::Paused, Self::Completed) => true,
            // All other transitions are invalid
            _ => false,
        }
    }
}

impl fmt::Display for ExperimentStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Draft => write!(f, "draft"),
            Self::Active => write!(f, "active"),
            Self::Paused => write!(f, "paused"),
            Self::Completed => write!(f, "completed"),
        }
    }
}

// ============================================================================
// VariantConfig
// ============================================================================

/// Configuration for a variant - either a model reference or config override
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum VariantConfig {
    /// Use a different model entirely
    ModelReference { model_id: String },
    /// Use the same model with different configuration parameters
    ConfigOverride {
        model_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        temperature: Option<f32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        max_tokens: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        top_p: Option<f32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        presence_penalty: Option<f32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        frequency_penalty: Option<f32>,
    },
}

impl VariantConfig {
    /// Create a model reference variant
    pub fn model_reference(model_id: impl Into<String>) -> Self {
        Self::ModelReference {
            model_id: model_id.into(),
        }
    }

    /// Create a config override variant with no overrides
    pub fn config_override(model_id: impl Into<String>) -> Self {
        Self::ConfigOverride {
            model_id: model_id.into(),
            temperature: None,
            max_tokens: None,
            top_p: None,
            presence_penalty: None,
            frequency_penalty: None,
        }
    }

    /// Get the model ID for this variant
    pub fn model_id(&self) -> &str {
        match self {
            Self::ModelReference { model_id } => model_id,
            Self::ConfigOverride { model_id, .. } => model_id,
        }
    }

    /// Check if this is a config override
    pub fn is_config_override(&self) -> bool {
        matches!(self, Self::ConfigOverride { .. })
    }
}

// ============================================================================
// TrafficAllocation
// ============================================================================

/// Traffic allocation for a variant
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TrafficAllocation {
    variant_id: VariantId,
    percentage: u8,
}

impl TrafficAllocation {
    /// Create a new traffic allocation
    pub fn new(variant_id: VariantId, percentage: u8) -> Self {
        Self {
            variant_id,
            percentage: percentage.min(100),
        }
    }

    /// Get the variant ID
    pub fn variant_id(&self) -> &VariantId {
        &self.variant_id
    }

    /// Get the traffic percentage
    pub fn percentage(&self) -> u8 {
        self.percentage
    }
}

// ============================================================================
// Variant
// ============================================================================

/// A variant in an A/B test experiment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Variant {
    id: VariantId,
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    config: VariantConfig,
    control: bool,
}

impl Variant {
    /// Create a new variant
    pub fn new(id: VariantId, name: impl Into<String>, config: VariantConfig) -> Self {
        Self {
            id,
            name: name.into(),
            description: None,
            config,
            control: false,
        }
    }

    /// Set the description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set whether this is the control variant
    pub fn with_control(mut self, control: bool) -> Self {
        self.control = control;
        self
    }

    /// Get the variant ID
    pub fn id(&self) -> &VariantId {
        &self.id
    }

    /// Get the variant name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the description
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// Get the variant configuration
    pub fn config(&self) -> &VariantConfig {
        &self.config
    }

    /// Check if this is the control variant
    pub fn is_control(&self) -> bool {
        self.control
    }

    /// Get the model ID from the config
    pub fn model_id(&self) -> &str {
        self.config.model_id()
    }
}

// ============================================================================
// Experiment
// ============================================================================

/// An A/B test experiment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Experiment {
    id: ExperimentId,
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    status: ExperimentStatus,
    variants: Vec<Variant>,
    traffic_allocation: Vec<TrafficAllocation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    started_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    completed_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    enabled: bool,
}

impl Experiment {
    /// Create a new experiment in Draft status
    pub fn new(id: ExperimentId, name: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id,
            name: name.into(),
            description: None,
            status: ExperimentStatus::Draft,
            variants: Vec::new(),
            traffic_allocation: Vec::new(),
            started_at: None,
            completed_at: None,
            created_at: now,
            updated_at: now,
            enabled: true,
        }
    }

    // Builder methods

    /// Set the description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Add a variant
    pub fn with_variant(mut self, variant: Variant) -> Self {
        self.variants.push(variant);
        self
    }

    /// Add a traffic allocation
    pub fn with_traffic_allocation(mut self, allocation: TrafficAllocation) -> Self {
        self.traffic_allocation.push(allocation);
        self
    }

    /// Set enabled status
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    // Getters

    /// Get the experiment ID
    pub fn id(&self) -> &ExperimentId {
        &self.id
    }

    /// Get the experiment name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the description
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// Get the current status
    pub fn status(&self) -> ExperimentStatus {
        self.status
    }

    /// Get all variants
    pub fn variants(&self) -> &[Variant] {
        &self.variants
    }

    /// Get all traffic allocations
    pub fn traffic_allocation(&self) -> &[TrafficAllocation] {
        &self.traffic_allocation
    }

    /// Get when the experiment was started
    pub fn started_at(&self) -> Option<DateTime<Utc>> {
        self.started_at
    }

    /// Get when the experiment was completed
    pub fn completed_at(&self) -> Option<DateTime<Utc>> {
        self.completed_at
    }

    /// Get when the experiment was created
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    /// Get when the experiment was last updated
    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    /// Check if the experiment is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    // Mutators

    /// Set the experiment name
    pub fn set_name(&mut self, name: impl Into<String>) {
        self.name = name.into();
        self.touch();
    }

    /// Set the description
    pub fn set_description(&mut self, description: Option<String>) {
        self.description = description;
        self.touch();
    }

    /// Set the enabled status
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        self.touch();
    }

    /// Set all variants
    pub fn set_variants(&mut self, variants: Vec<Variant>) {
        self.variants = variants;
        self.touch();
    }

    /// Set all traffic allocations
    pub fn set_traffic_allocation(&mut self, allocation: Vec<TrafficAllocation>) {
        self.traffic_allocation = allocation;
        self.touch();
    }

    // Status transitions

    /// Start the experiment
    pub fn start(&mut self) -> Result<(), ExperimentValidationError> {
        if !self.status.can_transition_to(ExperimentStatus::Active) {
            return Err(ExperimentValidationError::InvalidStatusTransition(
                self.status.to_string(),
                "active".to_string(),
            ));
        }
        self.status = ExperimentStatus::Active;
        self.started_at = Some(Utc::now());
        self.touch();
        Ok(())
    }

    /// Pause the experiment
    pub fn pause(&mut self) -> Result<(), ExperimentValidationError> {
        if !self.status.can_transition_to(ExperimentStatus::Paused) {
            return Err(ExperimentValidationError::InvalidStatusTransition(
                self.status.to_string(),
                "paused".to_string(),
            ));
        }
        self.status = ExperimentStatus::Paused;
        self.touch();
        Ok(())
    }

    /// Resume the experiment
    pub fn resume(&mut self) -> Result<(), ExperimentValidationError> {
        if !self.status.can_transition_to(ExperimentStatus::Active) {
            return Err(ExperimentValidationError::InvalidStatusTransition(
                self.status.to_string(),
                "active".to_string(),
            ));
        }
        self.status = ExperimentStatus::Active;
        self.touch();
        Ok(())
    }

    /// Complete the experiment
    pub fn complete(&mut self) -> Result<(), ExperimentValidationError> {
        if !self.status.can_transition_to(ExperimentStatus::Completed) {
            return Err(ExperimentValidationError::InvalidStatusTransition(
                self.status.to_string(),
                "completed".to_string(),
            ));
        }
        self.status = ExperimentStatus::Completed;
        self.completed_at = Some(Utc::now());
        self.touch();
        Ok(())
    }

    // Assignment

    /// Get a variant based on a hash value (0-99)
    pub fn get_variant_for_hash(&self, hash_value: u8) -> Option<&Variant> {
        let mut cumulative: u8 = 0;

        for allocation in &self.traffic_allocation {
            cumulative = cumulative.saturating_add(allocation.percentage());

            if hash_value < cumulative {
                return self
                    .variants
                    .iter()
                    .find(|v| v.id() == allocation.variant_id());
            }
        }

        None
    }

    /// Get all model IDs referenced by variants
    pub fn referenced_model_ids(&self) -> Vec<&str> {
        self.variants.iter().map(|v| v.model_id()).collect()
    }

    /// Get the control variant if one exists
    pub fn control_variant(&self) -> Option<&Variant> {
        self.variants.iter().find(|v| v.is_control())
    }

    // Private helpers

    fn touch(&mut self) {
        self.updated_at = Utc::now();
    }
}

impl StorageEntity for Experiment {
    type Key = ExperimentId;

    fn key(&self) -> &Self::Key {
        &self.id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod experiment_id_tests {
        use super::*;

        #[test]
        fn test_valid_experiment_id() {
            let id = ExperimentId::new("my-experiment").unwrap();
            assert_eq!(id.as_str(), "my-experiment");
        }

        #[test]
        fn test_experiment_id_serialization() {
            let id = ExperimentId::new("test-exp").unwrap();
            let json = serde_json::to_string(&id).unwrap();
            assert_eq!(json, "\"test-exp\"");

            let parsed: ExperimentId = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, id);
        }

        #[test]
        fn test_invalid_experiment_id() {
            assert!(ExperimentId::new("").is_err());
            assert!(ExperimentId::new("-invalid").is_err());
            assert!(ExperimentId::new("invalid-").is_err());
        }
    }

    mod variant_id_tests {
        use super::*;

        #[test]
        fn test_valid_variant_id() {
            let id = VariantId::new("control").unwrap();
            assert_eq!(id.as_str(), "control");
        }

        #[test]
        fn test_variant_id_serialization() {
            let id = VariantId::new("variant-a").unwrap();
            let json = serde_json::to_string(&id).unwrap();
            assert_eq!(json, "\"variant-a\"");

            let parsed: VariantId = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, id);
        }
    }

    mod experiment_status_tests {
        use super::*;

        #[test]
        fn test_default_status() {
            let status = ExperimentStatus::default();
            assert_eq!(status, ExperimentStatus::Draft);
        }

        #[test]
        fn test_status_transitions() {
            // Valid transitions
            assert!(ExperimentStatus::Draft.can_transition_to(ExperimentStatus::Active));
            assert!(ExperimentStatus::Active.can_transition_to(ExperimentStatus::Paused));
            assert!(ExperimentStatus::Active.can_transition_to(ExperimentStatus::Completed));
            assert!(ExperimentStatus::Paused.can_transition_to(ExperimentStatus::Active));
            assert!(ExperimentStatus::Paused.can_transition_to(ExperimentStatus::Completed));

            // Invalid transitions
            assert!(!ExperimentStatus::Draft.can_transition_to(ExperimentStatus::Paused));
            assert!(!ExperimentStatus::Draft.can_transition_to(ExperimentStatus::Completed));
            assert!(!ExperimentStatus::Completed.can_transition_to(ExperimentStatus::Active));
            assert!(!ExperimentStatus::Completed.can_transition_to(ExperimentStatus::Draft));
        }

        #[test]
        fn test_status_display() {
            assert_eq!(ExperimentStatus::Draft.to_string(), "draft");
            assert_eq!(ExperimentStatus::Active.to_string(), "active");
            assert_eq!(ExperimentStatus::Paused.to_string(), "paused");
            assert_eq!(ExperimentStatus::Completed.to_string(), "completed");
        }
    }

    mod variant_config_tests {
        use super::*;

        #[test]
        fn test_model_reference() {
            let config = VariantConfig::model_reference("gpt-4");
            assert_eq!(config.model_id(), "gpt-4");
            assert!(!config.is_config_override());
        }

        #[test]
        fn test_config_override() {
            let config = VariantConfig::ConfigOverride {
                model_id: "gpt-4".to_string(),
                temperature: Some(0.7),
                max_tokens: Some(1000),
                top_p: None,
                presence_penalty: None,
                frequency_penalty: None,
            };
            assert_eq!(config.model_id(), "gpt-4");
            assert!(config.is_config_override());
        }

        #[test]
        fn test_variant_config_serialization() {
            let config = VariantConfig::model_reference("gpt-4");
            let json = serde_json::to_string(&config).unwrap();
            assert!(json.contains("\"type\":\"model_reference\""));
            assert!(json.contains("\"model_id\":\"gpt-4\""));

            let parsed: VariantConfig = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, config);
        }
    }

    mod experiment_tests {
        use super::*;

        fn create_test_experiment() -> Experiment {
            let id = ExperimentId::new("test-exp").unwrap();
            let mut exp = Experiment::new(id, "Test Experiment");

            let control_id = VariantId::new("control").unwrap();
            let treatment_id = VariantId::new("treatment").unwrap();

            exp = exp
                .with_variant(
                    Variant::new(
                        control_id.clone(),
                        "Control",
                        VariantConfig::model_reference("gpt-4"),
                    )
                    .with_control(true),
                )
                .with_variant(Variant::new(
                    treatment_id.clone(),
                    "Treatment",
                    VariantConfig::model_reference("gpt-4-turbo"),
                ))
                .with_traffic_allocation(TrafficAllocation::new(control_id, 50))
                .with_traffic_allocation(TrafficAllocation::new(treatment_id, 50));

            exp
        }

        #[test]
        fn test_experiment_creation() {
            let exp = create_test_experiment();
            assert_eq!(exp.name(), "Test Experiment");
            assert_eq!(exp.status(), ExperimentStatus::Draft);
            assert_eq!(exp.variants().len(), 2);
            assert!(exp.is_enabled());
        }

        #[test]
        fn test_experiment_status_transitions() {
            let mut exp = create_test_experiment();

            // Start
            assert!(exp.start().is_ok());
            assert_eq!(exp.status(), ExperimentStatus::Active);
            assert!(exp.started_at().is_some());

            // Pause
            assert!(exp.pause().is_ok());
            assert_eq!(exp.status(), ExperimentStatus::Paused);

            // Resume
            assert!(exp.resume().is_ok());
            assert_eq!(exp.status(), ExperimentStatus::Active);

            // Complete
            assert!(exp.complete().is_ok());
            assert_eq!(exp.status(), ExperimentStatus::Completed);
            assert!(exp.completed_at().is_some());
        }

        #[test]
        fn test_invalid_status_transition() {
            let mut exp = create_test_experiment();

            // Can't pause from Draft
            assert!(exp.pause().is_err());

            // Can't complete from Draft
            assert!(exp.complete().is_err());
        }

        #[test]
        fn test_variant_assignment() {
            let exp = create_test_experiment();

            // Hash 0-49 should get control (first 50%)
            let variant_low = exp.get_variant_for_hash(25);
            assert!(variant_low.is_some());
            assert_eq!(variant_low.unwrap().name(), "Control");

            // Hash 50-99 should get treatment (last 50%)
            let variant_high = exp.get_variant_for_hash(75);
            assert!(variant_high.is_some());
            assert_eq!(variant_high.unwrap().name(), "Treatment");
        }

        #[test]
        fn test_control_variant() {
            let exp = create_test_experiment();
            let control = exp.control_variant();
            assert!(control.is_some());
            assert!(control.unwrap().is_control());
        }

        #[test]
        fn test_referenced_model_ids() {
            let exp = create_test_experiment();
            let model_ids = exp.referenced_model_ids();
            assert_eq!(model_ids.len(), 2);
            assert!(model_ids.contains(&"gpt-4"));
            assert!(model_ids.contains(&"gpt-4-turbo"));
        }
    }
}
