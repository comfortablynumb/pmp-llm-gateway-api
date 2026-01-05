//! CRAG pipeline types

use serde::{Deserialize, Serialize};

use super::config::{CragConfig, RelevanceClassification};
use super::scorer::ScoredDocument;

/// Result of CRAG filtering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CragResult {
    /// Documents classified as correct (relevant)
    pub correct: Vec<ScoredDocument>,
    /// Documents classified as ambiguous
    pub ambiguous: Vec<ScoredDocument>,
    /// Documents classified as incorrect (irrelevant)
    pub incorrect: Vec<ScoredDocument>,
    /// Total documents processed
    pub total_processed: usize,
}

impl CragResult {
    /// Create a new empty CRAG result
    pub fn new() -> Self {
        Self {
            correct: Vec::new(),
            ambiguous: Vec::new(),
            incorrect: Vec::new(),
            total_processed: 0,
        }
    }

    /// Create a result from scored documents
    pub fn from_scored_documents(documents: Vec<ScoredDocument>) -> Self {
        let total = documents.len();
        let mut result = Self::new();
        result.total_processed = total;

        for doc in documents {
            match doc.classification {
                RelevanceClassification::Correct => result.correct.push(doc),
                RelevanceClassification::Ambiguous => result.ambiguous.push(doc),
                RelevanceClassification::Incorrect => result.incorrect.push(doc),
            }
        }

        result
    }

    /// Get all relevant documents (correct + optionally ambiguous)
    pub fn relevant_documents(&self, include_ambiguous: bool) -> Vec<&ScoredDocument> {
        let mut docs: Vec<&ScoredDocument> = self.correct.iter().collect();

        if include_ambiguous {
            docs.extend(self.ambiguous.iter());
        }

        docs
    }

    /// Get the number of relevant documents
    pub fn relevant_count(&self, include_ambiguous: bool) -> usize {
        if include_ambiguous {
            self.correct.len() + self.ambiguous.len()
        } else {
            self.correct.len()
        }
    }

    /// Check if any relevant documents were found
    pub fn has_relevant(&self) -> bool {
        !self.correct.is_empty()
    }

    /// Check if any ambiguous documents were found
    pub fn has_ambiguous(&self) -> bool {
        !self.ambiguous.is_empty()
    }

    /// Get summary statistics
    pub fn summary(&self) -> CragSummary {
        CragSummary {
            total: self.total_processed,
            correct: self.correct.len(),
            ambiguous: self.ambiguous.len(),
            incorrect: self.incorrect.len(),
        }
    }
}

impl Default for CragResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Summary statistics for CRAG results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CragSummary {
    pub total: usize,
    pub correct: usize,
    pub ambiguous: usize,
    pub incorrect: usize,
}

impl CragSummary {
    /// Get the percentage of correct documents
    pub fn correct_percentage(&self) -> f32 {
        if self.total == 0 {
            0.0
        } else {
            (self.correct as f32 / self.total as f32) * 100.0
        }
    }

    /// Get the percentage of relevant documents (correct + ambiguous)
    pub fn relevant_percentage(&self) -> f32 {
        if self.total == 0 {
            0.0
        } else {
            ((self.correct + self.ambiguous) as f32 / self.total as f32) * 100.0
        }
    }
}

/// Filter for applying CRAG results
#[derive(Debug, Clone)]
pub struct CragFilter {
    config: CragConfig,
}

impl CragFilter {
    /// Create a new CRAG filter with the given configuration
    pub fn new(config: CragConfig) -> Self {
        Self { config }
    }

    /// Filter scored documents based on configuration
    pub fn filter(&self, documents: Vec<ScoredDocument>) -> CragResult {
        let result = CragResult::from_scored_documents(documents);
        result
    }

