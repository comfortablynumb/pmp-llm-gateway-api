//! Document scoring trait and types

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

use super::config::RelevanceClassification;
use crate::domain::knowledge_base::SearchResult;
use crate::domain::DomainError;

/// A document with its relevance score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredDocument {
    /// Original search result
    pub document: SearchResult,
    /// Relevance score (0.0 - 1.0)
    pub relevance_score: f32,
    /// Classification based on score
    pub classification: RelevanceClassification,
    /// Explanation for the score (from LLM-based scoring)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl ScoredDocument {
    /// Create a new scored document
    pub fn new(
        document: SearchResult,
        relevance_score: f32,
        classification: RelevanceClassification,
    ) -> Self {
        Self {
            document,
            relevance_score,
            classification,
            reason: None,
        }
    }

    /// Add a reason for the score
    pub fn with_reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }

    /// Check if this document is relevant
    pub fn is_relevant(&self) -> bool {
        self.classification.is_relevant()
    }

    /// Check if this document is ambiguous
    pub fn is_ambiguous(&self) -> bool {
        self.classification.is_ambiguous()
    }

    /// Check if this document is irrelevant
    pub fn is_irrelevant(&self) -> bool {
        self.classification.is_irrelevant()
    }
}

/// Input for document scoring
#[derive(Debug, Clone)]
pub struct ScoringInput {
    /// The user's query
    pub query: String,
    /// Documents to score
    pub documents: Vec<SearchResult>,
}

impl ScoringInput {
    /// Create a new scoring input
    pub fn new(query: impl Into<String>, documents: Vec<SearchResult>) -> Self {
        Self {
            query: query.into(),
            documents,
        }
    }
}

/// Trait for scoring document relevance
#[async_trait]
pub trait DocumentScorer: Send + Sync + Debug {
    /// Score a single document's relevance to a query
    async fn score_document(
        &self,
        query: &str,
        document: &SearchResult,
    ) -> Result<ScoredDocument, DomainError>;

    /// Score multiple documents
    /// Default implementation scores documents sequentially
    async fn score_documents(
        &self,
        input: ScoringInput,
    ) -> Result<Vec<ScoredDocument>, DomainError> {
        let mut results = Vec::with_capacity(input.documents.len());

        for document in &input.documents {
            let scored = self.score_document(&input.query, document).await?;
            results.push(scored);
        }

        Ok(results)
    }

    /// Get the scorer name
    fn scorer_name(&self) -> &'static str;
}

#[cfg(test)]
pub mod mock {
    use super::*;

    /// Mock document scorer for testing
    #[derive(Debug)]
    pub struct MockDocumentScorer {
        fixed_score: Option<f32>,
        scores: std::collections::HashMap<String, f32>,
        error: Option<String>,
    }

    impl MockDocumentScorer {
        /// Create a new mock scorer
        pub fn new() -> Self {
            Self {
                fixed_score: None,
                scores: std::collections::HashMap::new(),
                error: None,
            }
        }

        /// Set a fixed score for all documents
        pub fn with_fixed_score(mut self, score: f32) -> Self {
            self.fixed_score = Some(score);
            self
        }

        /// Set a specific score for a document ID
        pub fn with_score_for(mut self, doc_id: impl Into<String>, score: f32) -> Self {
            self.scores.insert(doc_id.into(), score);
            self
        }

        /// Set an error to return
        pub fn with_error(mut self, error: impl Into<String>) -> Self {
            self.error = Some(error.into());
            self
        }

        fn classify(&self, score: f32) -> RelevanceClassification {
            if score >= 0.8 {
                RelevanceClassification::Correct
            } else if score >= 0.5 {
                RelevanceClassification::Ambiguous
            } else {
                RelevanceClassification::Incorrect
            }
        }
    }

    impl Default for MockDocumentScorer {
        fn default() -> Self {
            Self::new()
        }
    }

