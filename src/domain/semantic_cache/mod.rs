//! Semantic cache domain models and traits
//!
//! Provides vector-based caching that matches semantically similar queries
//! rather than requiring exact key matches.

mod config;
mod repository;

pub use config::SemanticCacheConfig;
pub use repository::{
    CachedEntry, SemanticCache, SemanticCacheStats, SemanticSearchParams, SemanticSearchResult,
};
