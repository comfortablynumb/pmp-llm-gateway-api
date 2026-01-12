//! In-memory implementations of usage and budget repositories

use std::collections::HashMap;
use std::sync::RwLock;

use async_trait::async_trait;

use crate::domain::usage::{
    Budget, BudgetId, BudgetPeriod, BudgetRepository, UsageAggregate, UsageQuery, UsageRecord,
    UsageRecordId, UsageRepository, UsageSummary,
};
use crate::domain::usage::DailyUsage;
use crate::domain::DomainError;

/// In-memory usage repository
#[derive(Debug)]
pub struct InMemoryUsageRepository {
    records: RwLock<HashMap<UsageRecordId, UsageRecord>>,
    max_records: usize,
}

impl InMemoryUsageRepository {
    /// Create a new in-memory usage repository
    pub fn new(max_records: usize) -> Self {
        Self {
            records: RwLock::new(HashMap::new()),
            max_records,
        }
    }

    /// Evict oldest records if over limit
    fn evict_if_needed(&self, records: &mut HashMap<UsageRecordId, UsageRecord>) {
        if records.len() <= self.max_records {
            return;
        }

        // Find and remove oldest records
        let mut entries: Vec<_> = records.iter().map(|(k, v)| (k.clone(), v.timestamp)).collect();
        entries.sort_by(|a, b| a.1.cmp(&b.1));

        let to_remove = records.len() - self.max_records;
        for (id, _) in entries.into_iter().take(to_remove) {
            records.remove(&id);
        }
    }
}

impl Default for InMemoryUsageRepository {
    fn default() -> Self {
        Self::new(100000)
    }
}

#[async_trait]
impl UsageRepository for InMemoryUsageRepository {
    async fn record(&self, record: UsageRecord) -> Result<(), DomainError> {
        let mut records = self.records.write().map_err(|e| {
            DomainError::internal(format!("Failed to acquire write lock: {}", e))
        })?;

        records.insert(record.id().clone(), record);
        self.evict_if_needed(&mut records);

        Ok(())
    }

    async fn get(&self, id: &UsageRecordId) -> Result<Option<UsageRecord>, DomainError> {
        let records = self.records.read().map_err(|e| {
            DomainError::internal(format!("Failed to acquire read lock: {}", e))
        })?;

        Ok(records.get(id).cloned())
    }

    async fn query(&self, query: &UsageQuery) -> Result<Vec<UsageRecord>, DomainError> {
        let records = self.records.read().map_err(|e| {
            DomainError::internal(format!("Failed to acquire read lock: {}", e))
        })?;

        let mut results: Vec<_> = records
            .values()
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
            .cloned()
            .collect();

        // Sort by timestamp descending
        results.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        // Apply pagination
        let offset = query.offset.unwrap_or(0);
        let limit = query.limit.unwrap_or(usize::MAX);

        Ok(results.into_iter().skip(offset).take(limit).collect())
    }

    async fn count(&self, query: &UsageQuery) -> Result<usize, DomainError> {
        let results = self.query(query).await?;
        Ok(results.len())
    }

    async fn aggregate(&self, query: &UsageQuery) -> Result<UsageAggregate, DomainError> {
        let records = self.query(query).await?;
        let mut aggregate = UsageAggregate::new();

        for record in &records {
            aggregate.add_record(record);
        }

        Ok(aggregate)
    }

    async fn summary(&self, query: &UsageQuery) -> Result<UsageSummary, DomainError> {
        let records = self.query(query).await?;
        let mut aggregate = UsageAggregate::new();

        // Build daily breakdown
        let mut daily_map: HashMap<u64, (u64, u64, i64)> = HashMap::new();

        for record in &records {
            aggregate.add_record(record);

            // Group by day (start of day timestamp)
            let day_start = (record.timestamp / 86400) * 86400;
            let entry = daily_map.entry(day_start).or_insert((0, 0, 0));
            entry.0 += 1;
            entry.1 += record.total_tokens as u64;
            entry.2 += record.cost_micros;
        }

        let mut daily: Vec<DailyUsage> = daily_map
            .into_iter()
            .map(|(date, (requests, tokens, cost_micros))| DailyUsage {
                date,
                requests,
                tokens,
                cost_micros,
            })
            .collect();

        daily.sort_by(|a, b| a.date.cmp(&b.date));

        Ok(UsageSummary {
            period_start: query.from_timestamp.unwrap_or(0),
            period_end: query.to_timestamp.unwrap_or(u64::MAX),
            aggregate,
            daily,
        })
    }

