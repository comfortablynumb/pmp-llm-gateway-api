//! In-memory semantic cache implementation

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::RwLock;

use async_trait::async_trait;

use crate::domain::embedding::cosine_similarity;
use crate::domain::semantic_cache::{
    CachedEntry, SemanticCache, SemanticCacheStats, SemanticSearchParams, SemanticSearchResult,
};
use crate::domain::DomainError;

/// In-memory semantic cache using linear search
///
/// Suitable for development and small-scale deployments.
/// For production with large cache sizes, use PgvectorSemanticCache.
#[derive(Debug)]
pub struct InMemorySemanticCache {
    entries: RwLock<HashMap<String, CachedEntry>>,
    max_entries: usize,
    hits: AtomicU64,
    misses: AtomicU64,
    evictions: AtomicU64,
    total_similarity: RwLock<f64>,
    hit_count_for_avg: RwLock<u64>,
}

impl InMemorySemanticCache {
    /// Create a new in-memory semantic cache
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
            max_entries,
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            evictions: AtomicU64::new(0),
            total_similarity: RwLock::new(0.0),
            hit_count_for_avg: RwLock::new(0),
        }
    }

    /// Evict oldest entries if cache is full
    fn evict_if_needed(&self, entries: &mut HashMap<String, CachedEntry>) {
        if entries.len() < self.max_entries {
            return;
        }

        // Find and remove the oldest entry
        if let Some(oldest_id) = entries
            .iter()
            .min_by_key(|(_, entry)| entry.created_at())
            .map(|(id, _)| id.clone())
        {
            entries.remove(&oldest_id);
            self.evictions.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Check if entry matches the filter criteria
    fn matches_filter(entry: &CachedEntry, params: &SemanticSearchParams) -> bool {
        // Check model ID filter
        if let Some(ref model_id) = params.model_id {
            if entry.model_id() != Some(model_id.as_str()) {
                return false;
            }
        }

        // Check temperature filter (with small tolerance)
        if let Some(temp) = params.temperature {
            if let Some(entry_temp) = entry.temperature() {
                if (entry_temp - temp).abs() > 0.01 {
                    return false;
                }
            }
        }

        true
    }

    #[allow(dead_code)]
    fn update_avg_similarity(&self, similarity: f32) {
        let mut total = self.total_similarity.write().unwrap();
        let mut count = self.hit_count_for_avg.write().unwrap();
        *total += similarity as f64;
        *count += 1;
    }

    fn get_avg_similarity(&self) -> f32 {
        let total = *self.total_similarity.read().unwrap();
        let count = *self.hit_count_for_avg.read().unwrap();

        if count == 0 {
            return 0.0;
        }

        (total / count as f64) as f32
    }
}

#[async_trait]
impl SemanticCache for InMemorySemanticCache {
    async fn search(
        &self,
        embedding: &[f32],
        params: &SemanticSearchParams,
    ) -> Result<Vec<SemanticSearchResult>, DomainError> {
        let entries = self.entries.read().map_err(|e| {
            DomainError::internal(format!("Failed to acquire read lock: {}", e))
        })?;

        let mut results: Vec<SemanticSearchResult> = entries
            .values()
            .filter(|entry| !entry.is_expired())
            .filter(|entry| Self::matches_filter(entry, params))
            .map(|entry| {
                let similarity = cosine_similarity(embedding, entry.embedding());
                SemanticSearchResult::new(entry.clone(), similarity)
            })
            .filter(|result| result.similarity >= params.min_similarity)
            .collect();

        // Sort by similarity descending
        results.sort_by(|a, b| {
            b.similarity
                .partial_cmp(&a.similarity)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Limit results
        results.truncate(params.limit);

        Ok(results)
    }

    async fn store(&self, entry: CachedEntry) -> Result<(), DomainError> {
        let mut entries = self.entries.write().map_err(|e| {
            DomainError::internal(format!("Failed to acquire write lock: {}", e))
        })?;

        self.evict_if_needed(&mut entries);
        entries.insert(entry.id().to_string(), entry);

        Ok(())
    }

    async fn get(&self, id: &str) -> Result<Option<CachedEntry>, DomainError> {
        let entries = self.entries.read().map_err(|e| {
            DomainError::internal(format!("Failed to acquire read lock: {}", e))
        })?;

        let entry = entries.get(id).cloned();

        // Filter out expired entries
        Ok(entry.filter(|e| !e.is_expired()))
    }

    async fn delete(&self, id: &str) -> Result<bool, DomainError> {
        let mut entries = self.entries.write().map_err(|e| {
            DomainError::internal(format!("Failed to acquire write lock: {}", e))
        })?;

        Ok(entries.remove(id).is_some())
    }

    async fn delete_by_model(&self, model_id: &str) -> Result<usize, DomainError> {
        let mut entries = self.entries.write().map_err(|e| {
            DomainError::internal(format!("Failed to acquire write lock: {}", e))
        })?;

        let keys_to_remove: Vec<String> = entries
            .iter()
            .filter(|(_, entry)| entry.model_id() == Some(model_id))
            .map(|(id, _)| id.clone())
            .collect();

        let count = keys_to_remove.len();

        for key in keys_to_remove {
            entries.remove(&key);
        }

        Ok(count)
    }

    async fn clear(&self) -> Result<(), DomainError> {
        let mut entries = self.entries.write().map_err(|e| {
            DomainError::internal(format!("Failed to acquire write lock: {}", e))
        })?;

        entries.clear();
        self.hits.store(0, Ordering::Relaxed);
        self.misses.store(0, Ordering::Relaxed);
        *self.total_similarity.write().unwrap() = 0.0;
        *self.hit_count_for_avg.write().unwrap() = 0;

        Ok(())
    }

    async fn stats(&self) -> Result<SemanticCacheStats, DomainError> {
        let entries = self.entries.read().map_err(|e| {
            DomainError::internal(format!("Failed to acquire read lock: {}", e))
        })?;

        Ok(SemanticCacheStats {
            total_entries: entries.len(),
            hits: self.hits.load(Ordering::Relaxed),
            misses: self.misses.load(Ordering::Relaxed),
            evictions: self.evictions.load(Ordering::Relaxed),
            avg_hit_similarity: self.get_avg_similarity(),
        })
    }

    async fn size(&self) -> Result<usize, DomainError> {
        let entries = self.entries.read().map_err(|e| {
            DomainError::internal(format!("Failed to acquire read lock: {}", e))
        })?;

        Ok(entries.len())
    }

    async fn record_hit(&self, id: &str) -> Result<(), DomainError> {
        self.hits.fetch_add(1, Ordering::Relaxed);

        // Update hit count on the entry
        let mut entries = self.entries.write().map_err(|e| {
            DomainError::internal(format!("Failed to acquire write lock: {}", e))
        })?;

        if let Some(entry) = entries.get_mut(id) {
            entry.increment_hits();
        }

        Ok(())
    }

    async fn record_miss(&self) -> Result<(), DomainError> {
        self.misses.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    async fn cleanup_expired(&self) -> Result<usize, DomainError> {
        let mut entries = self.entries.write().map_err(|e| {
            DomainError::internal(format!("Failed to acquire write lock: {}", e))
        })?;

        let expired_keys: Vec<String> = entries
            .iter()
            .filter(|(_, entry)| entry.is_expired())
            .map(|(id, _)| id.clone())
            .collect();

        let count = expired_keys.len();

        for key in expired_keys {
            entries.remove(&key);
        }

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn create_entry(id: &str, embedding: Vec<f32>, model_id: Option<&str>) -> CachedEntry {
        let mut entry = CachedEntry::new(
            id,
            embedding,
            format!("query for {}", id),
            format!(r#"{{"response": "{}"}}"#, id),
            Duration::from_secs(3600),
        );

        if let Some(model) = model_id {
            entry = entry.with_model_id(model);
        }

        entry
    }

    #[tokio::test]
    async fn test_store_and_get() {
        let cache = InMemorySemanticCache::new(100);
        let entry = create_entry("test-1", vec![0.1, 0.2, 0.3], None);

        cache.store(entry.clone()).await.unwrap();

        let retrieved = cache.get("test-1").await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id(), "test-1");
    }

    #[tokio::test]
    async fn test_search_similar() {
        let cache = InMemorySemanticCache::new(100);

        // Store an entry with a known embedding
        let entry = create_entry("test-1", vec![1.0, 0.0, 0.0], None);
        cache.store(entry).await.unwrap();

        // Search with identical embedding
        let params = SemanticSearchParams::new(0.9);
        let results = cache.search(&[1.0, 0.0, 0.0], &params).await.unwrap();

        assert_eq!(results.len(), 1);
        assert!((results[0].similarity - 1.0).abs() < 0.001);
    }

    #[tokio::test]
    async fn test_search_with_threshold() {
        let cache = InMemorySemanticCache::new(100);

        // Store entries
        cache
            .store(create_entry("similar", vec![1.0, 0.1, 0.0], None))
            .await
            .unwrap();
        cache
            .store(create_entry("different", vec![0.0, 1.0, 0.0], None))
            .await
            .unwrap();

        // Search with high threshold
        let params = SemanticSearchParams::new(0.95);
        let results = cache.search(&[1.0, 0.0, 0.0], &params).await.unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].entry.id(), "similar");
    }

    #[tokio::test]
    async fn test_search_with_model_filter() {
        let cache = InMemorySemanticCache::new(100);

        cache
            .store(create_entry("gpt4-entry", vec![1.0, 0.0, 0.0], Some("gpt-4")))
            .await
            .unwrap();
        cache
            .store(create_entry("gpt35-entry", vec![1.0, 0.0, 0.0], Some("gpt-3.5")))
            .await
            .unwrap();

        let params = SemanticSearchParams::new(0.9).with_model_id("gpt-4");
        let results = cache.search(&[1.0, 0.0, 0.0], &params).await.unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].entry.id(), "gpt4-entry");
    }

    #[tokio::test]
    async fn test_delete() {
        let cache = InMemorySemanticCache::new(100);

        cache
            .store(create_entry("test-1", vec![0.1, 0.2], None))
            .await
            .unwrap();
        assert!(cache.get("test-1").await.unwrap().is_some());

        let deleted = cache.delete("test-1").await.unwrap();
        assert!(deleted);
        assert!(cache.get("test-1").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_delete_by_model() {
        let cache = InMemorySemanticCache::new(100);

        cache
            .store(create_entry("entry-1", vec![0.1], Some("gpt-4")))
            .await
            .unwrap();
        cache
            .store(create_entry("entry-2", vec![0.2], Some("gpt-4")))
            .await
            .unwrap();
        cache
            .store(create_entry("entry-3", vec![0.3], Some("gpt-3.5")))
            .await
            .unwrap();

        let deleted = cache.delete_by_model("gpt-4").await.unwrap();

        assert_eq!(deleted, 2);
        assert_eq!(cache.size().await.unwrap(), 1);
    }

    #[tokio::test]
    async fn test_eviction() {
        let cache = InMemorySemanticCache::new(3);

        // Store 3 entries (at capacity)
        for i in 0..3 {
            cache
                .store(create_entry(&format!("entry-{}", i), vec![i as f32], None))
                .await
                .unwrap();
        }

        assert_eq!(cache.size().await.unwrap(), 3);

        // Store one more (should evict oldest)
        cache
            .store(create_entry("entry-new", vec![9.0], None))
            .await
            .unwrap();

        assert_eq!(cache.size().await.unwrap(), 3);

        let stats = cache.stats().await.unwrap();
        assert_eq!(stats.evictions, 1);
    }

    #[tokio::test]
    async fn test_stats() {
        let cache = InMemorySemanticCache::new(100);

        cache
            .store(create_entry("test-1", vec![1.0, 0.0], None))
            .await
            .unwrap();
        cache
            .store(create_entry("test-2", vec![0.0, 1.0], None))
            .await
            .unwrap();

        cache.record_hit("test-1").await.unwrap();
        cache.record_hit("test-1").await.unwrap();
        cache.record_miss().await.unwrap();

        let stats = cache.stats().await.unwrap();

        assert_eq!(stats.total_entries, 2);
        assert_eq!(stats.hits, 2);
        assert_eq!(stats.misses, 1);
    }

    #[tokio::test]
    async fn test_clear() {
        let cache = InMemorySemanticCache::new(100);

        cache
            .store(create_entry("test-1", vec![0.1], None))
            .await
            .unwrap();
        cache
            .store(create_entry("test-2", vec![0.2], None))
            .await
            .unwrap();
        cache.record_hit("test-1").await.unwrap();

        cache.clear().await.unwrap();

        assert_eq!(cache.size().await.unwrap(), 0);

        let stats = cache.stats().await.unwrap();
        assert_eq!(stats.hits, 0);
    }

    #[tokio::test]
    async fn test_expired_entries_not_returned() {
        let cache = InMemorySemanticCache::new(100);

        // Create an already-expired entry
        let entry = CachedEntry::new(
            "expired",
            vec![1.0, 0.0],
            "query",
            "value",
            Duration::from_secs(0),
        );

        // Force expiration
        // Note: This is a bit hacky, but the entry should be expired immediately
        std::thread::sleep(Duration::from_millis(10));

        cache.store(entry).await.unwrap();

        // Get should return None for expired
        let retrieved = cache.get("expired").await.unwrap();
        assert!(retrieved.is_none());

        // Search should not return expired
        let params = SemanticSearchParams::new(0.0);
        let results = cache.search(&[1.0, 0.0], &params).await.unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_cleanup_expired() {
        let cache = InMemorySemanticCache::new(100);

        // Store a normal entry
        cache
            .store(create_entry("valid", vec![0.1], None))
            .await
            .unwrap();

        // Store an expired entry (with 0 TTL)
        let expired = CachedEntry::new(
            "expired",
            vec![0.2],
            "query",
            "value",
            Duration::from_secs(0),
        );
        cache.store(expired).await.unwrap();

        // Wait a bit to ensure expiration
        std::thread::sleep(Duration::from_millis(10));

        let cleaned = cache.cleanup_expired().await.unwrap();

        assert_eq!(cleaned, 1);
        assert_eq!(cache.size().await.unwrap(), 1);
    }

    #[tokio::test]
    async fn test_search_ordering() {
        let cache = InMemorySemanticCache::new(100);

        // Store entries with different similarities to query [1.0, 0.0, 0.0]
        cache
            .store(create_entry("low", vec![0.5, 0.5, 0.5], None))
            .await
            .unwrap();
        cache
            .store(create_entry("high", vec![0.99, 0.1, 0.0], None))
            .await
            .unwrap();
        cache
            .store(create_entry("medium", vec![0.8, 0.3, 0.0], None))
            .await
            .unwrap();

        let params = SemanticSearchParams::new(0.5).with_limit(3);
        let results = cache.search(&[1.0, 0.0, 0.0], &params).await.unwrap();

        // Should be ordered by similarity descending
        assert!(results[0].similarity >= results[1].similarity);
        assert!(results[1].similarity >= results[2].similarity);
    }
}
