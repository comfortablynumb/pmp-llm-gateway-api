//! Experiment repository traits and query types

use async_trait::async_trait;
use std::fmt::Debug;

use super::entity::{Experiment, ExperimentId, ExperimentStatus};
use super::record::{ExperimentRecord, ExperimentRecordId};
use crate::domain::DomainError;

// ============================================================================
// ExperimentQuery
// ============================================================================

/// Query parameters for listing experiments
#[derive(Debug, Clone, Default)]
pub struct ExperimentQuery {
    /// Filter by status
    pub status: Option<ExperimentStatus>,
    /// Filter by model ID (matches any variant)
    pub model_id: Option<String>,
    /// Maximum number of results
    pub limit: Option<usize>,
    /// Number of results to skip
    pub offset: Option<usize>,
}

impl ExperimentQuery {
    /// Create a new query with no filters
    pub fn new() -> Self {
        Self::default()
    }

    /// Filter by status
    pub fn with_status(mut self, status: ExperimentStatus) -> Self {
        self.status = Some(status);
        self
    }

    /// Filter by model ID
    pub fn with_model(mut self, model_id: impl Into<String>) -> Self {
        self.model_id = Some(model_id.into());
        self
    }

    /// Set maximum number of results
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Set number of results to skip
    pub fn with_offset(mut self, offset: usize) -> Self {
        self.offset = Some(offset);
        self
    }
}

// ============================================================================
// ExperimentRecordQuery
// ============================================================================

/// Query parameters for experiment records
#[derive(Debug, Clone, Default)]
pub struct ExperimentRecordQuery {
    /// Filter by experiment ID
    pub experiment_id: Option<String>,
    /// Filter by variant ID
    pub variant_id: Option<String>,
    /// Filter by API key ID
    pub api_key_id: Option<String>,
    /// Filter by start timestamp (inclusive)
    pub from_timestamp: Option<u64>,
    /// Filter by end timestamp (exclusive)
    pub to_timestamp: Option<u64>,
    /// Maximum number of results
    pub limit: Option<usize>,
    /// Number of results to skip
    pub offset: Option<usize>,
}

impl ExperimentRecordQuery {
    /// Create a new query with no filters
    pub fn new() -> Self {
        Self::default()
    }

    /// Filter by experiment ID
    pub fn with_experiment(mut self, experiment_id: impl Into<String>) -> Self {
        self.experiment_id = Some(experiment_id.into());
        self
    }

    /// Filter by variant ID
    pub fn with_variant(mut self, variant_id: impl Into<String>) -> Self {
        self.variant_id = Some(variant_id.into());
        self
    }

    /// Filter by API key ID
    pub fn with_api_key(mut self, api_key_id: impl Into<String>) -> Self {
        self.api_key_id = Some(api_key_id.into());
        self
    }

    /// Filter by time range
    pub fn with_time_range(mut self, from: u64, to: u64) -> Self {
        self.from_timestamp = Some(from);
        self.to_timestamp = Some(to);
        self
    }

    /// Set maximum number of results
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Set number of results to skip
    pub fn with_offset(mut self, offset: usize) -> Self {
        self.offset = Some(offset);
        self
    }
}

// ============================================================================
// ExperimentRepository
// ============================================================================

/// Repository trait for experiments
#[async_trait]
pub trait ExperimentRepository: Send + Sync + Debug {
    /// Create a new experiment
    async fn create(&self, experiment: Experiment) -> Result<Experiment, DomainError>;

    /// Get an experiment by ID
    async fn get(&self, id: &ExperimentId) -> Result<Option<Experiment>, DomainError>;

    /// Update an existing experiment
    async fn update(&self, experiment: Experiment) -> Result<Experiment, DomainError>;

    /// Delete an experiment by ID
    async fn delete(&self, id: &ExperimentId) -> Result<bool, DomainError>;

    /// List experiments with optional filters
    async fn list(&self, query: &ExperimentQuery) -> Result<Vec<Experiment>, DomainError>;