    async fn delete_before(&self, timestamp: u64) -> Result<usize, DomainError> {
        let mut records = self.records.write().map_err(|e| {
            DomainError::internal(format!("Failed to acquire write lock: {}", e))
        })?;

        let before_count = records.len();
        records.retain(|_, r| r.timestamp >= timestamp);

        Ok(before_count - records.len())
    }

    async fn delete_by_api_key(&self, api_key_id: &str) -> Result<usize, DomainError> {
        let mut records = self.records.write().map_err(|e| {
            DomainError::internal(format!("Failed to acquire write lock: {}", e))
        })?;

        let before_count = records.len();
        records.retain(|_, r| r.api_key_id != api_key_id);

        Ok(before_count - records.len())
    }
}

/// In-memory budget repository
#[derive(Debug, Default)]
pub struct InMemoryBudgetRepository {
    budgets: RwLock<HashMap<BudgetId, Budget>>,
}

impl InMemoryBudgetRepository {
    /// Create a new in-memory budget repository
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl BudgetRepository for InMemoryBudgetRepository {
    async fn create(&self, budget: Budget) -> Result<Budget, DomainError> {
        let mut budgets = self.budgets.write().map_err(|e| {
            DomainError::internal(format!("Failed to acquire write lock: {}", e))
        })?;

        if budgets.contains_key(budget.id()) {
            return Err(DomainError::conflict(format!("Budget '{}' already exists", budget.id())));
        }

        budgets.insert(budget.id().clone(), budget.clone());
        Ok(budget)
    }

    async fn get(&self, id: &BudgetId) -> Result<Option<Budget>, DomainError> {
        let budgets = self.budgets.read().map_err(|e| {
            DomainError::internal(format!("Failed to acquire read lock: {}", e))
        })?;

        Ok(budgets.get(id).cloned())
    }

    async fn update(&self, budget: Budget) -> Result<Budget, DomainError> {
        let mut budgets = self.budgets.write().map_err(|e| {
            DomainError::internal(format!("Failed to acquire write lock: {}", e))
        })?;

        if !budgets.contains_key(budget.id()) {
            return Err(DomainError::not_found(format!("Budget '{}' not found", budget.id())));
        }

        budgets.insert(budget.id().clone(), budget.clone());
        Ok(budget)
    }

    async fn delete(&self, id: &BudgetId) -> Result<bool, DomainError> {
        let mut budgets = self.budgets.write().map_err(|e| {
            DomainError::internal(format!("Failed to acquire write lock: {}", e))
        })?;

        Ok(budgets.remove(id).is_some())
    }

    async fn list(&self) -> Result<Vec<Budget>, DomainError> {
        let budgets = self.budgets.read().map_err(|e| {
            DomainError::internal(format!("Failed to acquire read lock: {}", e))
        })?;

        Ok(budgets.values().cloned().collect())
    }

    async fn find_applicable(
        &self,
        api_key_id: &str,
        model_id: Option<&str>,
    ) -> Result<Vec<Budget>, DomainError> {
        let budgets = self.budgets.read().map_err(|e| {
            DomainError::internal(format!("Failed to acquire read lock: {}", e))
        })?;

        Ok(budgets
            .values()
            .filter(|b| {
                b.enabled
                    && b.applies_to_api_key(api_key_id)
                    && model_id.map_or(true, |m| b.applies_to_model(m))
            })
            .cloned()
            .collect())
    }

    async fn find_applicable_with_team(
        &self,
        api_key_id: &str,
        team_id: Option<&str>,
        model_id: Option<&str>,
    ) -> Result<Vec<Budget>, DomainError> {
        let budgets = self.budgets.read().map_err(|e| {
            DomainError::internal(format!("Failed to acquire read lock: {}", e))
        })?;

        Ok(budgets
            .values()
            .filter(|b| {
                b.enabled
                    && b.applies_to_api_key_with_team(api_key_id, team_id)
                    && model_id.map_or(true, |m| b.applies_to_model(m))
            })
            .cloned()
            .collect())
    }

    async fn find_by_team(&self, team_id: &str) -> Result<Vec<Budget>, DomainError> {
        let budgets = self.budgets.read().map_err(|e| {
            DomainError::internal(format!("Failed to acquire read lock: {}", e))
        })?;

        Ok(budgets
            .values()
            .filter(|b| b.enabled && b.applies_to_team(team_id))
            .cloned()
            .collect())
    }

    async fn get_expired_periods(&self, now: u64) -> Result<Vec<Budget>, DomainError> {
        let budgets = self.budgets.read().map_err(|e| {
            DomainError::internal(format!("Failed to acquire read lock: {}", e))
        })?;

        Ok(budgets
            .values()
            .filter(|b| {
                let period_duration = match b.period {
                    BudgetPeriod::Daily => 86400,
                    BudgetPeriod::Weekly => 604800,
                    BudgetPeriod::Monthly => 2592000, // ~30 days
                    BudgetPeriod::Lifetime => return false,
                };

                b.enabled && now >= b.period_start + period_duration
            })
            .cloned()
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::usage::UsageType;

    #[tokio::test]
    async fn test_usage_repository_record_and_get() {
        let repo = InMemoryUsageRepository::new(100);

        let record = UsageRecord::new("rec-1", UsageType::ChatCompletion, "api-key-1")
            .with_model_id("gpt-4")
            .with_tokens(100, 50);

        repo.record(record).await.unwrap();

        let retrieved = repo.get(&UsageRecordId::from("rec-1")).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().input_tokens, 100);
    }

    #[tokio::test]
    async fn test_usage_repository_query() {
        let repo = InMemoryUsageRepository::new(100);

        for i in 0..10 {
            let record = UsageRecord::new(
                format!("rec-{}", i),
                UsageType::ChatCompletion,
                if i < 5 { "key-1" } else { "key-2" },
            );
            repo.record(record).await.unwrap();
        }

        let query = UsageQuery::new().with_api_key("key-1");
        let results = repo.query(&query).await.unwrap();

        assert_eq!(results.len(), 5);
    }

    #[tokio::test]
    async fn test_usage_repository_aggregate() {
        let repo = InMemoryUsageRepository::new(100);

        let record1 = UsageRecord::new("rec-1", UsageType::ChatCompletion, "api-key-1")
            .with_tokens(100, 50)
            .with_cost(0.01);

        let record2 = UsageRecord::new("rec-2", UsageType::ChatCompletion, "api-key-1")
            .with_tokens(200, 100)
            .with_cost(0.02);

        repo.record(record1).await.unwrap();
        repo.record(record2).await.unwrap();

        let query = UsageQuery::new().with_api_key("api-key-1");
        let aggregate = repo.aggregate(&query).await.unwrap();

        assert_eq!(aggregate.total_requests, 2);
        assert_eq!(aggregate.total_input_tokens, 300);
        assert!((aggregate.total_cost_usd() - 0.03).abs() < 0.001);
    }

    #[tokio::test]
    async fn test_usage_repository_eviction() {
        let repo = InMemoryUsageRepository::new(5);

        for i in 0..10 {
            let mut record = UsageRecord::new(
                format!("rec-{}", i),
                UsageType::ChatCompletion,
                "api-key-1",
            );
            record.timestamp = 1000 + i as u64;
            repo.record(record).await.unwrap();
        }

        let query = UsageQuery::new();
        let count = repo.count(&query).await.unwrap();

        assert_eq!(count, 5);

        // Should have kept the newest records
        assert!(repo.get(&UsageRecordId::from("rec-9")).await.unwrap().is_some());
        assert!(repo.get(&UsageRecordId::from("rec-0")).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_budget_repository_crud() {
        let repo = InMemoryBudgetRepository::new();

        let budget = Budget::new("budget-1", "Test Budget", BudgetPeriod::Monthly)
            .with_hard_limit(100.0);

        // Create
        repo.create(budget.clone()).await.unwrap();

        // Get
        let retrieved = repo.get(&BudgetId::from("budget-1")).await.unwrap();
        assert!(retrieved.is_some());

        // Update
        let mut updated = retrieved.unwrap();
        updated.name = "Updated Budget".to_string();
        repo.update(updated).await.unwrap();

        let retrieved = repo.get(&BudgetId::from("budget-1")).await.unwrap();
        assert_eq!(retrieved.unwrap().name, "Updated Budget");

        // Delete
        let deleted = repo.delete(&BudgetId::from("budget-1")).await.unwrap();
        assert!(deleted);
        assert!(repo.get(&BudgetId::from("budget-1")).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_budget_repository_find_applicable() {
        let repo = InMemoryBudgetRepository::new();

        let budget1 = Budget::new("budget-1", "Key 1 Budget", BudgetPeriod::Monthly)
            .with_hard_limit(100.0)
            .with_api_key("key-1");

        let budget2 = Budget::new("budget-2", "All Keys Budget", BudgetPeriod::Monthly)
            .with_hard_limit(200.0);

        repo.create(budget1).await.unwrap();
        repo.create(budget2).await.unwrap();

        // key-1 should match both budgets
        let applicable = repo.find_applicable("key-1", None).await.unwrap();
        assert_eq!(applicable.len(), 2);

        // key-2 should only match the global budget
        let applicable = repo.find_applicable("key-2", None).await.unwrap();
        assert_eq!(applicable.len(), 1);
        assert_eq!(applicable[0].id().as_str(), "budget-2");
    }

    #[tokio::test]
    async fn test_budget_repository_find_applicable_with_team() {
        let repo = InMemoryBudgetRepository::new();

        // Team-scoped budget
        let budget1 = Budget::new("budget-1", "Team 1 Budget", BudgetPeriod::Monthly)
            .with_hard_limit(100.0)
            .with_team("team-1");

        // API key scoped budget
        let budget2 = Budget::new("budget-2", "Key Budget", BudgetPeriod::Monthly)
            .with_hard_limit(200.0)
            .with_api_key("key-1");

        // Global budget
        let budget3 = Budget::new("budget-3", "Global Budget", BudgetPeriod::Monthly)
            .with_hard_limit(300.0);

        repo.create(budget1).await.unwrap();
        repo.create(budget2).await.unwrap();
        repo.create(budget3).await.unwrap();

        // key-1 with team-1 should match team budget, key budget, and global
        let applicable = repo
            .find_applicable_with_team("key-1", Some("team-1"), None)
            .await
            .unwrap();
        assert_eq!(applicable.len(), 3);

        // key-2 with team-1 should match team budget and global
        let applicable = repo
            .find_applicable_with_team("key-2", Some("team-1"), None)
            .await
            .unwrap();
        assert_eq!(applicable.len(), 2);

        // key-1 without team should match key budget and global
        let applicable = repo
            .find_applicable_with_team("key-1", None, None)
            .await
            .unwrap();
        assert_eq!(applicable.len(), 2);

        // key-2 with team-2 should only match global
        let applicable = repo
            .find_applicable_with_team("key-2", Some("team-2"), None)
            .await
            .unwrap();
        assert_eq!(applicable.len(), 1);
        assert_eq!(applicable[0].id().as_str(), "budget-3");
    }

    #[tokio::test]
    async fn test_budget_repository_find_by_team() {
        let repo = InMemoryBudgetRepository::new();

        let budget1 = Budget::new("budget-1", "Team 1 Budget", BudgetPeriod::Monthly)
            .with_hard_limit(100.0)
            .with_team("team-1");

        let budget2 = Budget::new("budget-2", "Team 1+2 Budget", BudgetPeriod::Monthly)
            .with_hard_limit(200.0)
            .with_team("team-1")
            .with_team("team-2");

        let budget3 = Budget::new("budget-3", "Global Budget", BudgetPeriod::Monthly)
            .with_hard_limit(300.0);

        repo.create(budget1).await.unwrap();
        repo.create(budget2).await.unwrap();
        repo.create(budget3).await.unwrap();

        // team-1 should match team-1 budget, team-1+2 budget, and global
        let team_budgets = repo.find_by_team("team-1").await.unwrap();
        assert_eq!(team_budgets.len(), 3);

        // team-2 should match team-1+2 budget and global
        let team_budgets = repo.find_by_team("team-2").await.unwrap();
        assert_eq!(team_budgets.len(), 2);

        // team-3 should only match global
        let team_budgets = repo.find_by_team("team-3").await.unwrap();
        assert_eq!(team_budgets.len(), 1);
        assert_eq!(team_budgets[0].id().as_str(), "budget-3");
    }

    #[tokio::test]
    async fn test_budget_repository_mixed_scope() {
        let repo = InMemoryBudgetRepository::new();

        // Mixed scope budget (applies to specific keys AND teams)
        let budget = Budget::new("budget-1", "Mixed Budget", BudgetPeriod::Monthly)
            .with_hard_limit(100.0)
            .with_api_key("key-1")
            .with_team("team-1");

        repo.create(budget).await.unwrap();

        // Should match key-1 regardless of team
        let applicable = repo
            .find_applicable_with_team("key-1", None, None)
            .await
            .unwrap();
        assert_eq!(applicable.len(), 1);

        // Should match any key with team-1
        let applicable = repo
            .find_applicable_with_team("key-2", Some("team-1"), None)
            .await
            .unwrap();
        assert_eq!(applicable.len(), 1);

        // Should not match key-2 with team-2
        let applicable = repo
            .find_applicable_with_team("key-2", Some("team-2"), None)
            .await
            .unwrap();
        assert_eq!(applicable.len(), 0);
    }
}
