//! In-memory implementation of the experiment record repository

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::RwLock;

use crate::domain::experiment::{
    ExperimentRecord, ExperimentRecordId, ExperimentRecordQuery, ExperimentRecordRepository,
};
use crate::domain::DomainError;

/// In-memory experiment record repository implementation
#[derive(Debug)]
pub struct InMemoryExperimentRecordRepository {
    records: RwLock<HashMap<ExperimentRecordId, ExperimentRecord>>,
    max_records: usize,
}

impl InMemoryExperimentRecordRepository {
    /// Create a new empty repository with default max records (100,000)
    pub fn new() -> Self {
        Self {
            records: RwLock::new(HashMap::new()),
            max_records: 100_000,
        }
    }

    /// Create a repository with a custom max records limit
    pub fn with_max_records(max_records: usize) -> Self {
        Self {
            records: RwLock::new(HashMap::new()),
            max_records,
        }
    }

    /// Evict oldest records if we're over the limit
    fn evict_if_needed(records: &mut HashMap<ExperimentRecordId, ExperimentRecord>, max_records: usize) {
        if records.len() <= max_records {
            return;
        }

        // Find and remove oldest records
        let mut entries: Vec<_> = records
            .iter()
            .map(|(k, v)| (k.clone(), v.timestamp))
            .collect();
        entries.sort_by(|a, b| a.1.cmp(&b.1));

        let to_remove = records.len() - max_records;

        for (id, _) in entries.into_iter().take(to_remove) {
            records.remove(&id);
        }
    }
}

impl Default for InMemoryExperimentRecordRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ExperimentRecordRepository for InMemoryExperimentRecordRepository {
    async fn record(&self, record: ExperimentRecord) -> Result<(), DomainError> {
        let mut records = self
            .records
            .write()
            .map_err(|e| DomainError::internal(format!("Failed to acquire write lock: {}", e)))?;

        records.insert(record.id().clone(), record);
        Self::evict_if_needed(&mut records, self.max_records);

        Ok(())
    }

    async fn get(&self, id: &ExperimentRecordId) -> Result<Option<ExperimentRecord>, DomainError> {
        let records = self
            .records
            .read()
            .map_err(|e| DomainError::internal(format!("Failed to acquire read lock: {}", e)))?;

        Ok(records.get(id).cloned())
    }

    async fn query(
        &self,
        query: &ExperimentRecordQuery,
    ) -> Result<Vec<ExperimentRecord>, DomainError> {
        let records = self
            .records
            .read()
            .map_err(|e| DomainError::internal(format!("Failed to acquire read lock: {}", e)))?;

        let mut results: Vec<_> = records
            .values()
            .filter(|r| {
                // Filter by experiment ID
                if let Some(ref exp_id) = query.experiment_id {
                    if &r.experiment_id != exp_id {
                        return false;
                    }
                }

                // Filter by variant ID
                if let Some(ref var_id) = query.variant_id {
                    if &r.variant_id != var_id {
                        return false;
                    }
                }

                // Filter by API key ID
                if let Some(ref api_key) = query.api_key_id {
                    if &r.api_key_id != api_key {
                        return false;
                    }
                }

                // Filter by start timestamp
                if let Some(from) = query.from_timestamp {
                    if r.timestamp < from {
                        return false;
                    }
                }

                // Filter by end timestamp
                if let Some(to) = query.to_timestamp {
                    if r.timestamp >= to {
                        return false;
                    }
                }

                true
            })
            .cloned()
            .collect();

        // Sort by timestamp descending (newest first)
        results.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        // Apply pagination
        let offset = query.offset.unwrap_or(0);
        let limit = query.limit.unwrap_or(usize::MAX);

        Ok(results.into_iter().skip(offset).take(limit).collect())
    }

    async fn count(&self, query: &ExperimentRecordQuery) -> Result<usize, DomainError> {
        // For count, we don't apply pagination
        let mut count_query = query.clone();
        count_query.offset = None;
        count_query.limit = None;

        Ok(self.query(&count_query).await?.len())
    }

    async fn get_latencies_for_variant(
        &self,
        experiment_id: &str,
        variant_id: &str,
    ) -> Result<Vec<u64>, DomainError> {
        let records = self
            .records
            .read()
            .map_err(|e| DomainError::internal(format!("Failed to acquire read lock: {}", e)))?;

        Ok(records
            .values()
            .filter(|r| r.experiment_id == experiment_id && r.variant_id == variant_id)
            .map(|r| r.latency_ms)
            .collect())
    }

