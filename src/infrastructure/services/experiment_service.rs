//! Experiment service for A/B testing
//!
//! Provides business logic for managing experiments, variant assignments,
//! and result analysis.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use tracing::{debug, info};
use uuid::Uuid;

use crate::domain::experiment::{
    AssignmentResult, ConfigOverrides, Experiment, ExperimentId, ExperimentQuery,
    ExperimentRecord, ExperimentRecordQuery, ExperimentRecordRepository, ExperimentRepository,
    ExperimentResult, ExperimentStatus, TrafficAllocation, Variant, VariantConfig, VariantId,
    VariantMetrics,
};
use crate::domain::DomainError;
use crate::infrastructure::experiment::{calculate_significance, ConsistentHasher};

// ============================================================================
// Request Types
// ============================================================================

/// Request to create a new experiment
#[derive(Debug, Clone)]
pub struct CreateExperimentRequest {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub variants: Vec<CreateVariantRequest>,
    pub traffic_allocation: Vec<(String, u8)>,
    pub enabled: bool,
}

/// Request to create a new variant
#[derive(Debug, Clone)]
pub struct CreateVariantRequest {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub config: VariantConfig,
    pub control: bool,
}

/// Request to update an experiment
#[derive(Debug, Clone, Default)]
pub struct UpdateExperimentRequest {
    pub name: Option<String>,
    pub description: Option<Option<String>>,
    pub variants: Option<Vec<CreateVariantRequest>>,
    pub traffic_allocation: Option<Vec<(String, u8)>>,
    pub enabled: Option<bool>,
}

/// Parameters for recording an experiment result
#[derive(Debug, Clone)]
pub struct RecordExperimentParams {
    pub experiment_id: String,
    pub variant_id: String,
    pub api_key_id: String,
    pub model_id: String,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cost_micros: i64,
    pub latency_ms: u64,
    pub success: bool,
    pub error: Option<String>,
}

// ============================================================================
// Experiment Service
// ============================================================================

/// Service for managing A/B testing experiments
#[derive(Debug)]
pub struct ExperimentService<R: ExperimentRepository, RR: ExperimentRecordRepository> {
    repository: Arc<R>,
    record_repository: Arc<RR>,
}

impl<R: ExperimentRepository, RR: ExperimentRecordRepository> ExperimentService<R, RR> {
    /// Create a new experiment service
    pub fn new(repository: Arc<R>, record_repository: Arc<RR>) -> Self {
        Self {
            repository,
            record_repository,
        }
    }

    // ========================================================================
    // CRUD Operations
    // ========================================================================

    /// Get an experiment by ID
    pub async fn get(&self, id: &str) -> Result<Option<Experiment>, DomainError> {
        let experiment_id = self.parse_id(id)?;
        self.repository.get(&experiment_id).await
    }

    /// List experiments with optional filters
    pub async fn list(&self, query: &ExperimentQuery) -> Result<Vec<Experiment>, DomainError> {
        self.repository.list(query).await
    }

    /// Create a new experiment
    pub async fn create(
        &self,
        request: CreateExperimentRequest,
    ) -> Result<Experiment, DomainError> {
        debug!(experiment_id = %request.id, "Creating experiment");

        let experiment_id = self.parse_id(&request.id)?;

        if self.repository.exists(&experiment_id).await? {
            return Err(DomainError::conflict(format!(
                "Experiment '{}' already exists",
                request.id
            )));
        }

        self.validate_create_request(&request)?;

        let mut experiment = Experiment::new(experiment_id, &request.name);

        if let Some(desc) = request.description {
            experiment = experiment.with_description(desc);
        }

        experiment = experiment.with_enabled(request.enabled);

        for variant_req in request.variants {
            let variant = self.build_variant(&variant_req)?;
            experiment = experiment.with_variant(variant);
        }

        for (variant_id_str, percentage) in request.traffic_allocation {
            let variant_id = VariantId::new(&variant_id_str)
                .map_err(|e| DomainError::validation(e.to_string()))?;
            experiment = experiment.with_traffic_allocation(TrafficAllocation::new(
                variant_id, percentage,
            ));
        }

        let created = self.repository.create(experiment).await?;
        info!(experiment_id = %request.id, "Experiment created");

        Ok(created)
    }

