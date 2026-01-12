//! Experiment assignment types for routing requests to variants

use serde::{Deserialize, Serialize};

/// Configuration overrides that can be applied to a request
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConfigOverrides {
    /// Temperature override
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Max tokens override
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    /// Top-p override
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    /// Presence penalty override
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f32>,
    /// Frequency penalty override
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f32>,
}

impl ConfigOverrides {
    /// Create empty config overrides
    pub fn new() -> Self {
        Self::default()
    }

    /// Set temperature override
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Set max tokens override
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// Set top-p override
    pub fn with_top_p(mut self, top_p: f32) -> Self {
        self.top_p = Some(top_p);
        self
    }

    /// Set presence penalty override
    pub fn with_presence_penalty(mut self, presence_penalty: f32) -> Self {
        self.presence_penalty = Some(presence_penalty);
        self
    }

    /// Set frequency penalty override
    pub fn with_frequency_penalty(mut self, frequency_penalty: f32) -> Self {
        self.frequency_penalty = Some(frequency_penalty);
        self
    }

    /// Check if any overrides are set
    pub fn has_overrides(&self) -> bool {
        self.temperature.is_some()
            || self.max_tokens.is_some()
            || self.top_p.is_some()
            || self.presence_penalty.is_some()
            || self.frequency_penalty.is_some()
    }
}

/// Result of assigning a request to an experiment variant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssignmentResult {
    /// ID of the experiment
    pub experiment_id: String,
    /// ID of the assigned variant
    pub variant_id: String,
    /// ID of the model to use
    pub model_id: String,
    /// Optional configuration overrides
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_overrides: Option<ConfigOverrides>,
}

impl AssignmentResult {
    /// Create a new assignment result
    pub fn new(
        experiment_id: impl Into<String>,
        variant_id: impl Into<String>,
        model_id: impl Into<String>,
    ) -> Self {
        Self {
            experiment_id: experiment_id.into(),
            variant_id: variant_id.into(),
            model_id: model_id.into(),
            config_overrides: None,
        }
    }

    /// Set configuration overrides
    pub fn with_overrides(mut self, overrides: ConfigOverrides) -> Self {
        if overrides.has_overrides() {
            self.config_overrides = Some(overrides);
        }
        self
    }

    /// Check if this assignment has configuration overrides
    pub fn has_config_overrides(&self) -> bool {
        self.config_overrides
            .as_ref()
            .map(|o| o.has_overrides())
            .unwrap_or(false)
    }

    /// Get the temperature override if set
    pub fn temperature(&self) -> Option<f32> {
        self.config_overrides.as_ref().and_then(|o| o.temperature)
    }

    /// Get the max tokens override if set
    pub fn max_tokens(&self) -> Option<u32> {
        self.config_overrides.as_ref().and_then(|o| o.max_tokens)
    }

    /// Get the top-p override if set
    pub fn top_p(&self) -> Option<f32> {
        self.config_overrides.as_ref().and_then(|o| o.top_p)
    }

    /// Get the presence penalty override if set
    pub fn presence_penalty(&self) -> Option<f32> {
        self.config_overrides
            .as_ref()
            .and_then(|o| o.presence_penalty)
    }

    /// Get the frequency penalty override if set
    pub fn frequency_penalty(&self) -> Option<f32> {
        self.config_overrides
            .as_ref()
            .and_then(|o| o.frequency_penalty)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod config_overrides_tests {
        use super::*;

        #[test]
        fn test_empty_overrides() {
            let overrides = ConfigOverrides::new();
            assert!(!overrides.has_overrides());
        }

        #[test]
        fn test_with_temperature() {
            let overrides = ConfigOverrides::new().with_temperature(0.7);
            assert!(overrides.has_overrides());
            assert_eq!(overrides.temperature, Some(0.7));
        }

        #[test]
        fn test_builder_chain() {
            let overrides = ConfigOverrides::new()
                .with_temperature(0.7)
                .with_max_tokens(1000)
                .with_top_p(0.9);

            assert!(overrides.has_overrides());
            assert_eq!(overrides.temperature, Some(0.7));
            assert_eq!(overrides.max_tokens, Some(1000));
            assert_eq!(overrides.top_p, Some(0.9));
            assert_eq!(overrides.presence_penalty, None);
        }
    }

    mod assignment_result_tests {
        use super::*;

        #[test]
        fn test_new_assignment() {
            let result = AssignmentResult::new("exp-1", "control", "gpt-4");

            assert_eq!(result.experiment_id, "exp-1");
            assert_eq!(result.variant_id, "control");
            assert_eq!(result.model_id, "gpt-4");
            assert!(!result.has_config_overrides());
        }

        #[test]
        fn test_with_overrides() {
            let overrides = ConfigOverrides::new().with_temperature(0.5);
            let result =
                AssignmentResult::new("exp-1", "treatment", "gpt-4").with_overrides(overrides);

            assert!(result.has_config_overrides());
            assert_eq!(result.temperature(), Some(0.5));
        }

        #[test]
        fn test_empty_overrides_not_stored() {
            let overrides = ConfigOverrides::new();
            let result =
                AssignmentResult::new("exp-1", "control", "gpt-4").with_overrides(overrides);

            assert!(!result.has_config_overrides());
            assert!(result.config_overrides.is_none());
        }

        #[test]
        fn test_override_accessors() {
            let overrides = ConfigOverrides::new()
                .with_temperature(0.7)
                .with_max_tokens(500)
                .with_top_p(0.95)
                .with_presence_penalty(0.1)
                .with_frequency_penalty(0.2);

            let result =
                AssignmentResult::new("exp-1", "treatment", "gpt-4").with_overrides(overrides);

            assert_eq!(result.temperature(), Some(0.7));
            assert_eq!(result.max_tokens(), Some(500));
            assert_eq!(result.top_p(), Some(0.95));
            assert_eq!(result.presence_penalty(), Some(0.1));
            assert_eq!(result.frequency_penalty(), Some(0.2));
        }
    }
}