    /// Get documents that should be included in the final response
    pub fn get_included_documents<'a>(&self, result: &'a CragResult) -> Vec<&'a ScoredDocument> {
        result.relevant_documents(self.config.include_ambiguous)
    }

    /// Check if correction is needed (no relevant documents found)
    pub fn needs_correction(&self, result: &CragResult) -> bool {
        if self.config.include_ambiguous {
            result.correct.is_empty() && result.ambiguous.is_empty()
        } else {
            result.correct.is_empty()
        }
    }

    /// Check if web search might help (has ambiguous but no correct)
    pub fn should_try_web_search(&self, result: &CragResult) -> bool {
        result.correct.is_empty() && !result.ambiguous.is_empty()
    }

    /// Get the configuration
    pub fn config(&self) -> &CragConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::knowledge_base::SearchResult;

    fn create_scored_doc(id: &str, score: f32, classification: RelevanceClassification) -> ScoredDocument {
        let doc = SearchResult::new(id, format!("Content for {}", id), score);
        ScoredDocument::new(doc, score, classification)
    }

    #[test]
    fn test_crag_result_from_scored_documents() {
        let docs = vec![
            create_scored_doc("doc-1", 0.9, RelevanceClassification::Correct),
            create_scored_doc("doc-2", 0.7, RelevanceClassification::Ambiguous),
            create_scored_doc("doc-3", 0.3, RelevanceClassification::Incorrect),
            create_scored_doc("doc-4", 0.85, RelevanceClassification::Correct),
        ];

        let result = CragResult::from_scored_documents(docs);

        assert_eq!(result.total_processed, 4);
        assert_eq!(result.correct.len(), 2);
        assert_eq!(result.ambiguous.len(), 1);
        assert_eq!(result.incorrect.len(), 1);
    }

    #[test]
    fn test_relevant_documents_with_ambiguous() {
        let docs = vec![
            create_scored_doc("doc-1", 0.9, RelevanceClassification::Correct),
            create_scored_doc("doc-2", 0.7, RelevanceClassification::Ambiguous),
        ];
        let result = CragResult::from_scored_documents(docs);

        assert_eq!(result.relevant_documents(true).len(), 2);
        assert_eq!(result.relevant_documents(false).len(), 1);
    }

    #[test]
    fn test_crag_summary() {
        let docs = vec![
            create_scored_doc("doc-1", 0.9, RelevanceClassification::Correct),
            create_scored_doc("doc-2", 0.7, RelevanceClassification::Ambiguous),
            create_scored_doc("doc-3", 0.3, RelevanceClassification::Incorrect),
            create_scored_doc("doc-4", 0.85, RelevanceClassification::Correct),
        ];
        let result = CragResult::from_scored_documents(docs);
        let summary = result.summary();

        assert_eq!(summary.total, 4);
        assert_eq!(summary.correct, 2);
        assert_eq!(summary.ambiguous, 1);
        assert_eq!(summary.incorrect, 1);
        assert_eq!(summary.correct_percentage(), 50.0);
        assert_eq!(summary.relevant_percentage(), 75.0);
    }

    #[test]
    fn test_crag_filter_needs_correction() {
        let config = CragConfig::default();
        let filter = CragFilter::new(config);

        // Has correct documents - no correction needed
        let docs_with_correct = vec![
            create_scored_doc("doc-1", 0.9, RelevanceClassification::Correct),
        ];
        let result = filter.filter(docs_with_correct);
        assert!(!filter.needs_correction(&result));

        // Only ambiguous documents (include_ambiguous = true) - no correction
        let docs_only_ambiguous = vec![
            create_scored_doc("doc-1", 0.7, RelevanceClassification::Ambiguous),
        ];
        let result = filter.filter(docs_only_ambiguous);
        assert!(!filter.needs_correction(&result));

        // Only incorrect documents - needs correction
        let docs_only_incorrect = vec![
            create_scored_doc("doc-1", 0.3, RelevanceClassification::Incorrect),
        ];
        let result = filter.filter(docs_only_incorrect);
        assert!(filter.needs_correction(&result));
    }

    #[test]
    fn test_crag_filter_needs_correction_exclude_ambiguous() {
        let config = CragConfig::default().with_include_ambiguous(false);
        let filter = CragFilter::new(config);

        // Only ambiguous documents (include_ambiguous = false) - needs correction
        let docs_only_ambiguous = vec![
            create_scored_doc("doc-1", 0.7, RelevanceClassification::Ambiguous),
        ];
        let result = filter.filter(docs_only_ambiguous);
        assert!(filter.needs_correction(&result));
    }

    #[test]
    fn test_should_try_web_search() {
        let config = CragConfig::default();
        let filter = CragFilter::new(config);

        // Has correct - no web search
        let docs = vec![
            create_scored_doc("doc-1", 0.9, RelevanceClassification::Correct),
        ];
        let result = filter.filter(docs);
        assert!(!filter.should_try_web_search(&result));

        // Only ambiguous - should try web search
        let docs = vec![
            create_scored_doc("doc-1", 0.7, RelevanceClassification::Ambiguous),
        ];
        let result = filter.filter(docs);
        assert!(filter.should_try_web_search(&result));

        // Only incorrect - should not try (would need different correction)
        let docs = vec![
            create_scored_doc("doc-1", 0.3, RelevanceClassification::Incorrect),
        ];
        let result = filter.filter(docs);
        assert!(!filter.should_try_web_search(&result));
    }

    #[test]
    fn test_get_included_documents() {
        let docs = vec![
            create_scored_doc("doc-1", 0.9, RelevanceClassification::Correct),
            create_scored_doc("doc-2", 0.7, RelevanceClassification::Ambiguous),
            create_scored_doc("doc-3", 0.3, RelevanceClassification::Incorrect),
        ];

        // Include ambiguous
        let config = CragConfig::default().with_include_ambiguous(true);
        let filter = CragFilter::new(config);
        let result = filter.filter(docs.clone());
        let included = filter.get_included_documents(&result);
        assert_eq!(included.len(), 2);

        // Exclude ambiguous
        let config = CragConfig::default().with_include_ambiguous(false);
        let filter = CragFilter::new(config);
        let result = filter.filter(docs);
        let included = filter.get_included_documents(&result);
        assert_eq!(included.len(), 1);
    }

    #[test]
    fn test_empty_result() {
        let result = CragResult::new();

        assert_eq!(result.total_processed, 0);
        assert!(result.correct.is_empty());
        assert!(result.ambiguous.is_empty());
        assert!(result.incorrect.is_empty());
        assert!(!result.has_relevant());
        assert!(!result.has_ambiguous());

        let summary = result.summary();
        assert_eq!(summary.correct_percentage(), 0.0);
        assert_eq!(summary.relevant_percentage(), 0.0);
    }
}
