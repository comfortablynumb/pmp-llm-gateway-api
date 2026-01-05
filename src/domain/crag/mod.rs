//! CRAG (Corrective RAG) domain
//!
//! This module provides domain types and traits for implementing Corrective RAG,
//! which scores and filters retrieved documents based on their relevance to a query.

mod config;
mod pipeline;
mod scorer;

pub use config::{CragConfig, RelevanceClassification, ScoringStrategy};
pub use pipeline::{CragFilter, CragResult, CragSummary};
pub use scorer::{DocumentScorer, ScoredDocument, ScoringInput};

#[cfg(test)]
pub use scorer::mock::MockDocumentScorer;