    /// Find active experiments that include the given model ID
    async fn find_active_for_model(&self, model_id: &str) -> Result<Vec<Experiment>, DomainError>;

    /// Check if an experiment exists
    async fn exists(&self, id: &ExperimentId) -> Result<bool, DomainError> {
        Ok(self.get(id).await?.is_some())
    }

    /// Count experiments matching the query
    async fn count(&self, query: &ExperimentQuery) -> Result<usize, DomainError> {
        Ok(self.list(query).await?.len())
    }
}

// ============================================================================
// ExperimentRecordRepository
// ============================================================================

/// Repository trait for experiment records
#[async_trait]
pub trait ExperimentRecordRepository: Send + Sync + Debug {
    /// Record a new experiment result
    async fn record(&self, record: ExperimentRecord) -> Result<(), DomainError>;

    /// Get a record by ID
    async fn get(&self, id: &ExperimentRecordId) -> Result<Option<ExperimentRecord>, DomainError>;

    /// Query records with filters
    async fn query(&self, query: &ExperimentRecordQuery) -> Result<Vec<ExperimentRecord>, DomainError>;

    /// Count records matching the query
    async fn count(&self, query: &ExperimentRecordQuery) -> Result<usize, DomainError>;

    /// Get latency values for a specific variant in an experiment
    async fn get_latencies_for_variant(
        &self,
        experiment_id: &str,
        variant_id: &str,
    ) -> Result<Vec<u64>, DomainError>;

    /// Delete all records for an experiment
    async fn delete_by_experiment(&self, experiment_id: &str) -> Result<usize, DomainError>;
}

#[cfg(test)]
pub mod mock {
    //! Mock implementations for testing

    use super::*;
    use std::collections::HashMap;
    use std::sync::RwLock;

    /// Mock experiment repository for testing
    #[derive(Debug, Default)]
    pub struct MockExperimentRepository {
        experiments: RwLock<HashMap<String, Experiment>>,
        should_fail: RwLock<bool>,
    }

    impl MockExperimentRepository {
        pub fn new() -> Self {
            Self::default()
        }

        pub fn with_error(self) -> Self {
            *self.should_fail.write().unwrap() = true;
            self
        }

        fn check_should_fail(&self) -> Result<(), DomainError> {
            if *self.should_fail.read().unwrap() {
                Err(DomainError::internal("Mock error"))
            } else {
                Ok(())
            }
        }
    }

    #[async_trait]
    impl ExperimentRepository for MockExperimentRepository {
        async fn create(&self, experiment: Experiment) -> Result<Experiment, DomainError> {
            self.check_should_fail()?;
            let id = experiment.id().as_str().to_string();
            let mut experiments = self.experiments.write().unwrap();

            if experiments.contains_key(&id) {
                return Err(DomainError::conflict(format!(
                    "Experiment '{}' already exists",
                    id
                )));
            }

            experiments.insert(id, experiment.clone());
            Ok(experiment)
        }

        async fn get(&self, id: &ExperimentId) -> Result<Option<Experiment>, DomainError> {
            self.check_should_fail()?;
            let experiments = self.experiments.read().unwrap();
            Ok(experiments.get(id.as_str()).cloned())
        }

        async fn update(&self, experiment: Experiment) -> Result<Experiment, DomainError> {
            self.check_should_fail()?;
            let id = experiment.id().as_str().to_string();
            let mut experiments = self.experiments.write().unwrap();

            if !experiments.contains_key(&id) {
                return Err(DomainError::not_found(format!(
                    "Experiment '{}' not found",
                    id
                )));
            }

            experiments.insert(id, experiment.clone());
            Ok(experiment)
        }

        async fn delete(&self, id: &ExperimentId) -> Result<bool, DomainError> {
            self.check_should_fail()?;
            let mut experiments = self.experiments.write().unwrap();
            Ok(experiments.remove(id.as_str()).is_some())
        }

