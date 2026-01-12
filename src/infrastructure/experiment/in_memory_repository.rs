//! In-memory implementation of the experiment repository

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::RwLock;

use crate::domain::experiment::{
    Experiment, ExperimentId, ExperimentQuery, ExperimentRepository, ExperimentStatus,
};
use crate::domain::DomainError;

/// In-memory experiment repository implementation
#[derive(Debug)]
pub struct InMemoryExperimentRepository {
    experiments: RwLock<HashMap<String, Experiment>>,
}

impl InMemoryExperimentRepository {
    /// Create a new empty repository
    pub fn new() -> Self {
        Self {
            experiments: RwLock::new(HashMap::new()),
        }
    }

    /// Create a repository with initial experiments
    pub fn with_experiments(experiments: Vec<Experiment>) -> Self {
        let repo = Self::new();
        {
            let mut map = repo.experiments.write().unwrap();

            for experiment in experiments {
                map.insert(experiment.id().as_str().to_string(), experiment);
            }
        }
        repo
    }
}

impl Default for InMemoryExperimentRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ExperimentRepository for InMemoryExperimentRepository {
    async fn create(&self, experiment: Experiment) -> Result<Experiment, DomainError> {
        let id = experiment.id().as_str().to_string();
        let mut experiments = self
            .experiments
            .write()
            .map_err(|e| DomainError::internal(format!("Failed to acquire write lock: {}", e)))?;

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
        let experiments = self
            .experiments
            .read()
            .map_err(|e| DomainError::internal(format!("Failed to acquire read lock: {}", e)))?;

        Ok(experiments.get(id.as_str()).cloned())
    }

    async fn update(&self, experiment: Experiment) -> Result<Experiment, DomainError> {
        let id = experiment.id().as_str().to_string();
        let mut experiments = self
            .experiments
            .write()
            .map_err(|e| DomainError::internal(format!("Failed to acquire write lock: {}", e)))?;

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
        let mut experiments = self
            .experiments
            .write()
            .map_err(|e| DomainError::internal(format!("Failed to acquire write lock: {}", e)))?;

        Ok(experiments.remove(id.as_str()).is_some())
    }

    async fn list(&self, query: &ExperimentQuery) -> Result<Vec<Experiment>, DomainError> {
        let experiments = self
            .experiments
            .read()
            .map_err(|e| DomainError::internal(format!("Failed to acquire read lock: {}", e)))?;

        let mut results: Vec<_> = experiments
            .values()
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
            .cloned()
            .collect();

        // Sort by created_at descending
        results.sort_by(|a, b| b.created_at().cmp(&a.created_at()));

        // Apply pagination
        let offset = query.offset.unwrap_or(0);
        let limit = query.limit.unwrap_or(usize::MAX);

        Ok(results.into_iter().skip(offset).take(limit).collect())
    }

    async fn find_active_for_model(&self, model_id: &str) -> Result<Vec<Experiment>, DomainError> {
        let experiments = self
            .experiments
            .read()
            .map_err(|e| DomainError::internal(format!("Failed to acquire read lock: {}", e)))?;

        let results: Vec<_> = experiments
            .values()
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
            .cloned()
            .collect();

        Ok(results)
    }

    async fn exists(&self, id: &ExperimentId) -> Result<bool, DomainError> {
        let experiments = self
            .experiments
            .read()
            .map_err(|e| DomainError::internal(format!("Failed to acquire read lock: {}", e)))?;

        Ok(experiments.contains_key(id.as_str()))
    }

