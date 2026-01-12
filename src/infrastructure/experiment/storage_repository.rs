//! Storage-backed experiment repository implementations

use async_trait::async_trait;
use std::sync::Arc;

use crate::domain::experiment::{
    Experiment, ExperimentId, ExperimentQuery, ExperimentRepository, ExperimentStatus,
};
use crate::domain::storage::Storage;
use crate::domain::DomainError;

/// Storage-backed implementation of ExperimentRepository
#[derive(Debug)]
pub struct StorageExperimentRepository {
    storage: Arc<dyn Storage<Experiment>>,
}

impl StorageExperimentRepository {
    /// Create a new storage-backed repository
    pub fn new(storage: Arc<dyn Storage<Experiment>>) -> Self {
        Self { storage }
    }
}

#[async_trait]
impl ExperimentRepository for StorageExperimentRepository {
    async fn create(&self, experiment: Experiment) -> Result<Experiment, DomainError> {
        if self.storage.exists(experiment.id()).await? {
            return Err(DomainError::conflict(format!(
                "Experiment '{}' already exists",
                experiment.id()
            )));
        }

        self.storage.create(experiment).await
    }

    async fn get(&self, id: &ExperimentId) -> Result<Option<Experiment>, DomainError> {
        self.storage.get(id).await
    }

    async fn update(&self, experiment: Experiment) -> Result<Experiment, DomainError> {
        if !self.storage.exists(experiment.id()).await? {
            return Err(DomainError::not_found(format!(
                "Experiment '{}' not found",
                experiment.id()
            )));
        }

        self.storage.update(experiment).await
    }

    async fn delete(&self, id: &ExperimentId) -> Result<bool, DomainError> {
        self.storage.delete(id).await
    }

    async fn list(&self, query: &ExperimentQuery) -> Result<Vec<Experiment>, DomainError> {
        let all = self.storage.list().await?;

        let mut results: Vec<_> = all
            .into_iter()
            .filter(|e| {
                // Filter by status
                if let Some(status) = query.status {
                    if e.status() != status {
                        return false;
                    }
                }

                // Filter by model ID (matches any variant)
                if let Some(ref model_id) = query.model_id {
                    let model_ids = e.referenced_model_ids();

                    if !model_ids.contains(&model_id.as_str()) {
                        return false;
                    }
                }

                true
            })
            .collect();

        // Sort by created_at descending
        results.sort_by(|a, b| b.created_at().cmp(&a.created_at()));

        // Apply pagination
        let offset = query.offset.unwrap_or(0);
        let limit = query.limit.unwrap_or(usize::MAX);

        Ok(results.into_iter().skip(offset).take(limit).collect())
    }

    async fn find_active_for_model(&self, model_id: &str) -> Result<Vec<Experiment>, DomainError> {
        let all = self.storage.list().await?;

        let results: Vec<_> = all
            .into_iter()
            .filter(|e| {
                // Must be active
                if e.status() != ExperimentStatus::Active {
                    return false;
                }

                // Must be enabled
                if !e.is_enabled() {
                    return false;
                }

                // Must reference the model
                e.referenced_model_ids().contains(&model_id)
            })
            .collect();

        Ok(results)
    }

    async fn exists(&self, id: &ExperimentId) -> Result<bool, DomainError> {
        self.storage.exists(id).await
    }

    async fn count(&self, query: &ExperimentQuery) -> Result<usize, DomainError> {
        Ok(self.list(query).await?.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::experiment::{TrafficAllocation, Variant, VariantConfig, VariantId};
    use crate::infrastructure::storage::InMemoryStorage;

    fn create_repo() -> StorageExperimentRepository {
        let storage = Arc::new(InMemoryStorage::<Experiment>::new());
        StorageExperimentRepository::new(storage)
    }

    fn create_test_experiment(id: &str, model_id: &str) -> Experiment {
        let exp_id = ExperimentId::new(id).unwrap();
        let control_id = VariantId::new("control").unwrap();
        let treatment_id = VariantId::new("treatment").unwrap();

        Experiment::new(exp_id, format!("Experiment {}", id))
            .with_variant(
                Variant::new(
                    control_id.clone(),
                    "Control",
                    VariantConfig::model_reference(model_id),
                )
                .with_control(true),
            )
            .with_variant(Variant::new(
                treatment_id.clone(),
                "Treatment",
                VariantConfig::model_reference(format!("{}-turbo", model_id)),
            ))
            .with_traffic_allocation(TrafficAllocation::new(control_id, 50))
            .with_traffic_allocation(TrafficAllocation::new(treatment_id, 50))
    }

    #[tokio::test]
    async fn test_create_and_get() {
        let repo = create_repo();
        let exp = create_test_experiment("test-1", "gpt-4");

        let created = repo.create(exp).await.unwrap();
        assert_eq!(created.id().as_str(), "test-1");

        let exp_id = ExperimentId::new("test-1").unwrap();
        let fetched = repo.get(&exp_id).await.unwrap();
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().name(), "Experiment test-1");
    }

    #[tokio::test]
    async fn test_create_duplicate() {
        let repo = create_repo();
        let exp = create_test_experiment("test-1", "gpt-4");

        repo.create(exp.clone()).await.unwrap();
        let result = repo.create(exp).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }

    #[tokio::test]
    async fn test_update() {
        let repo = create_repo();
        let exp = create_test_experiment("test-1", "gpt-4");
        repo.create(exp).await.unwrap();

        let exp_id = ExperimentId::new("test-1").unwrap();
        let mut fetched = repo.get(&exp_id).await.unwrap().unwrap();
        fetched.set_name("Updated Name");

        let updated = repo.update(fetched).await.unwrap();
        assert_eq!(updated.name(), "Updated Name");
    }

    #[tokio::test]
    async fn test_delete() {
        let repo = create_repo();
        let exp = create_test_experiment("test-1", "gpt-4");
        repo.create(exp).await.unwrap();

        let exp_id = ExperimentId::new("test-1").unwrap();

        let deleted = repo.delete(&exp_id).await.unwrap();
        assert!(deleted);

        let fetched = repo.get(&exp_id).await.unwrap();
        assert!(fetched.is_none());
    }

    #[tokio::test]
    async fn test_list_all() {
        let repo = create_repo();

        for i in 1..=5 {
            let exp = create_test_experiment(&format!("exp-{}", i), "gpt-4");
            repo.create(exp).await.unwrap();
        }

        let all = repo.list(&ExperimentQuery::new()).await.unwrap();
        assert_eq!(all.len(), 5);
    }

    #[tokio::test]
    async fn test_list_by_status() {
        let repo = create_repo();

        let draft = create_test_experiment("draft-1", "gpt-4");
        repo.create(draft).await.unwrap();

        let mut active = create_test_experiment("active-1", "gpt-4");
        active.start().unwrap();
        repo.create(active).await.unwrap();

        let drafts = repo
            .list(&ExperimentQuery::new().with_status(ExperimentStatus::Draft))
            .await
            .unwrap();
        assert_eq!(drafts.len(), 1);
        assert_eq!(drafts[0].id().as_str(), "draft-1");
    }

    #[tokio::test]
    async fn test_find_active_for_model() {
        let repo = create_repo();

        let draft = create_test_experiment("draft-1", "gpt-4");
        repo.create(draft).await.unwrap();

        let mut active = create_test_experiment("active-1", "gpt-4");
        active.start().unwrap();
        repo.create(active).await.unwrap();

        let results = repo.find_active_for_model("gpt-4").await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id().as_str(), "active-1");
    }
}