    #[async_trait]
    impl DocumentScorer for MockDocumentScorer {
        async fn score_document(
            &self,
            _query: &str,
            document: &SearchResult,
        ) -> Result<ScoredDocument, DomainError> {
            if let Some(ref error) = self.error {
                return Err(DomainError::provider("mock_scorer", error));
            }

            let score = self
                .scores
                .get(&document.id)
                .copied()
                .or(self.fixed_score)
                .unwrap_or(document.score);

            let classification = self.classify(score);

            Ok(ScoredDocument::new(document.clone(), score, classification)
                .with_reason("Mock scorer"))
        }

        fn scorer_name(&self) -> &'static str {
            "mock"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::mock::MockDocumentScorer;

    fn create_test_document(id: &str, score: f32) -> SearchResult {
        SearchResult::new(id, format!("Content for {}", id), score)
    }

    #[test]
    fn test_scored_document_creation() {
        let doc = create_test_document("doc-1", 0.9);
        let scored = ScoredDocument::new(doc, 0.85, RelevanceClassification::Correct);

        assert_eq!(scored.relevance_score, 0.85);
        assert!(scored.is_relevant());
        assert!(!scored.is_ambiguous());
        assert!(!scored.is_irrelevant());
    }

    #[test]
    fn test_scored_document_with_reason() {
        let doc = create_test_document("doc-1", 0.9);
        let scored = ScoredDocument::new(doc, 0.85, RelevanceClassification::Correct)
            .with_reason("Highly relevant to the query");

        assert_eq!(scored.reason, Some("Highly relevant to the query".to_string()));
    }

    #[test]
    fn test_scoring_input() {
        let docs = vec![
            create_test_document("doc-1", 0.9),
            create_test_document("doc-2", 0.7),
        ];
        let input = ScoringInput::new("test query", docs);

        assert_eq!(input.query, "test query");
        assert_eq!(input.documents.len(), 2);
    }

    #[tokio::test]
    async fn test_mock_scorer_fixed_score() {
        let scorer = MockDocumentScorer::new().with_fixed_score(0.9);
        let doc = create_test_document("doc-1", 0.5);

        let result = scorer.score_document("query", &doc).await.unwrap();

        assert_eq!(result.relevance_score, 0.9);
        assert_eq!(result.classification, RelevanceClassification::Correct);
    }

    #[tokio::test]
    async fn test_mock_scorer_specific_scores() {
        let scorer = MockDocumentScorer::new()
            .with_score_for("doc-1", 0.95)
            .with_score_for("doc-2", 0.3);

        let doc1 = create_test_document("doc-1", 0.5);
        let doc2 = create_test_document("doc-2", 0.5);

        let result1 = scorer.score_document("query", &doc1).await.unwrap();
        let result2 = scorer.score_document("query", &doc2).await.unwrap();

        assert_eq!(result1.relevance_score, 0.95);
        assert_eq!(result1.classification, RelevanceClassification::Correct);

        assert_eq!(result2.relevance_score, 0.3);
        assert_eq!(result2.classification, RelevanceClassification::Incorrect);
    }

    #[tokio::test]
    async fn test_mock_scorer_uses_document_score_as_fallback() {
        let scorer = MockDocumentScorer::new();
        let doc = create_test_document("doc-1", 0.75);

        let result = scorer.score_document("query", &doc).await.unwrap();

        assert_eq!(result.relevance_score, 0.75);
        assert_eq!(result.classification, RelevanceClassification::Ambiguous);
    }

    #[tokio::test]
    async fn test_mock_scorer_error() {
        let scorer = MockDocumentScorer::new().with_error("Scoring failed");
        let doc = create_test_document("doc-1", 0.9);

        let result = scorer.score_document("query", &doc).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_score_multiple_documents() {
        let scorer = MockDocumentScorer::new()
            .with_score_for("doc-1", 0.9)
            .with_score_for("doc-2", 0.6)
            .with_score_for("doc-3", 0.3);

        let docs = vec![
            create_test_document("doc-1", 0.5),
            create_test_document("doc-2", 0.5),
            create_test_document("doc-3", 0.5),
        ];
        let input = ScoringInput::new("query", docs);

        let results = scorer.score_documents(input).await.unwrap();

        assert_eq!(results.len(), 3);
        assert!(results[0].is_relevant());
        assert!(results[1].is_ambiguous());
        assert!(results[2].is_irrelevant());
    }
}