    /// Update an existing experiment
    pub async fn update(
        &self,
        id: &str,
        request: UpdateExperimentRequest,
    ) -> Result<Experiment, DomainError> {
        debug!(experiment_id = %id, "Updating experiment");

        let experiment_id = self.parse_id(id)?;

        let mut experiment = self
            .repository
            .get(&experiment_id)
            .await?
            .ok_or_else(|| DomainError::not_found(format!("Experiment '{}' not found", id)))?;

        if experiment.status() != ExperimentStatus::Draft {
            return Err(DomainError::validation(
                "Can only update experiments in Draft status",
            ));
        }

        if let Some(name) = request.name {
            experiment.set_name(name);
        }

        if let Some(description) = request.description {
            experiment.set_description(description);
        }

        if let Some(enabled) = request.enabled {
            experiment.set_enabled(enabled);
        }

        if let Some(variants) = request.variants {
            let mut new_variants = Vec::new();

            for variant_req in variants {
                let variant = self.build_variant(&variant_req)?;
                new_variants.push(variant);
            }

            experiment.set_variants(new_variants);
        }

        if let Some(allocation) = request.traffic_allocation {
            let mut new_allocation = Vec::new();

            for (variant_id_str, percentage) in allocation {
                let variant_id = VariantId::new(&variant_id_str)
                    .map_err(|e| DomainError::validation(e.to_string()))?;
                new_allocation.push(TrafficAllocation::new(variant_id, percentage));
            }

            experiment.set_traffic_allocation(new_allocation);
        }

        let updated = self.repository.update(experiment).await?;
        info!(experiment_id = %id, "Experiment updated");

        Ok(updated)
    }

    /// Delete an experiment
    pub async fn delete(&self, id: &str) -> Result<bool, DomainError> {
        debug!(experiment_id = %id, "Deleting experiment");

        let experiment_id = self.parse_id(id)?;

        if let Some(experiment) = self.repository.get(&experiment_id).await? {
            if experiment.status() == ExperimentStatus::Active {
                return Err(DomainError::validation(
                    "Cannot delete an active experiment. Pause or complete it first.",
                ));
            }
        }

        // Delete associated records
        let deleted_records = self.record_repository.delete_by_experiment(id).await?;
        debug!(experiment_id = %id, deleted_records, "Deleted experiment records");

        let deleted = self.repository.delete(&experiment_id).await?;

        if deleted {
            info!(experiment_id = %id, "Experiment deleted");
        }

        Ok(deleted)
    }

    // ========================================================================
    // Lifecycle Operations
    // ========================================================================

    /// Start an experiment (Draft -> Active)
    pub async fn start(&self, id: &str) -> Result<Experiment, DomainError> {
        debug!(experiment_id = %id, "Starting experiment");

        let experiment_id = self.parse_id(id)?;

        let mut experiment = self
            .repository
            .get(&experiment_id)
            .await?
            .ok_or_else(|| DomainError::not_found(format!("Experiment '{}' not found", id)))?;

        self.validate_for_start(&experiment)?;

        experiment
            .start()
            .map_err(|e| DomainError::validation(e.to_string()))?;

        let updated = self.repository.update(experiment).await?;
        info!(experiment_id = %id, "Experiment started");

        Ok(updated)
    }