        async fn list(&self, query: &ExperimentQuery) -> Result<Vec<Experiment>, DomainError> {
            self.check_should_fail()?;
            let experiments = self.experiments.read().unwrap();

            let mut results: Vec<_> = experiments
                .values()
                .filter(|e| {
                    if let Some(status) = query.status {
                        if e.status() != status {
                            return false;
                        }
                    }

                    if let Some(ref model_id) = query.model_id {
                        if !e.referenced_model_ids().contains(&model_id.as_str()) {
                            return false;
                        }
                    }

                    true
                })
                .cloned()
                .collect();

            results.sort_by(|a, b| b.created_at().cmp(&a.created_at()));

            let offset = query.offset.unwrap_or(0);
            let limit = query.limit.unwrap_or(usize::MAX);

            Ok(results.into_iter().skip(offset).take(limit).collect())
        }

        async fn find_active_for_model(
            &self,
            model_id: &str,
        ) -> Result<Vec<Experiment>, DomainError> {
            self.list(
                &ExperimentQuery::new()
                    .with_status(ExperimentStatus::Active)
                    .with_model(model_id),
            )
            .await
        }
    }

    /// Mock experiment record repository for testing
    #[derive(Debug, Default)]
    pub struct MockExperimentRecordRepository {
        records: RwLock<HashMap<String, ExperimentRecord>>,
        should_fail: RwLock<bool>,
    }

    impl MockExperimentRecordRepository {
        pub fn new() -> Self {
            Self::default()
        }

        pub fn with_error(self) -> Self {
            *self.should_fail.write().unwrap() = true;
            self
        }

        fn check_should_fail(&self) -> Result<(), DomainError> {
            if *self.should_fail.read().unwrap() {
                Err(DomainError::internal("Mock error"))
            } else {
                Ok(())
            }
        }
    }

    #[async_trait]
    impl ExperimentRecordRepository for MockExperimentRecordRepository {
        async fn record(&self, record: ExperimentRecord) -> Result<(), DomainError> {
            self.check_should_fail()?;
            let mut records = self.records.write().unwrap();
            records.insert(record.id().to_string(), record);
            Ok(())
        }

        async fn get(
            &self,
            id: &ExperimentRecordId,
        ) -> Result<Option<ExperimentRecord>, DomainError> {
            self.check_should_fail()?;
            let records = self.records.read().unwrap();
            Ok(records.get(id.as_str()).cloned())
        }

        async fn query(
            &self,
            query: &ExperimentRecordQuery,
        ) -> Result<Vec<ExperimentRecord>, DomainError> {
            self.check_should_fail()?;
            let records = self.records.read().unwrap();

            let mut results: Vec<_> = records
                .values()
                .filter(|r| {
                    if let Some(ref exp_id) = query.experiment_id {
                        if &r.experiment_id != exp_id {
                            return false;
                        }
                    }

                    if let Some(ref var_id) = query.variant_id {
                        if &r.variant_id != var_id {
                            return false;
                        }
                    }

                    if let Some(ref api_key) = query.api_key_id {
                        if &r.api_key_id != api_key {
                            return false;
                        }
                    }

                    if let Some(from) = query.from_timestamp {
                        if r.timestamp < from {
                            return false;
                        }
                    }

                    if let Some(to) = query.to_timestamp {
                        if r.timestamp >= to {
                            return false;
                        }
                    }

                    true
                })
                .cloned()
                .collect();

            results.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

            let offset = query.offset.unwrap_or(0);
            let limit = query.limit.unwrap_or(usize::MAX);

            Ok(results.into_iter().skip(offset).take(limit).collect())
        }

        async fn count(&self, query: &ExperimentRecordQuery) -> Result<usize, DomainError> {
            Ok(self.query(query).await?.len())
        }

        async fn get_latencies_for_variant(
            &self,
            experiment_id: &str,
            variant_id: &str,
        ) -> Result<Vec<u64>, DomainError> {
            self.check_should_fail()?;
            let records = self.records.read().unwrap();

            Ok(records
                .values()
                .filter(|r| r.experiment_id == experiment_id && r.variant_id == variant_id)
                .map(|r| r.latency_ms)
                .collect())
        }

