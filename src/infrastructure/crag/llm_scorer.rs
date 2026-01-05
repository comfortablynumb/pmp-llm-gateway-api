//! LLM-based document scorer
//!
//! Uses an LLM to evaluate document relevance to a query.

use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use tracing::{debug, warn};

use crate::domain::crag::{CragConfig, DocumentScorer, ScoredDocument, ScoringInput};
use crate::domain::knowledge_base::SearchResult;
use crate::domain::llm::{LlmProvider, LlmRequest};
use crate::domain::DomainError;

/// Document scorer that uses an LLM for evaluation
#[derive(Debug)]
pub struct LlmDocumentScorer<P>
where
    P: LlmProvider,
{
    provider: Arc<P>,
    config: CragConfig,
    model: String,
}

impl<P: LlmProvider> LlmDocumentScorer<P> {
    /// Create a new LLM-based scorer
    pub fn new(provider: Arc<P>, model: impl Into<String>, config: CragConfig) -> Self {
        Self {
            provider,
            config,
            model: model.into(),
        }
    }

    /// Create with default configuration
    pub fn with_defaults(provider: Arc<P>, model: impl Into<String>) -> Self {
        Self::new(provider, model, CragConfig::llm_based())
    }

    fn build_evaluation_prompt(&self, query: &str, document: &SearchResult) -> String {
        let template = self.config.get_evaluation_prompt();
        template
            .replace("${query}", query)
            .replace("${document_content}", &document.content)
            .replace("${document_id}", &document.id)
    }

    fn parse_llm_response(&self, response: &str) -> Result<LlmScoreResponse, DomainError> {
        // Try to extract JSON from the response
        let json_str = extract_json(response).unwrap_or(response);

        serde_json::from_str(json_str).map_err(|e| {
            warn!("Failed to parse LLM scoring response: {} - Response: {}", e, response);
            DomainError::validation(format!(
                "Invalid LLM scoring response format: {}",
                e
            ))
        })
    }

    fn normalize_score(&self, score: f32) -> f32 {
        // LLM returns 0-10, we normalize to 0-1
        (score / 10.0).clamp(0.0, 1.0)
    }
}

/// Response structure from LLM scoring
#[derive(Debug, Deserialize)]
struct LlmScoreResponse {
    score: f32,
    reason: Option<String>,
}

/// Extract JSON object from a string (handles markdown code blocks)
fn extract_json(text: &str) -> Option<&str> {
    // Try to find JSON object directly
    if let Some(start) = text.find('{') {
        if let Some(end) = text.rfind('}') {
            if start < end {
                return Some(&text[start..=end]);
            }
        }
    }

    None
}

#[async_trait]
impl<P: LlmProvider> DocumentScorer for LlmDocumentScorer<P> {
    async fn score_document(
        &self,
        query: &str,
        document: &SearchResult,
    ) -> Result<ScoredDocument, DomainError> {
        let prompt = self.build_evaluation_prompt(query, document);

        debug!(
            "Scoring document {} with LLM (model: {})",
            document.id, self.model
        );

        let request = LlmRequest::builder()
            .user(prompt)
            .temperature(self.config.temperature)
            .max_tokens(150)
            .build();

        let response = self.provider.chat(&self.model, request).await?;

        let content = response
            .content()
            .ok_or_else(|| DomainError::provider("llm_scorer", "Empty response from LLM"))?;

        let parsed = self.parse_llm_response(content)?;
        let normalized_score = self.normalize_score(parsed.score);
        let classification = self.config.classify(normalized_score);

        debug!(
            "Document {} scored: raw={}, normalized={}, classification={:?}",
            document.id, parsed.score, normalized_score, classification
        );

        let mut scored = ScoredDocument::new(document.clone(), normalized_score, classification);

        if let Some(reason) = parsed.reason {
            scored = scored.with_reason(reason);
        }

        Ok(scored)
    }

    async fn score_documents(
        &self,
        input: ScoringInput,
    ) -> Result<Vec<ScoredDocument>, DomainError> {
        // Limit the number of documents to score
        let docs_to_score: Vec<_> = input
            .documents
            .iter()
            .take(self.config.max_documents_to_score)
            .collect();

        debug!(
            "Scoring {} documents (limited to {}) with LLM",
            docs_to_score.len(),
            self.config.max_documents_to_score
        );

        let mut results = Vec::with_capacity(docs_to_score.len());

        for document in docs_to_score {
            match self.score_document(&input.query, document).await {
                Ok(scored) => results.push(scored),
                Err(e) => {
                    warn!("Failed to score document {}: {}", document.id, e);
                    // On error, use similarity score as fallback
                    let classification = self.config.classify(document.score);
                    results.push(
                        ScoredDocument::new(document.clone(), document.score, classification)
                            .with_reason(format!("Fallback due to error: {}", e)),
                    );
                }
            }
        }

        Ok(results)
    }

    fn scorer_name(&self) -> &'static str {
        "llm"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::llm::MockLlmProvider;
    use crate::domain::llm::{LlmResponse, Message as LlmMessage};

    fn create_test_document(id: &str, score: f32) -> SearchResult {
        SearchResult::new(id, format!("Content for {}", id), score)
    }