    /// Pause an experiment (Active -> Paused)
    pub async fn pause(&self, id: &str) -> Result<Experiment, DomainError> {
        debug!(experiment_id = %id, "Pausing experiment");

        let experiment_id = self.parse_id(id)?;

        let mut experiment = self
            .repository
            .get(&experiment_id)
            .await?
            .ok_or_else(|| DomainError::not_found(format!("Experiment '{}' not found", id)))?;

        experiment
            .pause()
            .map_err(|e| DomainError::validation(e.to_string()))?;

        let updated = self.repository.update(experiment).await?;
        info!(experiment_id = %id, "Experiment paused");

        Ok(updated)
    }

    /// Resume an experiment (Paused -> Active)
    pub async fn resume(&self, id: &str) -> Result<Experiment, DomainError> {
        debug!(experiment_id = %id, "Resuming experiment");

        let experiment_id = self.parse_id(id)?;

        let mut experiment = self
            .repository
            .get(&experiment_id)
            .await?
            .ok_or_else(|| DomainError::not_found(format!("Experiment '{}' not found", id)))?;

        experiment
            .resume()
            .map_err(|e| DomainError::validation(e.to_string()))?;

        let updated = self.repository.update(experiment).await?;
        info!(experiment_id = %id, "Experiment resumed");

        Ok(updated)
    }

    /// Complete an experiment (Active/Paused -> Completed)
    pub async fn complete(&self, id: &str) -> Result<Experiment, DomainError> {
        debug!(experiment_id = %id, "Completing experiment");

        let experiment_id = self.parse_id(id)?;

        let mut experiment = self
            .repository
            .get(&experiment_id)
            .await?
            .ok_or_else(|| DomainError::not_found(format!("Experiment '{}' not found", id)))?;

        experiment
            .complete()
            .map_err(|e| DomainError::validation(e.to_string()))?;

        let updated = self.repository.update(experiment).await?;
        info!(experiment_id = %id, "Experiment completed");

        Ok(updated)
    }

    // ========================================================================
    // Assignment
    // ========================================================================

    /// Assign a variant to a request based on API key and model
    pub async fn assign_variant(
        &self,
        model_id: &str,
        api_key_id: &str,
    ) -> Result<Option<AssignmentResult>, DomainError> {
        let experiments = self.repository.find_active_for_model(model_id).await?;

        for experiment in experiments {
            if !experiment.is_enabled() {
                continue;
            }

            let hash = ConsistentHasher::hash_assignment(api_key_id, experiment.id().as_str());

            if let Some(variant) = experiment.get_variant_for_hash(hash) {
                debug!(
                    experiment_id = %experiment.id(),
                    variant_id = %variant.id(),
                    api_key_id = %api_key_id,
                    hash = hash,
                    "Assigned variant to request"
                );

                let config = variant.config();
                let overrides = self.extract_config_overrides(config);

                return Ok(Some(
                    AssignmentResult::new(
                        experiment.id().as_str(),
                        variant.id().as_str(),
                        config.model_id(),
                    )
                    .with_overrides(overrides),
                ));
            }
        }

        Ok(None)
    }

    // ========================================================================
    // Recording
    // ========================================================================

    /// Record an experiment result
    pub async fn record(&self, params: RecordExperimentParams) -> Result<(), DomainError> {
        let record_id = format!("expr-{}", Uuid::new_v4());

        let mut record = ExperimentRecord::new(
            record_id,
            &params.experiment_id,
            &params.variant_id,
            &params.api_key_id,
        )
        .with_model_id(&params.model_id)
        .with_tokens(params.input_tokens, params.output_tokens)
        .with_cost_micros(params.cost_micros)
        .with_latency_ms(params.latency_ms);

        if !params.success {
            if let Some(error) = params.error {
                record = record.with_error(error);
            } else {
                record = record.with_error("Unknown error");
            }
        }

        self.record_repository.record(record).await?;

        debug!(
            experiment_id = %params.experiment_id,
            variant_id = %params.variant_id,
            latency_ms = params.latency_ms,
            success = params.success,
            "Recorded experiment result"
        );

        Ok(())
    }

    // ========================================================================
    // Results
    // ========================================================================