        async fn delete_by_experiment(&self, experiment_id: &str) -> Result<usize, DomainError> {
            self.check_should_fail()?;
            let mut records = self.records.write().unwrap();
            let before = records.len();
            records.retain(|_, r| r.experiment_id != experiment_id);
            Ok(before - records.len())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::mock::*;
    use super::*;
    use crate::domain::experiment::{TrafficAllocation, Variant, VariantConfig, VariantId};

    fn create_test_experiment(id: &str) -> Experiment {
        let exp_id = ExperimentId::new(id).unwrap();
        let control_id = VariantId::new("control").unwrap();
        let treatment_id = VariantId::new("treatment").unwrap();

        Experiment::new(exp_id, format!("Experiment {}", id))
            .with_variant(
                Variant::new(
                    control_id.clone(),
                    "Control",
                    VariantConfig::model_reference("gpt-4"),
                )
                .with_control(true),
            )
            .with_variant(Variant::new(
                treatment_id.clone(),
                "Treatment",
                VariantConfig::model_reference("gpt-4-turbo"),
            ))
            .with_traffic_allocation(TrafficAllocation::new(control_id, 50))
            .with_traffic_allocation(TrafficAllocation::new(treatment_id, 50))
    }

    #[tokio::test]
    async fn test_mock_experiment_repository_crud() {
        let repo = MockExperimentRepository::new();

        // Create
        let exp = create_test_experiment("test-1");
        let created = repo.create(exp.clone()).await.unwrap();
        assert_eq!(created.id().as_str(), "test-1");

        // Get
        let exp_id = ExperimentId::new("test-1").unwrap();
        let fetched = repo.get(&exp_id).await.unwrap();
        assert!(fetched.is_some());

        // Update
        let mut updated = fetched.unwrap();
        updated.set_name("Updated Name");
        let result = repo.update(updated).await.unwrap();
        assert_eq!(result.name(), "Updated Name");

        // Delete
        let deleted = repo.delete(&exp_id).await.unwrap();
        assert!(deleted);

        // Get after delete
        let fetched = repo.get(&exp_id).await.unwrap();
        assert!(fetched.is_none());
    }

    #[tokio::test]
    async fn test_mock_experiment_repository_list() {
        let repo = MockExperimentRepository::new();

        // Create multiple experiments
        for i in 1..=5 {
            let exp = create_test_experiment(&format!("exp-{}", i));
            repo.create(exp).await.unwrap();
        }

        // List all
        let all = repo.list(&ExperimentQuery::new()).await.unwrap();
        assert_eq!(all.len(), 5);

        // List with limit
        let limited = repo
            .list(&ExperimentQuery::new().with_limit(3))
            .await
            .unwrap();
        assert_eq!(limited.len(), 3);
    }

    #[tokio::test]
    async fn test_mock_record_repository_crud() {
        let repo = MockExperimentRecordRepository::new();

        // Record
        let record = ExperimentRecord::new("rec-1", "exp-1", "control", "api-key-1")
            .with_model_id("gpt-4")
            .with_tokens(100, 50)
            .with_latency_ms(200);

        repo.record(record).await.unwrap();

        // Get
        let fetched = repo.get(&ExperimentRecordId::from("rec-1")).await.unwrap();
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().latency_ms, 200);
    }

    #[tokio::test]
    async fn test_mock_record_repository_query() {
        let repo = MockExperimentRecordRepository::new();

        // Create records for different variants
        for i in 1..=10 {
            let variant = if i <= 5 { "control" } else { "treatment" };
            let record = ExperimentRecord::new(format!("rec-{}", i), "exp-1", variant, "api-1")
                .with_latency_ms(100 + i as u64 * 10);
            repo.record(record).await.unwrap();
        }

        // Query by variant
        let control_records = repo
            .query(&ExperimentRecordQuery::new().with_variant("control"))
            .await
            .unwrap();
        assert_eq!(control_records.len(), 5);

        // Get latencies
        let latencies = repo
            .get_latencies_for_variant("exp-1", "control")
            .await
            .unwrap();
        assert_eq!(latencies.len(), 5);
    }
}
