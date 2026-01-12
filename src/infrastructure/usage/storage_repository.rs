//! Storage-backed usage and budget repository implementations

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

use crate::domain::storage::Storage;
use crate::domain::usage::{
    Budget, BudgetId, BudgetPeriod, BudgetRepository, DailyUsage, UsageAggregate, UsageQuery,
    UsageRecord, UsageRecordId, UsageRepository, UsageSummary,
};
use crate::domain::DomainError;

/// Storage-backed implementation of UsageRepository
#[derive(Debug)]
pub struct StorageUsageRepository {
    storage: Arc<dyn Storage<UsageRecord>>,
}

impl StorageUsageRepository {
    /// Create a new storage-backed repository
    pub fn new(storage: Arc<dyn Storage<UsageRecord>>) -> Self {
        Self { storage }
    }

    fn filter_records<'a>(
        &self,
        records: impl Iterator<Item = &'a UsageRecord>,
        query: &UsageQuery,
    ) -> Vec<&'a UsageRecord> {
        records
            .filter(|r| {
                if let Some(ref api_key) = query.api_key_id {
                    if &r.api_key_id != api_key {
                        return false;
                    }
                }

                if let Some(ref model) = query.model_id {
                    if r.model_id.as_ref() != Some(model) {
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
            .collect()
    }
}

#[async_trait]
impl UsageRepository for StorageUsageRepository {
    async fn record(&self, record: UsageRecord) -> Result<(), DomainError> {
        self.storage.create(record).await?;
        Ok(())
    }

    async fn get(&self, id: &UsageRecordId) -> Result<Option<UsageRecord>, DomainError> {
        self.storage.get(id).await
    }

    async fn query(&self, query: &UsageQuery) -> Result<Vec<UsageRecord>, DomainError> {
        let all = self.storage.list().await?;
        let mut filtered: Vec<UsageRecord> = self
            .filter_records(all.iter(), query)
            .into_iter()
            .cloned()
            .collect();

        // Sort by timestamp descending
        filtered.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        // Apply pagination
        let offset = query.offset.unwrap_or(0);

        if offset < filtered.len() {
            filtered = filtered.into_iter().skip(offset).collect();
        } else {
            filtered.clear();
        }

        if let Some(limit) = query.limit {
            filtered.truncate(limit);
        }

        Ok(filtered)
    }

    async fn count(&self, query: &UsageQuery) -> Result<usize, DomainError> {
        let all = self.storage.list().await?;
        Ok(self.filter_records(all.iter(), query).len())
    }

    async fn aggregate(&self, query: &UsageQuery) -> Result<UsageAggregate, DomainError> {
        let all = self.storage.list().await?;
        let filtered = self.filter_records(all.iter(), query);

        let mut agg = UsageAggregate::default();

        for record in filtered {
            agg.add_record(record);
        }

        Ok(agg)
    }

    async fn summary(&self, query: &UsageQuery) -> Result<UsageSummary, DomainError> {
        let all = self.storage.list().await?;
        let filtered = self.filter_records(all.iter(), query);

        // Group by day (using day timestamp as key)
        let mut daily_map: HashMap<u64, Vec<&UsageRecord>> = HashMap::new();

        for record in &filtered {
            let day = timestamp_to_day_start(record.timestamp);
            daily_map.entry(day).or_default().push(record);
        }

        // Calculate aggregate
        let aggregate = self.aggregate(query).await?;

        // Calculate daily summaries
        let mut daily: Vec<DailyUsage> = daily_map
            .into_iter()
            .map(|(date, records)| {
                let mut day = DailyUsage {
                    date,
                    requests: records.len() as u64,
                    tokens: 0,
                    cost_micros: 0,
                };

                for r in records {
                    day.tokens += r.total_tokens as u64;
                    day.cost_micros += r.cost_micros;
                }

                day
            })
            .collect();

        daily.sort_by(|a, b| a.date.cmp(&b.date));

        // Get period bounds
        let period_start = query.from_timestamp.unwrap_or_else(|| {
            filtered
                .iter()
                .map(|r| r.timestamp)
                .min()
                .unwrap_or_default()
        });

        let period_end = query.to_timestamp.unwrap_or_else(|| {
            filtered
                .iter()
                .map(|r| r.timestamp)
                .max()
                .unwrap_or_default()
        });

        Ok(UsageSummary {
            period_start,
            period_end,
            aggregate,
            daily,
        })
    }

    async fn delete_before(&self, timestamp: u64) -> Result<usize, DomainError> {
        let all = self.storage.list().await?;
        let mut deleted = 0;

        for record in all {
            if record.timestamp < timestamp {
                if self.storage.delete(record.id()).await? {
                    deleted += 1;
                }
            }
        }

        Ok(deleted)
    }

    async fn delete_by_api_key(&self, api_key_id: &str) -> Result<usize, DomainError> {
        let all = self.storage.list().await?;
        let mut deleted = 0;

        for record in all {
            if record.api_key_id == api_key_id {
                if self.storage.delete(record.id()).await? {
                    deleted += 1;
                }
            }
        }

        Ok(deleted)
    }
}

fn timestamp_to_day_start(timestamp: u64) -> u64 {
    // Convert to start of day (midnight UTC)
    const SECONDS_PER_DAY: u64 = 86400;
    (timestamp / SECONDS_PER_DAY) * SECONDS_PER_DAY
}

/// Storage-backed implementation of BudgetRepository
#[derive(Debug)]
pub struct StorageBudgetRepository {
    storage: Arc<dyn Storage<Budget>>,
}

impl StorageBudgetRepository {
    /// Create a new storage-backed repository
    pub fn new(storage: Arc<dyn Storage<Budget>>) -> Self {
        Self { storage }
    }
}

#[async_trait]
impl BudgetRepository for StorageBudgetRepository {
    async fn create(&self, budget: Budget) -> Result<Budget, DomainError> {
        if self.storage.exists(budget.id()).await? {
            return Err(DomainError::conflict(format!(
                "Budget '{}' already exists",
                budget.id()
            )));
        }

        self.storage.create(budget).await
    }

    async fn get(&self, id: &BudgetId) -> Result<Option<Budget>, DomainError> {
        self.storage.get(id).await
    }

    async fn update(&self, budget: Budget) -> Result<Budget, DomainError> {
        if !self.storage.exists(budget.id()).await? {
            return Err(DomainError::not_found(format!(
                "Budget '{}' not found",
                budget.id()
            )));
        }

        self.storage.update(budget).await
    }

    async fn delete(&self, id: &BudgetId) -> Result<bool, DomainError> {
        self.storage.delete(id).await
    }

    async fn list(&self) -> Result<Vec<Budget>, DomainError> {
        self.storage.list().await
    }

    async fn find_applicable(
        &self,
        api_key_id: &str,
        model_id: Option<&str>,
    ) -> Result<Vec<Budget>, DomainError> {
        let all = self.storage.list().await?;
        Ok(all
            .into_iter()
            .filter(|b| {
                b.enabled
                    && b.applies_to_api_key(api_key_id)
                    && model_id.map_or(true, |m| b.applies_to_model(m))
            })
            .collect())
    }

    async fn find_applicable_with_team(
        &self,
        api_key_id: &str,
        team_id: Option<&str>,
        model_id: Option<&str>,
    ) -> Result<Vec<Budget>, DomainError> {
        let all = self.storage.list().await?;
        Ok(all
            .into_iter()
            .filter(|b| {
                b.enabled
                    && b.applies_to_api_key_with_team(api_key_id, team_id)
                    && model_id.map_or(true, |m| b.applies_to_model(m))
            })
            .collect())
    }

    async fn find_by_team(&self, team_id: &str) -> Result<Vec<Budget>, DomainError> {
        let all = self.storage.list().await?;
        Ok(all
            .into_iter()
            .filter(|b| b.applies_to_team(team_id))
            .collect())
    }

    async fn get_expired_periods(&self, now: u64) -> Result<Vec<Budget>, DomainError> {
        let all = self.storage.list().await?;
        Ok(all
            .into_iter()
            .filter(|b| {
                let period_duration = match b.period {
                    BudgetPeriod::Daily => 86400,
                    BudgetPeriod::Weekly => 604800,
                    BudgetPeriod::Monthly => 2592000, // ~30 days
                    BudgetPeriod::Lifetime => return false,
                };

                b.enabled && now >= b.period_start + period_duration
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::usage::{BudgetPeriod, UsageType};
    use crate::infrastructure::storage::InMemoryStorage;

    fn create_usage_repo() -> StorageUsageRepository {
        let storage = Arc::new(InMemoryStorage::<UsageRecord>::new());
        StorageUsageRepository::new(storage)
    }

    fn create_budget_repo() -> StorageBudgetRepository {
        let storage = Arc::new(InMemoryStorage::<Budget>::new());
        StorageBudgetRepository::new(storage)
    }

    #[tokio::test]
    async fn test_usage_record_and_query() {
        let repo = create_usage_repo();

        let record = UsageRecord::new("rec-1", UsageType::ChatCompletion, "api-key-1")
            .with_model_id("gpt-4")
            .with_tokens(100, 50);

        repo.record(record).await.unwrap();

        let results = repo.query(&UsageQuery::new()).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].input_tokens, 100);
    }

    #[tokio::test]
    async fn test_usage_aggregate() {
        let repo = create_usage_repo();

        for i in 0..5 {
            let record =
                UsageRecord::new(format!("rec-{}", i), UsageType::ChatCompletion, "api-key-1")
                    .with_model_id("gpt-4")
                    .with_tokens(100, 50)
                    .with_cost_micros(1000);

            repo.record(record).await.unwrap();
        }

        let agg = repo.aggregate(&UsageQuery::new()).await.unwrap();
        assert_eq!(agg.total_requests, 5);
        assert_eq!(agg.total_cost_micros, 5000);
    }

    #[tokio::test]
    async fn test_budget_crud() {
        let repo = create_budget_repo();

        let budget = Budget::new("budget-1", "Test Budget", BudgetPeriod::Monthly)
            .with_hard_limit(100.0);

        // Create
        repo.create(budget).await.unwrap();

        // Get
        let found = repo.get(&BudgetId::from("budget-1")).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "Test Budget");

        // Update
        let mut updated = repo.get(&BudgetId::from("budget-1")).await.unwrap().unwrap();
        updated.name = "Updated Budget".to_string();
        repo.update(updated).await.unwrap();

        let found = repo.get(&BudgetId::from("budget-1")).await.unwrap();
        assert_eq!(found.unwrap().name, "Updated Budget");

        // Delete
        let deleted = repo.delete(&BudgetId::from("budget-1")).await.unwrap();
        assert!(deleted);
        assert!(repo.get(&BudgetId::from("budget-1")).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_budget_find_applicable() {
        let repo = create_budget_repo();

        let budget1 = Budget::new("budget-1", "Key 1 Budget", BudgetPeriod::Monthly)
            .with_hard_limit(100.0)
            .with_api_key("key-1".to_string());

        let budget2 = Budget::new("budget-2", "Global Budget", BudgetPeriod::Monthly)
            .with_hard_limit(100.0);

        repo.create(budget1).await.unwrap();
        repo.create(budget2).await.unwrap();

        // key-1 should match both
        let applicable = repo.find_applicable("key-1", None).await.unwrap();
        assert_eq!(applicable.len(), 2);

        // key-2 should only match global
        let applicable = repo.find_applicable("key-2", None).await.unwrap();
        assert_eq!(applicable.len(), 1);
    }
}