    /// Get experiment results with metrics and statistical analysis
    pub async fn get_results(&self, id: &str) -> Result<ExperimentResult, DomainError> {
        debug!(experiment_id = %id, "Getting experiment results");

        let experiment_id = self.parse_id(id)?;

        let experiment = self
            .repository
            .get(&experiment_id)
            .await?
            .ok_or_else(|| DomainError::not_found(format!("Experiment '{}' not found", id)))?;

        let query = ExperimentRecordQuery::new().with_experiment(id);
        let records = self.record_repository.query(&query).await?;

        let mut result = ExperimentResult::new(id, experiment.name(), experiment.status());

        // Calculate duration
        if let (Some(started), completed) = (experiment.started_at(), experiment.completed_at()) {
            let end = completed.unwrap_or_else(chrono::Utc::now);
            result.duration_hours = Some((end - started).num_seconds() as f64 / 3600.0);
        }

        result.total_requests = records.len() as u64;

        // Group records by variant
        let mut variant_records: HashMap<String, Vec<&ExperimentRecord>> = HashMap::new();

        for record in &records {
            variant_records
                .entry(record.variant_id.clone())
                .or_default()
                .push(record);
        }

        // Calculate per-variant metrics
        let control_variant = experiment.control_variant();

        for variant in experiment.variants() {
            let variant_id = variant.id().as_str();
            let mut metrics = VariantMetrics::new(variant_id, variant.name());

            if let Some(records) = variant_records.get(variant_id) {
                for record in records {
                    metrics.add_record(record);
                }

                let latencies = self
                    .record_repository
                    .get_latencies_for_variant(id, variant_id)
                    .await?;

                metrics.set_latency_from_samples(latencies);
            }

            result.variant_metrics.push(metrics);
        }

        // Calculate statistical significance for non-control variants
        if let Some(control) = control_variant {
            let control_id = control.id().as_str();
            let control_latencies: Vec<f64> = self
                .record_repository
                .get_latencies_for_variant(id, control_id)
                .await?
                .into_iter()
                .map(|l| l as f64)
                .collect();

            for variant in experiment.variants() {
                if variant.is_control() {
                    continue;
                }

                let treatment_id = variant.id().as_str();
                let treatment_latencies: Vec<f64> = self
                    .record_repository
                    .get_latencies_for_variant(id, treatment_id)
                    .await?
                    .into_iter()
                    .map(|l| l as f64)
                    .collect();

                if let Some(significance) = calculate_significance(
                    &control_latencies,
                    &treatment_latencies,
                    control_id,
                    treatment_id,
                    "latency_ms",
                    0.95,
                ) {
                    result.significance_tests.push(significance);
                }
            }
        }

        // Determine winner if completed and significant
        if experiment.status() == ExperimentStatus::Completed && result.has_significant_result() {
            // Find the variant with lowest latency that's significant
            if let Some(best_sig) = result
                .significance_tests
                .iter()
                .filter(|s| s.is_significant && s.treatment_is_better_lower())
                .min_by(|a, b| {
                    a.treatment_mean
                        .partial_cmp(&b.treatment_mean)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
            {
                result.winner_variant_id = Some(best_sig.treatment_variant_id.clone());
                result.recommendation = Some(format!(
                    "Variant '{}' shows significant improvement ({:.1}% lower latency)",
                    best_sig.treatment_variant_id,
                    best_sig.relative_change.abs()
                ));
            }
        }

        Ok(result)
    }

    // ========================================================================
    // Private Helpers
    // ========================================================================

    fn parse_id(&self, id: &str) -> Result<ExperimentId, DomainError> {
        ExperimentId::new(id).map_err(|e| DomainError::validation(e.to_string()))
    }

    fn build_variant(&self, request: &CreateVariantRequest) -> Result<Variant, DomainError> {
        let variant_id =
            VariantId::new(&request.id).map_err(|e| DomainError::validation(e.to_string()))?;

        let mut variant = Variant::new(variant_id, &request.name, request.config.clone());

        if let Some(ref desc) = request.description {
            variant = variant.with_description(desc);
        }

        variant = variant.with_control(request.control);

        Ok(variant)
    }

    fn extract_config_overrides(&self, config: &VariantConfig) -> ConfigOverrides {
        match config {
            VariantConfig::ConfigOverride {
                temperature,
                max_tokens,
                top_p,
                presence_penalty,
                frequency_penalty,
                ..
            } => ConfigOverrides {
                temperature: *temperature,
                max_tokens: *max_tokens,
                top_p: *top_p,
                presence_penalty: *presence_penalty,
                frequency_penalty: *frequency_penalty,
            },
            _ => ConfigOverrides::default(),
        }
    }

    fn validate_create_request(
        &self,
        request: &CreateExperimentRequest,
    ) -> Result<(), DomainError> {
        if request.variants.len() < 2 {
            return Err(DomainError::validation(
                "Experiment must have at least 2 variants",
            ));
        }

        let total_percentage: u8 = request.traffic_allocation.iter().map(|(_, p)| *p).sum();

        if total_percentage != 100 {
            return Err(DomainError::validation(format!(
                "Traffic allocations must sum to 100, got {}",
                total_percentage
            )));
        }

        // Check for duplicate variant IDs
        let mut seen_ids = HashSet::new();

        for variant in &request.variants {
            if !seen_ids.insert(&variant.id) {
                return Err(DomainError::validation(format!(
                    "Duplicate variant ID: '{}'",
                    variant.id
                )));
            }
        }

        // Verify all allocated variants exist
        let variant_ids: HashSet<_> = request.variants.iter().map(|v| &v.id).collect();

        for (variant_id, _) in &request.traffic_allocation {
            if !variant_ids.contains(variant_id) {
                return Err(DomainError::validation(format!(
                    "Traffic allocated to unknown variant: '{}'",
                    variant_id
                )));
            }
        }

        Ok(())
    }

    fn validate_for_start(&self, experiment: &Experiment) -> Result<(), DomainError> {
        if experiment.variants().len() < 2 {
            return Err(DomainError::validation(
                "Experiment must have at least 2 variants to start",
            ));
        }

        let total_percentage: u8 = experiment
            .traffic_allocation()
            .iter()
            .map(|a| a.percentage())
            .sum();

        if total_percentage != 100 {
            return Err(DomainError::validation(format!(
                "Traffic allocations must sum to 100 before starting, got {}",
                total_percentage
            )));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::experiment::{MockExperimentRecordRepository, MockExperimentRepository};

    fn create_service(
    ) -> ExperimentService<MockExperimentRepository, MockExperimentRecordRepository> {
        let repo = Arc::new(MockExperimentRepository::new());
        let record_repo = Arc::new(MockExperimentRecordRepository::new());
        ExperimentService::new(repo, record_repo)
    }

    fn create_valid_request(id: &str) -> CreateExperimentRequest {
        CreateExperimentRequest {
            id: id.to_string(),
            name: format!("Experiment {}", id),
            description: Some("Test experiment".to_string()),
            variants: vec![
                CreateVariantRequest {
                    id: "control".to_string(),
                    name: "Control".to_string(),
                    description: None,
                    config: VariantConfig::model_reference("gpt-4"),
                    control: true,
                },
                CreateVariantRequest {
                    id: "treatment".to_string(),
                    name: "Treatment".to_string(),
                    description: None,
                    config: VariantConfig::model_reference("gpt-4-turbo"),
                    control: false,
                },
            ],
            traffic_allocation: vec![
                ("control".to_string(), 50),
                ("treatment".to_string(), 50),
            ],
            enabled: true,
        }
    }

    #[tokio::test]
    async fn test_create_experiment() {
        let service = create_service();
        let request = create_valid_request("test-exp");

        let created = service.create(request).await.unwrap();

        assert_eq!(created.id().as_str(), "test-exp");
        assert_eq!(created.variants().len(), 2);
        assert_eq!(created.status(), ExperimentStatus::Draft);
    }

    #[tokio::test]
    async fn test_create_duplicate() {
        let service = create_service();
        let request = create_valid_request("test-exp");

        service.create(request.clone()).await.unwrap();
        let result = service.create(request).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }

    #[tokio::test]
    async fn test_create_invalid_traffic_sum() {
        let service = create_service();
        let mut request = create_valid_request("test-exp");
        request.traffic_allocation = vec![
            ("control".to_string(), 30),
            ("treatment".to_string(), 30),
        ];

        let result = service.create(request).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("sum to 100"));
    }

    #[tokio::test]
    async fn test_create_insufficient_variants() {
        let service = create_service();
        let mut request = create_valid_request("test-exp");
        request.variants = vec![request.variants[0].clone()];
        request.traffic_allocation = vec![("control".to_string(), 100)];

        let result = service.create(request).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("at least 2"));
    }

    #[tokio::test]
    async fn test_get_experiment() {
        let service = create_service();
        let request = create_valid_request("test-exp");
        service.create(request).await.unwrap();

        let fetched = service.get("test-exp").await.unwrap();
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().name(), "Experiment test-exp");
    }

    #[tokio::test]
    async fn test_update_experiment() {
        let service = create_service();
        let request = create_valid_request("test-exp");
        service.create(request).await.unwrap();

        let update = UpdateExperimentRequest {
            name: Some("Updated Name".to_string()),
            ..Default::default()
        };

        let updated = service.update("test-exp", update).await.unwrap();
        assert_eq!(updated.name(), "Updated Name");
    }

    #[tokio::test]
    async fn test_update_active_experiment_fails() {
        let service = create_service();
        let request = create_valid_request("test-exp");
        service.create(request).await.unwrap();
        service.start("test-exp").await.unwrap();

        let update = UpdateExperimentRequest {
            name: Some("Updated Name".to_string()),
            ..Default::default()
        };

        let result = service.update("test-exp", update).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Draft status"));
    }

    #[tokio::test]
    async fn test_start_experiment() {
        let service = create_service();
        let request = create_valid_request("test-exp");
        service.create(request).await.unwrap();

        let started = service.start("test-exp").await.unwrap();

        assert_eq!(started.status(), ExperimentStatus::Active);
        assert!(started.started_at().is_some());
    }

    #[tokio::test]
    async fn test_pause_experiment() {
        let service = create_service();
        let request = create_valid_request("test-exp");
        service.create(request).await.unwrap();
        service.start("test-exp").await.unwrap();

        let paused = service.pause("test-exp").await.unwrap();
        assert_eq!(paused.status(), ExperimentStatus::Paused);
    }

    #[tokio::test]
    async fn test_resume_experiment() {
        let service = create_service();
        let request = create_valid_request("test-exp");
        service.create(request).await.unwrap();
        service.start("test-exp").await.unwrap();
        service.pause("test-exp").await.unwrap();

        let resumed = service.resume("test-exp").await.unwrap();
        assert_eq!(resumed.status(), ExperimentStatus::Active);
    }

    #[tokio::test]
    async fn test_complete_experiment() {
        let service = create_service();
        let request = create_valid_request("test-exp");
        service.create(request).await.unwrap();
        service.start("test-exp").await.unwrap();

        let completed = service.complete("test-exp").await.unwrap();

        assert_eq!(completed.status(), ExperimentStatus::Completed);
        assert!(completed.completed_at().is_some());
    }

    #[tokio::test]
    async fn test_delete_experiment() {
        let service = create_service();
        let request = create_valid_request("test-exp");
        service.create(request).await.unwrap();

        let deleted = service.delete("test-exp").await.unwrap();
        assert!(deleted);

        let fetched = service.get("test-exp").await.unwrap();
        assert!(fetched.is_none());
    }

    #[tokio::test]
    async fn test_delete_active_experiment_fails() {
        let service = create_service();
        let request = create_valid_request("test-exp");
        service.create(request).await.unwrap();
        service.start("test-exp").await.unwrap();

        let result = service.delete("test-exp").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("active"));
    }

