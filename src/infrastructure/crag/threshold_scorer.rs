//! Threshold-based document scorer
//!
//! Scores documents based on their similarity scores using configurable thresholds.

use async_trait::async_trait;

use crate::domain::crag::{CragConfig, DocumentScorer, ScoredDocument};
use crate::domain::knowledge_base::SearchResult;
use crate::domain::DomainError;

/// Document scorer that uses similarity thresholds only
#[derive(Debug, Clone)]
pub struct ThresholdDocumentScorer {
    config: CragConfig,
}

impl ThresholdDocumentScorer {
    /// Create a new threshold-based scorer
    pub fn new(config: CragConfig) -> Self {
        Self { config }
    }

    /// Create with default thresholds
    pub fn with_defaults() -> Self {
        Self::new(CragConfig::threshold_based(0.8, 0.5))
    }

    /// Create with custom thresholds
    pub fn with_thresholds(correct: f32, ambiguous: f32) -> Self {
        Self::new(CragConfig::threshold_based(correct, ambiguous))
    }
}

#[async_trait]
impl DocumentScorer for ThresholdDocumentScorer {
    async fn score_document(
        &self,
        _query: &str,
        document: &SearchResult,
    ) -> Result<ScoredDocument, DomainError> {
        let classification = self.config.classify(document.score);

        Ok(ScoredDocument::new(
            document.clone(),
            document.score,
            classification,
        ))
    }

    fn scorer_name(&self) -> &'static str {
        "threshold"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_document(id: &str, score: f32) -> SearchResult {
        SearchResult::new(id, format!("Content for {}", id), score)
    }

    #[tokio::test]
    async fn test_threshold_scorer_correct() {
        let scorer = ThresholdDocumentScorer::with_defaults();
        let doc = create_test_document("doc-1", 0.9);

        let result = scorer.score_document("query", &doc).await.unwrap();

        assert!(result.is_relevant());
        assert_eq!(result.relevance_score, 0.9);
    }

    #[tokio::test]
    async fn test_threshold_scorer_ambiguous() {
        let scorer = ThresholdDocumentScorer::with_defaults();
        let doc = create_test_document("doc-1", 0.6);

        let result = scorer.score_document("query", &doc).await.unwrap();

        assert!(result.is_ambiguous());
        assert_eq!(result.relevance_score, 0.6);
    }

    #[tokio::test]
    async fn test_threshold_scorer_incorrect() {
        let scorer = ThresholdDocumentScorer::with_defaults();
        let doc = create_test_document("doc-1", 0.3);

        let result = scorer.score_document("query", &doc).await.unwrap();

        assert!(result.is_irrelevant());
        assert_eq!(result.relevance_score, 0.3);
    }

    #[tokio::test]
    async fn test_threshold_scorer_custom_thresholds() {
        let scorer = ThresholdDocumentScorer::with_thresholds(0.9, 0.7);

        // 0.85 would be correct with defaults, but ambiguous with custom thresholds
        let doc = create_test_document("doc-1", 0.85);
        let result = scorer.score_document("query", &doc).await.unwrap();
        assert!(result.is_ambiguous());

        // 0.6 would be ambiguous with defaults, but incorrect with custom thresholds
        let doc = create_test_document("doc-2", 0.6);
        let result = scorer.score_document("query", &doc).await.unwrap();
        assert!(result.is_irrelevant());
    }

    #[test]
    fn test_scorer_name() {
        let scorer = ThresholdDocumentScorer::with_defaults();
        assert_eq!(scorer.scorer_name(), "threshold");
    }
}