    async fn count(&self, query: &ExperimentQuery) -> Result<usize, DomainError> {
        Ok(self.list(query).await?.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::experiment::{
        TrafficAllocation, Variant, VariantConfig, VariantId,
    };

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
        let repo = InMemoryExperimentRepository::new();
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
        let repo = InMemoryExperimentRepository::new();
        let exp = create_test_experiment("test-1", "gpt-4");

        repo.create(exp.clone()).await.unwrap();
        let result = repo.create(exp).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }

    #[tokio::test]
    async fn test_update() {
        let repo = InMemoryExperimentRepository::new();
        let exp = create_test_experiment("test-1", "gpt-4");
        repo.create(exp).await.unwrap();

        let exp_id = ExperimentId::new("test-1").unwrap();
        let mut fetched = repo.get(&exp_id).await.unwrap().unwrap();
        fetched.set_name("Updated Name");

        let updated = repo.update(fetched).await.unwrap();
        assert_eq!(updated.name(), "Updated Name");
    }

    #[tokio::test]
    async fn test_update_not_found() {
        let repo = InMemoryExperimentRepository::new();
        let exp = create_test_experiment("test-1", "gpt-4");

        let result = repo.update(exp).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[tokio::test]
    async fn test_delete() {
        let repo = InMemoryExperimentRepository::new();
        let exp = create_test_experiment("test-1", "gpt-4");
        repo.create(exp).await.unwrap();

        let exp_id = ExperimentId::new("test-1").unwrap();

        let deleted = repo.delete(&exp_id).await.unwrap();
        assert!(deleted);

        let fetched = repo.get(&exp_id).await.unwrap();
        assert!(fetched.is_none());

        // Delete again should return false
        let deleted_again = repo.delete(&exp_id).await.unwrap();
        assert!(!deleted_again);
    }

    #[tokio::test]
    async fn test_list_all() {
        let repo = InMemoryExperimentRepository::new();

        for i in 1..=5 {
            let exp = create_test_experiment(&format!("exp-{}", i), "gpt-4");
            repo.create(exp).await.unwrap();
        }

        let all = repo.list(&ExperimentQuery::new()).await.unwrap();
        assert_eq!(all.len(), 5);
    }

    #[tokio::test]
    async fn test_list_with_pagination() {
        let repo = InMemoryExperimentRepository::new();

        for i in 1..=10 {
            let exp = create_test_experiment(&format!("exp-{}", i), "gpt-4");
            repo.create(exp).await.unwrap();
        }

        let page1 = repo
            .list(&ExperimentQuery::new().with_limit(3))
            .await
            .unwrap();
        assert_eq!(page1.len(), 3);

        let page2 = repo
            .list(&ExperimentQuery::new().with_offset(3).with_limit(3))
            .await
            .unwrap();
        assert_eq!(page2.len(), 3);
    }

    #[tokio::test]
    async fn test_list_by_status() {
        let repo = InMemoryExperimentRepository::new();

        // Create draft and active experiments
        let draft = create_test_experiment("draft-1", "gpt-4");
        repo.create(draft).await.unwrap();

        let mut active = create_test_experiment("active-1", "gpt-4");
        active.start().unwrap();
        repo.create(active).await.unwrap();

        // List only draft
        let drafts = repo
            .list(&ExperimentQuery::new().with_status(ExperimentStatus::Draft))
            .await
            .unwrap();
        assert_eq!(drafts.len(), 1);
        assert_eq!(drafts[0].id().as_str(), "draft-1");

        // List only active
        let actives = repo
            .list(&ExperimentQuery::new().with_status(ExperimentStatus::Active))
            .await
            .unwrap();
        assert_eq!(actives.len(), 1);
        assert_eq!(actives[0].id().as_str(), "active-1");
    }

    #[tokio::test]
    async fn test_list_by_model() {
        let repo = InMemoryExperimentRepository::new();

        let exp1 = create_test_experiment("exp-1", "gpt-4");
        let exp2 = create_test_experiment("exp-2", "claude-3");
        repo.create(exp1).await.unwrap();
        repo.create(exp2).await.unwrap();

        let gpt_experiments = repo
            .list(&ExperimentQuery::new().with_model("gpt-4"))
            .await
            .unwrap();
        assert_eq!(gpt_experiments.len(), 1);
        assert_eq!(gpt_experiments[0].id().as_str(), "exp-1");
    }

    #[tokio::test]
    async fn test_find_active_for_model() {
        let repo = InMemoryExperimentRepository::new();

        // Draft experiment
        let draft = create_test_experiment("draft-1", "gpt-4");
        repo.create(draft).await.unwrap();

        // Active experiment
        let mut active = create_test_experiment("active-1", "gpt-4");
        active.start().unwrap();
        repo.create(active).await.unwrap();

        // Active but different model
        let mut other = create_test_experiment("active-2", "claude-3");
        other.start().unwrap();
        repo.create(other).await.unwrap();

        let results = repo.find_active_for_model("gpt-4").await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id().as_str(), "active-1");
    }

    #[tokio::test]
    async fn test_exists() {
        let repo = InMemoryExperimentRepository::new();
        let exp = create_test_experiment("test-1", "gpt-4");
        repo.create(exp).await.unwrap();

        let exp_id = ExperimentId::new("test-1").unwrap();
        let nonexistent = ExperimentId::new("nonexistent").unwrap();

        assert!(repo.exists(&exp_id).await.unwrap());
        assert!(!repo.exists(&nonexistent).await.unwrap());
    }

    #[tokio::test]
    async fn test_count() {
        let repo = InMemoryExperimentRepository::new();

        for i in 1..=5 {
            let exp = create_test_experiment(&format!("exp-{}", i), "gpt-4");
            repo.create(exp).await.unwrap();
        }

        let count = repo.count(&ExperimentQuery::new()).await.unwrap();
        assert_eq!(count, 5);
    }

    #[tokio::test]
    async fn test_with_experiments() {
        let experiments = vec![
            create_test_experiment("exp-1", "gpt-4"),
            create_test_experiment("exp-2", "gpt-4"),
        ];

        let repo = InMemoryExperimentRepository::with_experiments(experiments);

        let all = repo.list(&ExperimentQuery::new()).await.unwrap();
        assert_eq!(all.len(), 2);
    }
}