    fn create_mock_provider_with_response(score: f32, reason: &str) -> Arc<MockLlmProvider> {
        let response_json = format!(r#"{{"score": {}, "reason": "{}"}}"#, score, reason);
        let response = LlmResponse::new(
            "resp-1".to_string(),
            "gpt-4".to_string(),
            LlmMessage::assistant(response_json),
        );
        Arc::new(MockLlmProvider::new("mock").with_response(response))
    }

    #[test]
    fn test_extract_json() {
        let text = r#"Here is the result: {"score": 8, "reason": "Relevant"}"#;
        let json = extract_json(text).unwrap();
        assert_eq!(json, r#"{"score": 8, "reason": "Relevant"}"#);
    }

    #[test]
    fn test_extract_json_with_markdown() {
        let text = r#"```json
{"score": 7, "reason": "Partially relevant"}
```"#;
        let json = extract_json(text).unwrap();
        assert_eq!(json, r#"{"score": 7, "reason": "Partially relevant"}"#);
    }

    #[test]
    fn test_extract_json_none() {
        let text = "No JSON here";
        assert!(extract_json(text).is_none());
    }

    #[tokio::test]
    async fn test_llm_scorer_high_score() {
        let provider = create_mock_provider_with_response(9.0, "Highly relevant");
        let scorer = LlmDocumentScorer::with_defaults(provider, "gpt-4");
        let doc = create_test_document("doc-1", 0.5);

        let result = scorer.score_document("test query", &doc).await.unwrap();

        assert!(result.is_relevant());
        assert_eq!(result.relevance_score, 0.9); // 9/10 = 0.9
        assert_eq!(result.reason, Some("Highly relevant".to_string()));
    }

    #[tokio::test]
    async fn test_llm_scorer_medium_score() {
        let provider = create_mock_provider_with_response(6.0, "Somewhat related");
        let scorer = LlmDocumentScorer::with_defaults(provider, "gpt-4");
        let doc = create_test_document("doc-1", 0.5);

        let result = scorer.score_document("test query", &doc).await.unwrap();

        assert!(result.is_ambiguous());
        assert_eq!(result.relevance_score, 0.6); // 6/10 = 0.6
    }

    #[tokio::test]
    async fn test_llm_scorer_low_score() {
        let provider = create_mock_provider_with_response(2.0, "Not relevant");
        let scorer = LlmDocumentScorer::with_defaults(provider, "gpt-4");
        let doc = create_test_document("doc-1", 0.5);

        let result = scorer.score_document("test query", &doc).await.unwrap();

        assert!(result.is_irrelevant());
        assert_eq!(result.relevance_score, 0.2); // 2/10 = 0.2
    }

    #[tokio::test]
    async fn test_llm_scorer_score_clamping() {
        // Test score above 10 gets clamped
        let provider = create_mock_provider_with_response(15.0, "Very high");
        let scorer = LlmDocumentScorer::with_defaults(provider, "gpt-4");
        let doc = create_test_document("doc-1", 0.5);

        let result = scorer.score_document("test query", &doc).await.unwrap();

        assert_eq!(result.relevance_score, 1.0); // Clamped to 1.0
    }

    #[tokio::test]
    async fn test_llm_scorer_error_fallback() {
        let provider = Arc::new(MockLlmProvider::new("mock").with_error("API error"));
        let scorer = LlmDocumentScorer::with_defaults(provider, "gpt-4");

        let docs = vec![create_test_document("doc-1", 0.75)];
        let input = ScoringInput::new("test query", docs);

        let results = scorer.score_documents(input).await.unwrap();

        // Should fallback to document score
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].relevance_score, 0.75);
        assert!(results[0].reason.as_ref().unwrap().contains("Fallback"));
    }

    #[tokio::test]
    async fn test_llm_scorer_respects_max_documents() {
        let provider = create_mock_provider_with_response(8.0, "Relevant");
        let config = CragConfig::llm_based().with_max_documents(2);
        let scorer = LlmDocumentScorer::new(provider, "gpt-4", config);

        let docs = vec![
            create_test_document("doc-1", 0.9),
            create_test_document("doc-2", 0.8),
            create_test_document("doc-3", 0.7),
            create_test_document("doc-4", 0.6),
        ];
        let input = ScoringInput::new("test query", docs);

        let results = scorer.score_documents(input).await.unwrap();

        // Should only score first 2 documents
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_build_evaluation_prompt() {
        let provider = Arc::new(MockLlmProvider::new("mock"));
        let scorer = LlmDocumentScorer::with_defaults(provider, "gpt-4");

        let doc = SearchResult::new("doc-123", "This is the document content", 0.8);
        let prompt = scorer.build_evaluation_prompt("What is AI?", &doc);

        assert!(prompt.contains("What is AI?"));
        assert!(prompt.contains("This is the document content"));
    }

    #[test]
    fn test_scorer_name() {
        let provider = Arc::new(MockLlmProvider::new("mock"));
        let scorer = LlmDocumentScorer::with_defaults(provider, "gpt-4");
        assert_eq!(scorer.scorer_name(), "llm");
    }
}
