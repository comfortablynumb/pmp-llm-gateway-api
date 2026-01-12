//! Storage-backed experiment record repository implementation

use async_trait::async_trait;
use std::sync::Arc;

use crate::domain::experiment::{
    ExperimentRecord, ExperimentRecordId, ExperimentRecordQuery, ExperimentRecordRepository,
};
use crate::domain::storage::Storage;
use crate::domain::DomainError;

/// Storage-backed implementation of ExperimentRecordRepository
#[derive(Debug)]
pub struct StorageExperimentRecordRepository {
    storage: Arc<dyn Storage<ExperimentRecord>>,
}

impl StorageExperimentRecordRepository {
    /// Create a new storage-backed repository
    pub fn new(storage: Arc<dyn Storage<ExperimentRecord>>) -> Self {
        Self { storage }
    }

    fn filter_records<'a>(
        &self,
        records: impl Iterator<Item = &'a ExperimentRecord>,
        query: &ExperimentRecordQuery,
    ) -> Vec<&'a ExperimentRecord> {
        records
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
            .collect()
    }
}

#[async_trait]
impl ExperimentRecordRepository for StorageExperimentRecordRepository {
    async fn record(&self, record: ExperimentRecord) -> Result<(), DomainError> {
        self.storage.create(record).await?;
        Ok(())
    }

    async fn get(&self, id: &ExperimentRecordId) -> Result<Option<ExperimentRecord>, DomainError> {
        self.storage.get(id).await
    }

    async fn query(
        &self,
        query: &ExperimentRecordQuery,
    ) -> Result<Vec<ExperimentRecord>, DomainError> {
        let all = self.storage.list().await?;
        let mut filtered: Vec<ExperimentRecord> = self
            .filter_records(all.iter(), query)
            .into_iter()
            .cloned()
            .collect();

        // Sort by timestamp descending (newest first)
        filtered.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        // Apply pagination
        let offset = query.offset.unwrap_or(0);
        let limit = query.limit.unwrap_or(usize::MAX);

        Ok(filtered.into_iter().skip(offset).take(limit).collect())
    }

    async fn count(&self, query: &ExperimentRecordQuery) -> Result<usize, DomainError> {
        // For count, we don't apply pagination
        let all = self.storage.list().await?;
        Ok(self.filter_records(all.iter(), query).len())
    }

    async fn get_latencies_for_variant(
        &self,
        experiment_id: &str,
        variant_id: &str,
    ) -> Result<Vec<u64>, DomainError> {
        let all = self.storage.list().await?;

        Ok(all
            .iter()
            .filter(|r| r.experiment_id == experiment_id && r.variant_id == variant_id)
            .map(|r| r.latency_ms)
            .collect())
    }

    async fn delete_by_experiment(&self, experiment_id: &str) -> Result<usize, DomainError> {
        let all = self.storage.list().await?;
        let mut deleted = 0;

        for record in all {
            if record.experiment_id == experiment_id {
                if self.storage.delete(record.id()).await? {
                    deleted += 1;
                }
            }
        }

        Ok(deleted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::storage::InMemoryStorage;

    fn create_repo() -> StorageExperimentRecordRepository {
        let storage = Arc::new(InMemoryStorage::<ExperimentRecord>::new());
        StorageExperimentRecordRepository::new(storage)
    }

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
        let repo = create_repo();
        let record = create_test_record("rec-1", "exp-1", "control", 1000);

        repo.record(record).await.unwrap();

        let fetched = repo.get(&ExperimentRecordId::from("rec-1")).await.unwrap();
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().experiment_id, "exp-1");
    }

    #[tokio::test]
    async fn test_query_all() {
        let repo = create_repo();

        for i in 1..=10 {
            let record = create_test_record(&format!("rec-{}", i), "exp-1", "control", i as u64);
            repo.record(record).await.unwrap();
        }

        let results = repo.query(&ExperimentRecordQuery::new()).await.unwrap();
        assert_eq!(results.len(), 10);
    }

    #[tokio::test]
    async fn test_query_by_experiment() {
        let repo = create_repo();

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
    async fn test_count() {
        let repo = create_repo();

        for i in 1..=10 {
            let record = create_test_record(&format!("rec-{}", i), "exp-1", "control", i as u64);
            repo.record(record).await.unwrap();
        }

        let count = repo.count(&ExperimentRecordQuery::new()).await.unwrap();
        assert_eq!(count, 10);
    }

    #[tokio::test]
    async fn test_get_latencies_for_variant() {
        let repo = create_repo();

        for i in 1..=5 {
            let mut record =
                create_test_record(&format!("rec-c-{}", i), "exp-1", "control", i as u64);
            record.latency_ms = 100 + i as u64 * 10;
            repo.record(record).await.unwrap();
        }

        let latencies = repo
            .get_latencies_for_variant("exp-1", "control")
            .await
            .unwrap();
        assert_eq!(latencies.len(), 5);
    }

    #[tokio::test]
    async fn test_delete_by_experiment() {
        let repo = create_repo();

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
}
