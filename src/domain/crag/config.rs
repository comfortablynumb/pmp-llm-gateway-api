//! CRAG configuration types

use serde::{Deserialize, Serialize};

/// Relevance classification for a document
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelevanceClassification {
    /// Document is highly relevant to the query
    Correct,
    /// Document relevance is uncertain
    Ambiguous,
    /// Document is not relevant to the query
    Incorrect,
}

impl RelevanceClassification {
    /// Check if this classification indicates relevance
    pub fn is_relevant(&self) -> bool {
        matches!(self, Self::Correct)
    }

    /// Check if this classification is ambiguous
    pub fn is_ambiguous(&self) -> bool {
        matches!(self, Self::Ambiguous)
    }

    /// Check if this classification indicates irrelevance
    pub fn is_irrelevant(&self) -> bool {
        matches!(self, Self::Incorrect)
    }
}

/// Scoring strategy for document evaluation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ScoringStrategy {
    /// Use LLM to evaluate document relevance with a prompt
    #[default]
    LlmBased,
    /// Use similarity score thresholds only (no LLM call)
    ThresholdBased,
    /// Combine similarity threshold with LLM evaluation
    Hybrid,
}

/// Configuration for CRAG evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CragConfig {
    /// Scoring strategy to use
    #[serde(default)]
    pub strategy: ScoringStrategy,
    /// Threshold for "correct" classification (0.0 - 1.0)
    #[serde(default = "default_correct_threshold")]
    pub correct_threshold: f32,
    /// Threshold for "ambiguous" classification (0.0 - 1.0)
    /// Documents with scores below this are "incorrect"
    #[serde(default = "default_ambiguous_threshold")]
    pub ambiguous_threshold: f32,
    /// Maximum number of documents to score (for LLM-based scoring)
    #[serde(default = "default_max_documents")]
    pub max_documents_to_score: usize,
    /// Whether to include ambiguous documents in results
    #[serde(default = "default_true")]
    pub include_ambiguous: bool,
    /// Custom evaluation prompt template
    /// Available variables: ${query}, ${document_content}, ${document_id}
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evaluation_prompt: Option<String>,
    /// Model to use for LLM-based scoring (optional, uses default if not set)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scoring_model: Option<String>,
    /// Temperature for LLM scoring (lower = more deterministic)
    #[serde(default = "default_temperature")]
    pub temperature: f32,
}

fn default_correct_threshold() -> f32 {
    0.8
}

fn default_ambiguous_threshold() -> f32 {
    0.5
}

fn default_max_documents() -> usize {
    10
}

fn default_true() -> bool {
    true
}

fn default_temperature() -> f32 {
    0.0
}

impl Default for CragConfig {
    fn default() -> Self {
        Self {
            strategy: ScoringStrategy::default(),
            correct_threshold: default_correct_threshold(),
            ambiguous_threshold: default_ambiguous_threshold(),
            max_documents_to_score: default_max_documents(),
            include_ambiguous: default_true(),
            evaluation_prompt: None,
            scoring_model: None,
            temperature: default_temperature(),
        }
    }
}

impl CragConfig {
    /// Create a new CRAG configuration with defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a threshold-based configuration
    pub fn threshold_based(correct: f32, ambiguous: f32) -> Self {
        Self {
            strategy: ScoringStrategy::ThresholdBased,
            correct_threshold: correct,
            ambiguous_threshold: ambiguous,
            ..Default::default()
        }
    }

    /// Create an LLM-based configuration
    pub fn llm_based() -> Self {
        Self {
            strategy: ScoringStrategy::LlmBased,
            ..Default::default()
        }
    }

    /// Set the scoring strategy
    pub fn with_strategy(mut self, strategy: ScoringStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    /// Set the correct threshold
    pub fn with_correct_threshold(mut self, threshold: f32) -> Self {
        self.correct_threshold = threshold.clamp(0.0, 1.0);
        self
    }

    /// Set the ambiguous threshold
    pub fn with_ambiguous_threshold(mut self, threshold: f32) -> Self {
        self.ambiguous_threshold = threshold.clamp(0.0, 1.0);
        self
    }

    /// Set the maximum documents to score
    pub fn with_max_documents(mut self, max: usize) -> Self {
        self.max_documents_to_score = max;
        self
    }

    /// Set whether to include ambiguous documents
    pub fn with_include_ambiguous(mut self, include: bool) -> Self {
        self.include_ambiguous = include;
        self
    }

    /// Set a custom evaluation prompt
    pub fn with_evaluation_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.evaluation_prompt = Some(prompt.into());
        self
    }

    /// Set the scoring model
    pub fn with_scoring_model(mut self, model: impl Into<String>) -> Self {
        self.scoring_model = Some(model.into());
        self
    }

