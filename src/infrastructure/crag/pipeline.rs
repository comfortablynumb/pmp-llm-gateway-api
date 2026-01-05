//! CRAG Pipeline implementation
//!
//! Integrates knowledge base search with document scoring and filtering.

use std::sync::Arc;

use tracing::{debug, info};

use crate::domain::crag::{
    CragConfig, CragFilter, CragResult, DocumentScorer, ScoringInput, ScoringStrategy,
};
use crate::domain::knowledge_base::{KnowledgeBaseProvider, SearchParams};
use crate::domain::DomainError;

use super::{LlmDocumentScorer, ThresholdDocumentScorer};
use crate::domain::llm::LlmProvider;

/// CRAG pipeline that combines knowledge base search with document scoring
#[derive(Debug)]
pub struct CragPipeline<K, P>
where
    K: KnowledgeBaseProvider,
    P: LlmProvider,
{
    knowledge_base: Arc<K>,
    llm_provider: Option<Arc<P>>,
    config: CragConfig,
}

impl<K, P> CragPipeline<K, P>
where
    K: KnowledgeBaseProvider,
    P: LlmProvider,
{
    /// Create a new CRAG pipeline
    pub fn new(knowledge_base: Arc<K>, config: CragConfig) -> Self {
        Self {
            knowledge_base,
            llm_provider: None,
            config,
        }
    }

    /// Add an LLM provider for LLM-based scoring
    pub fn with_llm_provider(mut self, provider: Arc<P>) -> Self {
        self.llm_provider = Some(provider);
        self
    }

    /// Execute CRAG search and filtering
    pub async fn search(&self, params: SearchParams) -> Result<CragResult, DomainError> {
        info!(
            "CRAG search: kb={}, query={}, strategy={:?}",
            self.knowledge_base.knowledge_base_id(),
            params.query,
            self.config.strategy
        );

        // Step 1: Search the knowledge base
        let query_text = params.query.clone();
        let search_results = self.knowledge_base.search(params).await?;

        debug!(
            "Retrieved {} documents from knowledge base",
            search_results.len()
        );

        if search_results.is_empty() {
            return Ok(CragResult::new());
        }

        // Step 2: Score documents based on strategy
        let scored_documents = match self.config.strategy {
            ScoringStrategy::ThresholdBased => {
                let scorer = ThresholdDocumentScorer::new(self.config.clone());
                let input = ScoringInput::new(&query_text, search_results);
                scorer.score_documents(input).await?
            }
            ScoringStrategy::LlmBased => {
                let llm = self.llm_provider.as_ref().ok_or_else(|| {
                    DomainError::validation(
                        "LLM provider required for LLM-based scoring strategy",
                    )
                })?;
                let model = self
                    .config
                    .scoring_model
                    .clone()
                    .unwrap_or_else(|| "gpt-4".to_string());
                let scorer = LlmDocumentScorer::new(llm.clone(), model, self.config.clone());
                let input = ScoringInput::new(&query_text, search_results);
                scorer.score_documents(input).await?
            }
            ScoringStrategy::Hybrid => {
                // First filter by threshold, then use LLM for borderline cases
                self.hybrid_scoring(&query_text, search_results).await?
            }
        };

        // Step 3: Filter and categorize results
        let filter = CragFilter::new(self.config.clone());
        let result = filter.filter(scored_documents);

        let summary = result.summary();
        info!(
            "CRAG complete: total={}, correct={}, ambiguous={}, incorrect={}",
            summary.total, summary.correct, summary.ambiguous, summary.incorrect
        );

        Ok(result)
    }

    /// Hybrid scoring: threshold first, then LLM for ambiguous
    async fn hybrid_scoring(
        &self,
        query: &str,
        documents: Vec<crate::domain::knowledge_base::SearchResult>,
    ) -> Result<Vec<crate::domain::crag::ScoredDocument>, DomainError> {
        debug!("Hybrid scoring: threshold pass first");

        // First pass: threshold scoring
        let threshold_scorer = ThresholdDocumentScorer::new(self.config.clone());
        let input = ScoringInput::new(query, documents);
        let mut initial_scores = threshold_scorer.score_documents(input).await?;

        // Find documents that need LLM evaluation (ambiguous only)
        let ambiguous_indices: Vec<usize> = initial_scores
            .iter()
            .enumerate()
            .filter(|(_, doc)| doc.is_ambiguous())
            .map(|(i, _)| i)
            .collect();

        if ambiguous_indices.is_empty() {
            debug!("No ambiguous documents, skipping LLM pass");
            return Ok(initial_scores);
        }

        // Second pass: LLM scoring for ambiguous documents
        if let Some(llm) = &self.llm_provider {
            debug!(
                "LLM pass for {} ambiguous documents",
                ambiguous_indices.len()
            );

            let model = self
                .config
                .scoring_model
                .clone()
                .unwrap_or_else(|| "gpt-4".to_string());
            let llm_scorer = LlmDocumentScorer::new(llm.clone(), model, self.config.clone());

            for i in ambiguous_indices {
                let doc = &initial_scores[i].document;

                match llm_scorer.score_document(query, doc).await {
                    Ok(rescored) => {
                        initial_scores[i] = rescored;
                    }
                    Err(e) => {
                        debug!("LLM scoring failed for {}: {}, keeping threshold score", doc.id, e);
                    }
                }
            }
        } else {
            debug!("No LLM provider, keeping threshold scores for ambiguous documents");
        }

        Ok(initial_scores)
    }

    /// Get the configuration
    pub fn config(&self) -> &CragConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::crag::RelevanceClassification;
    use crate::domain::knowledge_base::{KnowledgeBaseId, MockKnowledgeBaseProvider, SearchResult};
    use crate::domain::llm::MockLlmProvider;
    use crate::domain::llm::{LlmResponse, Message};

    fn create_mock_kb_with_results(results: Vec<SearchResult>) -> Arc<MockKnowledgeBaseProvider> {
        let id = KnowledgeBaseId::new("test-kb").unwrap();
        Arc::new(MockKnowledgeBaseProvider::new(id).with_search_results(results))
    }

    fn create_mock_llm_with_score(score: f32) -> Arc<MockLlmProvider> {
        let response_json = format!(r#"{{"score": {}, "reason": "Test"}}"#, score);
        let response = LlmResponse::new(
            "resp-1".to_string(),
            "gpt-4".to_string(),
            Message::assistant(response_json),
        );
        Arc::new(MockLlmProvider::new("mock").with_response(response))
    }

    #[tokio::test]
    async fn test_pipeline_threshold_strategy() {
        let results = vec![
            SearchResult::new("doc-1", "High score content", 0.9),
            SearchResult::new("doc-2", "Medium score content", 0.6),
            SearchResult::new("doc-3", "Low score content", 0.3),
        ];
        let kb = create_mock_kb_with_results(results);
        let config = CragConfig::threshold_based(0.8, 0.5);

        let pipeline: CragPipeline<MockKnowledgeBaseProvider, MockLlmProvider> =
            CragPipeline::new(kb, config);

        let params = SearchParams::new("test query");
        let result: CragResult = pipeline.search(params).await.unwrap();

        assert_eq!(result.correct.len(), 1);
        assert_eq!(result.ambiguous.len(), 1);
        assert_eq!(result.incorrect.len(), 1);
    }

    #[tokio::test]
    async fn test_pipeline_llm_strategy() {
        let results = vec![
            SearchResult::new("doc-1", "Test content", 0.5),
        ];
        let kb = create_mock_kb_with_results(results);
        let llm = create_mock_llm_with_score(9.0);
        let config = CragConfig::llm_based();

        let pipeline = CragPipeline::new(kb, config).with_llm_provider(llm);

        let params = SearchParams::new("test query");
        let result = pipeline.search(params).await.unwrap();

        // LLM scored it as 9/10 = 0.9, which is "correct"
        assert_eq!(result.correct.len(), 1);
    }

    #[tokio::test]
    async fn test_pipeline_llm_strategy_requires_provider() {
        let results = vec![SearchResult::new("doc-1", "Test content", 0.5)];
        let kb = create_mock_kb_with_results(results);
        let config = CragConfig::llm_based();

        let pipeline: CragPipeline<MockKnowledgeBaseProvider, MockLlmProvider> =
            CragPipeline::new(kb, config);

        let params = SearchParams::new("test query");
        let result: Result<CragResult, _> = pipeline.search(params).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("LLM provider required"));
    }

    #[tokio::test]
    async fn test_pipeline_empty_results() {
        let kb = create_mock_kb_with_results(vec![]);
        let config = CragConfig::default();

        let pipeline: CragPipeline<MockKnowledgeBaseProvider, MockLlmProvider> =
            CragPipeline::new(kb, config);

        let params = SearchParams::new("test query");
        let result: CragResult = pipeline.search(params).await.unwrap();

        assert_eq!(result.total_processed, 0);
        assert!(result.correct.is_empty());
        assert!(result.ambiguous.is_empty());
        assert!(result.incorrect.is_empty());
    }

    #[tokio::test]
    async fn test_pipeline_hybrid_strategy() {
        // Create docs: one clear correct, one ambiguous, one incorrect
        let results = vec![
            SearchResult::new("doc-1", "Clear correct", 0.95),
            SearchResult::new("doc-2", "Ambiguous", 0.6),
            SearchResult::new("doc-3", "Clear incorrect", 0.2),
        ];
        let kb = create_mock_kb_with_results(results);
        // LLM will score the ambiguous doc as 9.0 (correct)
        let llm = create_mock_llm_with_score(9.0);
        let config = CragConfig::new().with_strategy(ScoringStrategy::Hybrid);

        let pipeline = CragPipeline::new(kb, config).with_llm_provider(llm);

        let params = SearchParams::new("test query");
        let result = pipeline.search(params).await.unwrap();

        // After hybrid: doc-1 stays correct (threshold), doc-2 becomes correct (LLM), doc-3 stays incorrect
        assert_eq!(result.correct.len(), 2);
        assert_eq!(result.incorrect.len(), 1);
    }

    #[tokio::test]
    async fn test_pipeline_summary() {
        let results = vec![
            SearchResult::new("doc-1", "Content 1", 0.9),
            SearchResult::new("doc-2", "Content 2", 0.85),
            SearchResult::new("doc-3", "Content 3", 0.6),
            SearchResult::new("doc-4", "Content 4", 0.3),
        ];
        let kb = create_mock_kb_with_results(results);
        let config = CragConfig::threshold_based(0.8, 0.5);

        let pipeline: CragPipeline<MockKnowledgeBaseProvider, MockLlmProvider> =
            CragPipeline::new(kb, config);

        let params = SearchParams::new("test query");
        let result: CragResult = pipeline.search(params).await.unwrap();

        let summary = result.summary();
        assert_eq!(summary.total, 4);
        assert_eq!(summary.correct, 2);
        assert_eq!(summary.ambiguous, 1);
        assert_eq!(summary.incorrect, 1);
        assert_eq!(summary.correct_percentage(), 50.0);
        assert_eq!(summary.relevant_percentage(), 75.0);
    }

    #[tokio::test]
    async fn test_relevant_documents_extraction() {
        let results = vec![
            SearchResult::new("doc-1", "Content 1", 0.9),
            SearchResult::new("doc-2", "Content 2", 0.6),
            SearchResult::new("doc-3", "Content 3", 0.3),
        ];
        let kb = create_mock_kb_with_results(results);
        let config = CragConfig::threshold_based(0.8, 0.5).with_include_ambiguous(true);

        let pipeline: CragPipeline<MockKnowledgeBaseProvider, MockLlmProvider> =
            CragPipeline::new(kb, config);

        let params = SearchParams::new("test query");
        let result: CragResult = pipeline.search(params).await.unwrap();

        // With include_ambiguous = true
        assert_eq!(result.relevant_documents(true).len(), 2);
        assert_eq!(result.relevant_documents(false).len(), 1);
    }

    #[tokio::test]
    async fn test_classification_preserved() {
        let results = vec![
            SearchResult::new("doc-1", "Content", 0.9),
        ];
        let kb = create_mock_kb_with_results(results);
        let config = CragConfig::threshold_based(0.8, 0.5);

        let pipeline: CragPipeline<MockKnowledgeBaseProvider, MockLlmProvider> =
            CragPipeline::new(kb, config);

        let params = SearchParams::new("test query");
        let result: CragResult = pipeline.search(params).await.unwrap();

        assert_eq!(result.correct.len(), 1);
        assert_eq!(
            result.correct[0].classification,
            RelevanceClassification::Correct
        );
    }
}