    async fn delete_by_experiment(&self, experiment_id: &str) -> Result<usize, DomainError> {
        let mut records = self
            .records
            .write()
            .map_err(|e| DomainError::internal(format!("Failed to acquire write lock: {}", e)))?;

        let before = records.len();
        records.retain(|_, r| r.experiment_id != experiment_id);
        Ok(before - records.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_record(
        id: &str,
        experiment_id: &str,
        variant_id: &str,
        timestamp: u64,
    ) -> ExperimentRecord {
        ExperimentRecord::new(id, experiment_id, variant_id, "api-key-1")
            .with_model_id("gpt-4")
            .with_tokens(100, 50)
            .with_latency_ms(200)
            .with_timestamp(timestamp)
    }

    #[tokio::test]
    async fn test_record_and_get() {
        let repo = InMemoryExperimentRecordRepository::new();
        let record = create_test_record("rec-1", "exp-1", "control", 1000);

        repo.record(record).await.unwrap();

        let fetched = repo.get(&ExperimentRecordId::from("rec-1")).await.unwrap();
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().experiment_id, "exp-1");
    }

    #[tokio::test]
    async fn test_get_not_found() {
        let repo = InMemoryExperimentRecordRepository::new();

        let fetched = repo.get(&ExperimentRecordId::from("nonexistent")).await.unwrap();
        assert!(fetched.is_none());
    }

    #[tokio::test]
    async fn test_query_all() {
        let repo = InMemoryExperimentRecordRepository::new();

        for i in 1..=10 {
            let record = create_test_record(&format!("rec-{}", i), "exp-1", "control", i as u64);
            repo.record(record).await.unwrap();
        }

        let results = repo.query(&ExperimentRecordQuery::new()).await.unwrap();
        assert_eq!(results.len(), 10);
    }

    #[tokio::test]
    async fn test_query_by_experiment() {
        let repo = InMemoryExperimentRecordRepository::new();

        for i in 1..=5 {
            let record = create_test_record(&format!("rec-a-{}", i), "exp-1", "control", i as u64);
            repo.record(record).await.unwrap();
        }

        for i in 1..=5 {
            let record = create_test_record(&format!("rec-b-{}", i), "exp-2", "control", i as u64);
            repo.record(record).await.unwrap();
        }

        let results = repo
            .query(&ExperimentRecordQuery::new().with_experiment("exp-1"))
            .await
            .unwrap();
        assert_eq!(results.len(), 5);
        assert!(results.iter().all(|r| r.experiment_id == "exp-1"));
    }

    #[tokio::test]
    async fn test_query_by_variant() {
        let repo = InMemoryExperimentRecordRepository::new();

        for i in 1..=5 {
            let record = create_test_record(&format!("rec-c-{}", i), "exp-1", "control", i as u64);
            repo.record(record).await.unwrap();
        }

        for i in 1..=5 {
            let record =
                create_test_record(&format!("rec-t-{}", i), "exp-1", "treatment", i as u64);
            repo.record(record).await.unwrap();
        }

        let results = repo
            .query(&ExperimentRecordQuery::new().with_variant("control"))
            .await
            .unwrap();
        assert_eq!(results.len(), 5);
        assert!(results.iter().all(|r| r.variant_id == "control"));
    }

    #[tokio::test]
    async fn test_query_by_time_range() {
        let repo = InMemoryExperimentRecordRepository::new();

        for i in 1..=10 {
            let record = create_test_record(&format!("rec-{}", i), "exp-1", "control", i as u64);
            repo.record(record).await.unwrap();
        }

        let results = repo
            .query(&ExperimentRecordQuery::new().with_time_range(3, 8))
            .await
            .unwrap();

        // Should include timestamps 3, 4, 5, 6, 7 (8 is exclusive)
        assert_eq!(results.len(), 5);
        assert!(results.iter().all(|r| r.timestamp >= 3 && r.timestamp < 8));
    }

    #[tokio::test]
    async fn test_query_with_pagination() {
        let repo = InMemoryExperimentRecordRepository::new();

        for i in 1..=20 {
            let record = create_test_record(&format!("rec-{}", i), "exp-1", "control", i as u64);
            repo.record(record).await.unwrap();
        }

        let page1 = repo
            .query(&ExperimentRecordQuery::new().with_limit(5))
            .await
            .unwrap();
        assert_eq!(page1.len(), 5);

        let page2 = repo
            .query(&ExperimentRecordQuery::new().with_offset(5).with_limit(5))
            .await
            .unwrap();
        assert_eq!(page2.len(), 5);

        // Pages should be different
        assert_ne!(page1[0].id().as_str(), page2[0].id().as_str());
    }

    #[tokio::test]
    async fn test_query_sorted_by_timestamp_desc() {
        let repo = InMemoryExperimentRecordRepository::new();

        // Insert in random order
        for i in [5, 2, 8, 1, 9, 3] {
            let record = create_test_record(&format!("rec-{}", i), "exp-1", "control", i as u64);
            repo.record(record).await.unwrap();
        }

        let results = repo.query(&ExperimentRecordQuery::new()).await.unwrap();

        // Should be sorted descending
        for i in 0..results.len() - 1 {
            assert!(results[i].timestamp >= results[i + 1].timestamp);
        }
    }

    #[tokio::test]
    async fn test_count() {
        let repo = InMemoryExperimentRecordRepository::new();

        for i in 1..=10 {
            let record = create_test_record(&format!("rec-{}", i), "exp-1", "control", i as u64);
            repo.record(record).await.unwrap();
        }

        let count = repo.count(&ExperimentRecordQuery::new()).await.unwrap();
        assert_eq!(count, 10);

        // Count ignores pagination
        let count_with_limit = repo
            .count(&ExperimentRecordQuery::new().with_limit(5))
            .await
            .unwrap();
        assert_eq!(count_with_limit, 10);
    }

    #[tokio::test]
    async fn test_get_latencies_for_variant() {
        let repo = InMemoryExperimentRecordRepository::new();

        // Create records with different latencies
        for i in 1..=5 {
            let mut record =
                create_test_record(&format!("rec-c-{}", i), "exp-1", "control", i as u64);
            record.latency_ms = 100 + i as u64 * 10;
            repo.record(record).await.unwrap();
        }

        for i in 1..=5 {
            let mut record =
                create_test_record(&format!("rec-t-{}", i), "exp-1", "treatment", i as u64);
            record.latency_ms = 80 + i as u64 * 10;
            repo.record(record).await.unwrap();
        }

        let control_latencies = repo
            .get_latencies_for_variant("exp-1", "control")
            .await
            .unwrap();
        assert_eq!(control_latencies.len(), 5);

        let treatment_latencies = repo
            .get_latencies_for_variant("exp-1", "treatment")
            .await
            .unwrap();
        assert_eq!(treatment_latencies.len(), 5);

        // Control should have higher latencies
        let control_avg: f64 =
            control_latencies.iter().sum::<u64>() as f64 / control_latencies.len() as f64;
        let treatment_avg: f64 =
            treatment_latencies.iter().sum::<u64>() as f64 / treatment_latencies.len() as f64;
        assert!(control_avg > treatment_avg);
    }

    #[tokio::test]
    async fn test_delete_by_experiment() {
        let repo = InMemoryExperimentRecordRepository::new();

        for i in 1..=5 {
            let record = create_test_record(&format!("rec-a-{}", i), "exp-1", "control", i as u64);
            repo.record(record).await.unwrap();
        }

        for i in 1..=5 {
            let record = create_test_record(&format!("rec-b-{}", i), "exp-2", "control", i as u64);
            repo.record(record).await.unwrap();
        }

        let deleted = repo.delete_by_experiment("exp-1").await.unwrap();
        assert_eq!(deleted, 5);

        let remaining = repo.query(&ExperimentRecordQuery::new()).await.unwrap();
        assert_eq!(remaining.len(), 5);
        assert!(remaining.iter().all(|r| r.experiment_id == "exp-2"));
    }

    #[tokio::test]
    async fn test_eviction() {
        let repo = InMemoryExperimentRecordRepository::with_max_records(10);

        // Insert 15 records
        for i in 1..=15 {
            let record = create_test_record(&format!("rec-{}", i), "exp-1", "control", i as u64);
            repo.record(record).await.unwrap();
        }

        let all = repo.query(&ExperimentRecordQuery::new()).await.unwrap();
        assert_eq!(all.len(), 10);

        // Oldest should have been evicted (1-5)
        for r in &all {
            let num: u64 = r.id().as_str().strip_prefix("rec-").unwrap().parse().unwrap();
            assert!(num > 5, "Record {} should have been evicted", num);
        }
    }

    #[tokio::test]
    async fn test_combined_filters() {
        let repo = InMemoryExperimentRecordRepository::new();

        // Create various records
        for exp in ["exp-1", "exp-2"] {
            for variant in ["control", "treatment"] {
                for i in 1..=5 {
                    let record = create_test_record(
                        &format!("{}-{}-{}", exp, variant, i),
                        exp,
                        variant,
                        i as u64,
                    );
                    repo.record(record).await.unwrap();
                }
            }
        }

        // Total: 20 records

        // Query exp-1, control only
        let results = repo
            .query(
                &ExperimentRecordQuery::new()
                    .with_experiment("exp-1")
                    .with_variant("control"),
            )
            .await
            .unwrap();
        assert_eq!(results.len(), 5);
        assert!(results
            .iter()
            .all(|r| r.experiment_id == "exp-1" && r.variant_id == "control"));
    }
}