    /// Set the temperature for LLM scoring
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = temperature.clamp(0.0, 2.0);
        self
    }

    /// Classify a score based on the configured thresholds
    pub fn classify(&self, score: f32) -> RelevanceClassification {
        if score >= self.correct_threshold {
            RelevanceClassification::Correct
        } else if score >= self.ambiguous_threshold {
            RelevanceClassification::Ambiguous
        } else {
            RelevanceClassification::Incorrect
        }
    }

    /// Get the default evaluation prompt
    pub fn default_evaluation_prompt() -> &'static str {
        r#"You are evaluating the relevance of a document to a user query.

Query: ${query}

Document:
${document_content}

Rate the relevance of this document to the query on a scale of 0 to 10, where:
- 0-3: Not relevant - the document does not help answer the query
- 4-6: Partially relevant - the document contains some related information
- 7-10: Highly relevant - the document directly addresses the query

Respond with ONLY a JSON object in this exact format:
{"score": <number>, "reason": "<brief explanation>"}"#
    }

    /// Get the evaluation prompt to use
    pub fn get_evaluation_prompt(&self) -> &str {
        match &self.evaluation_prompt {
            Some(prompt) => prompt.as_str(),
            None => Self::default_evaluation_prompt(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = CragConfig::default();

        assert_eq!(config.strategy, ScoringStrategy::LlmBased);
        assert_eq!(config.correct_threshold, 0.8);
        assert_eq!(config.ambiguous_threshold, 0.5);
        assert_eq!(config.max_documents_to_score, 10);
        assert!(config.include_ambiguous);
        assert!(config.evaluation_prompt.is_none());
    }

    #[test]
    fn test_threshold_based_config() {
        let config = CragConfig::threshold_based(0.9, 0.6);

        assert_eq!(config.strategy, ScoringStrategy::ThresholdBased);
        assert_eq!(config.correct_threshold, 0.9);
        assert_eq!(config.ambiguous_threshold, 0.6);
    }

    #[test]
    fn test_classification() {
        let config = CragConfig::default();

        assert_eq!(config.classify(0.9), RelevanceClassification::Correct);
        assert_eq!(config.classify(0.8), RelevanceClassification::Correct);
        assert_eq!(config.classify(0.7), RelevanceClassification::Ambiguous);
        assert_eq!(config.classify(0.5), RelevanceClassification::Ambiguous);
        assert_eq!(config.classify(0.4), RelevanceClassification::Incorrect);
        assert_eq!(config.classify(0.0), RelevanceClassification::Incorrect);
    }

    #[test]
    fn test_relevance_classification_methods() {
        assert!(RelevanceClassification::Correct.is_relevant());
        assert!(!RelevanceClassification::Correct.is_ambiguous());
        assert!(!RelevanceClassification::Correct.is_irrelevant());

        assert!(!RelevanceClassification::Ambiguous.is_relevant());
        assert!(RelevanceClassification::Ambiguous.is_ambiguous());
        assert!(!RelevanceClassification::Ambiguous.is_irrelevant());

        assert!(!RelevanceClassification::Incorrect.is_relevant());
        assert!(!RelevanceClassification::Incorrect.is_ambiguous());
        assert!(RelevanceClassification::Incorrect.is_irrelevant());
    }

    #[test]
    fn test_builder_pattern() {
        let config = CragConfig::new()
            .with_strategy(ScoringStrategy::Hybrid)
            .with_correct_threshold(0.85)
            .with_ambiguous_threshold(0.55)
            .with_max_documents(5)
            .with_include_ambiguous(false)
            .with_scoring_model("gpt-4")
            .with_temperature(0.1);

        assert_eq!(config.strategy, ScoringStrategy::Hybrid);
        assert_eq!(config.correct_threshold, 0.85);
        assert_eq!(config.ambiguous_threshold, 0.55);
        assert_eq!(config.max_documents_to_score, 5);
        assert!(!config.include_ambiguous);
        assert_eq!(config.scoring_model, Some("gpt-4".to_string()));
        assert_eq!(config.temperature, 0.1);
    }

    #[test]
    fn test_threshold_clamping() {
        let config = CragConfig::new()
            .with_correct_threshold(1.5)
            .with_ambiguous_threshold(-0.5);

        assert_eq!(config.correct_threshold, 1.0);
        assert_eq!(config.ambiguous_threshold, 0.0);
    }

    #[test]
    fn test_custom_evaluation_prompt() {
        let custom_prompt = "Custom prompt: ${query}";
        let config = CragConfig::new().with_evaluation_prompt(custom_prompt);

        assert_eq!(config.get_evaluation_prompt(), custom_prompt);
    }

    #[test]
    fn test_default_evaluation_prompt() {
        let config = CragConfig::default();

        assert!(config.get_evaluation_prompt().contains("${query}"));
        assert!(config.get_evaluation_prompt().contains("${document_content}"));
    }
}