    #[tokio::test]
    async fn test_assign_variant() {
        let service = create_service();
        let request = create_valid_request("test-exp");
        service.create(request).await.unwrap();
        service.start("test-exp").await.unwrap();

        let assignment = service.assign_variant("gpt-4", "api-key-1").await.unwrap();

        assert!(assignment.is_some());
        let result = assignment.unwrap();
        assert_eq!(result.experiment_id, "test-exp");
        assert!(result.variant_id == "control" || result.variant_id == "treatment");
    }

    #[tokio::test]
    async fn test_assign_variant_consistent() {
        let service = create_service();
        let request = create_valid_request("test-exp");
        service.create(request).await.unwrap();
        service.start("test-exp").await.unwrap();

        let assignment1 = service
            .assign_variant("gpt-4", "api-key-1")
            .await
            .unwrap()
            .unwrap();
        let assignment2 = service
            .assign_variant("gpt-4", "api-key-1")
            .await
            .unwrap()
            .unwrap();

        assert_eq!(assignment1.variant_id, assignment2.variant_id);
    }

    #[tokio::test]
    async fn test_assign_variant_no_active_experiment() {
        let service = create_service();
        let request = create_valid_request("test-exp");
        service.create(request).await.unwrap();

        // Not started, so no assignment
        let assignment = service.assign_variant("gpt-4", "api-key-1").await.unwrap();
        assert!(assignment.is_none());
    }

    #[tokio::test]
    async fn test_record_experiment() {
        let service = create_service();
        let request = create_valid_request("test-exp");
        service.create(request).await.unwrap();
        service.start("test-exp").await.unwrap();

        let params = RecordExperimentParams {
            experiment_id: "test-exp".to_string(),
            variant_id: "control".to_string(),
            api_key_id: "api-key-1".to_string(),
            model_id: "gpt-4".to_string(),
            input_tokens: 100,
            output_tokens: 50,
            cost_micros: 1500,
            latency_ms: 200,
            success: true,
            error: None,
        };

        service.record(params).await.unwrap();

        // Verify through results
        let results = service.get_results("test-exp").await.unwrap();
        assert_eq!(results.total_requests, 1);
    }

    #[tokio::test]
    async fn test_get_results() {
        let service = create_service();
        let request = create_valid_request("test-exp");
        service.create(request).await.unwrap();
        service.start("test-exp").await.unwrap();

        // Record some results
        for i in 0..10 {
            let variant_id = if i < 5 { "control" } else { "treatment" };
            let latency = if i < 5 { 200 } else { 150 }; // Treatment is faster

            let params = RecordExperimentParams {
                experiment_id: "test-exp".to_string(),
                variant_id: variant_id.to_string(),
                api_key_id: "api-key-1".to_string(),
                model_id: "gpt-4".to_string(),
                input_tokens: 100,
                output_tokens: 50,
                cost_micros: 1500,
                latency_ms: latency,
                success: true,
                error: None,
            };

            service.record(params).await.unwrap();
        }

        let results = service.get_results("test-exp").await.unwrap();

        assert_eq!(results.experiment_id, "test-exp");
        assert_eq!(results.total_requests, 10);
        assert_eq!(results.variant_metrics.len(), 2);

        // Check metrics
        let control_metrics = results.get_variant_metrics("control").unwrap();
        assert_eq!(control_metrics.total_requests, 5);

        let treatment_metrics = results.get_variant_metrics("treatment").unwrap();
        assert_eq!(treatment_metrics.total_requests, 5);
    }
}
