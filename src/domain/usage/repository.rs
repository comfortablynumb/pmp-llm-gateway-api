//! Usage and budget repository traits

use async_trait::async_trait;
use std::fmt::Debug;

use super::{Budget, BudgetId, UsageAggregate, UsageRecord, UsageRecordId, UsageSummary};
use crate::domain::DomainError;

/// Query parameters for usage records
#[derive(Debug, Clone, Default)]
pub struct UsageQuery {
    /// Filter by API key ID
    pub api_key_id: Option<String>,
    /// Filter by model ID
    pub model_id: Option<String>,
    /// Start timestamp (inclusive)
    pub from_timestamp: Option<u64>,
    /// End timestamp (exclusive)
    pub to_timestamp: Option<u64>,
    /// Maximum number of records to return
    pub limit: Option<usize>,
    /// Offset for pagination
    pub offset: Option<usize>,
}

impl UsageQuery {
    /// Create a new query
    pub fn new() -> Self {
        Self::default()
    }

    /// Filter by API key
    pub fn with_api_key(mut self, api_key_id: impl Into<String>) -> Self {
        self.api_key_id = Some(api_key_id.into());
        self
    }

    /// Filter by model
    pub fn with_model(mut self, model_id: impl Into<String>) -> Self {
        self.model_id = Some(model_id.into());
        self
    }

    /// Filter by time range
    pub fn with_time_range(mut self, from: u64, to: u64) -> Self {
        self.from_timestamp = Some(from);
        self.to_timestamp = Some(to);
        self
    }

    /// Set limit
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Set offset
    pub fn with_offset(mut self, offset: usize) -> Self {
        self.offset = Some(offset);
        self
    }
}

/// Repository for usage records
#[async_trait]
pub trait UsageRepository: Send + Sync + Debug {
    /// Record a usage event
    async fn record(&self, record: UsageRecord) -> Result<(), DomainError>;

    /// Get a usage record by ID
    async fn get(&self, id: &UsageRecordId) -> Result<Option<UsageRecord>, DomainError>;

    /// Query usage records
    async fn query(&self, query: &UsageQuery) -> Result<Vec<UsageRecord>, DomainError>;

    /// Count usage records matching query
    async fn count(&self, query: &UsageQuery) -> Result<usize, DomainError>;

    /// Get aggregated usage for a query
    async fn aggregate(&self, query: &UsageQuery) -> Result<UsageAggregate, DomainError>;

    /// Get usage summary with daily breakdown
    async fn summary(&self, query: &UsageQuery) -> Result<UsageSummary, DomainError>;

    /// Delete records older than timestamp
    async fn delete_before(&self, timestamp: u64) -> Result<usize, DomainError>;

    /// Delete all records for an API key
    async fn delete_by_api_key(&self, api_key_id: &str) -> Result<usize, DomainError>;
}

/// Repository for budgets
#[async_trait]
pub trait BudgetRepository: Send + Sync + Debug {
    /// Create a new budget
    async fn create(&self, budget: Budget) -> Result<Budget, DomainError>;

    /// Get a budget by ID
    async fn get(&self, id: &BudgetId) -> Result<Option<Budget>, DomainError>;

    /// Update a budget
    async fn update(&self, budget: Budget) -> Result<Budget, DomainError>;

    /// Delete a budget
    async fn delete(&self, id: &BudgetId) -> Result<bool, DomainError>;

    /// List all budgets
    async fn list(&self) -> Result<Vec<Budget>, DomainError>;

    /// Find budgets applicable to an API key and model (no team context)
    async fn find_applicable(
        &self,
        api_key_id: &str,
        model_id: Option<&str>,
    ) -> Result<Vec<Budget>, DomainError>;

    /// Find budgets applicable to an API key with team context
    async fn find_applicable_with_team(
        &self,
        api_key_id: &str,
        team_id: Option<&str>,
        model_id: Option<&str>,
    ) -> Result<Vec<Budget>, DomainError>;

    /// Find budgets applicable to a team
    async fn find_by_team(&self, team_id: &str) -> Result<Vec<Budget>, DomainError>;

    /// Get budgets that need period reset
    async fn get_expired_periods(&self, now: u64) -> Result<Vec<Budget>, DomainError>;
}

#[cfg(test)]
pub mod mock {
    use super::*;
    use std::collections::HashMap;
    use std::sync::RwLock;

    #[derive(Debug, Default)]
    pub struct MockUsageRepository {
        records: RwLock<HashMap<String, UsageRecord>>,
    }

    impl MockUsageRepository {
        pub fn new() -> Self {
            Self::default()
        }
    }

    #[async_trait]
    impl UsageRepository for MockUsageRepository {
        async fn record(&self, record: UsageRecord) -> Result<(), DomainError> {
            self.records
                .write()
                .unwrap()
                .insert(record.id().to_string(), record);
            Ok(())
        }

        async fn get(&self, id: &UsageRecordId) -> Result<Option<UsageRecord>, DomainError> {
            Ok(self.records.read().unwrap().get(id.as_str()).cloned())
        }

        async fn query(&self, query: &UsageQuery) -> Result<Vec<UsageRecord>, DomainError> {
            let records = self.records.read().unwrap();
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

            results.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

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
            let aggregate = self.aggregate(query).await?;
            Ok(UsageSummary {
                period_start: query.from_timestamp.unwrap_or(0),
                period_end: query.to_timestamp.unwrap_or(u64::MAX),
                aggregate,
                daily: Vec::new(),
            })
        }

        async fn delete_before(&self, timestamp: u64) -> Result<usize, DomainError> {
            let mut records = self.records.write().unwrap();
            let before_count = records.len();
            records.retain(|_, r| r.timestamp >= timestamp);
            Ok(before_count - records.len())
        }

        async fn delete_by_api_key(&self, api_key_id: &str) -> Result<usize, DomainError> {
            let mut records = self.records.write().unwrap();
            let before_count = records.len();
            records.retain(|_, r| r.api_key_id != api_key_id);
            Ok(before_count - records.len())
        }
    }

    #[derive(Debug, Default)]
    pub struct MockBudgetRepository {
        budgets: RwLock<HashMap<String, Budget>>,
    }

    impl MockBudgetRepository {
        pub fn new() -> Self {
            Self::default()
        }
    }

    #[async_trait]
    impl BudgetRepository for MockBudgetRepository {
        async fn create(&self, budget: Budget) -> Result<Budget, DomainError> {
            let mut budgets = self.budgets.write().unwrap();

            if budgets.contains_key(budget.id().as_str()) {
                return Err(DomainError::conflict(format!(
                    "Budget '{}' already exists",
                    budget.id()
                )));
            }

            budgets.insert(budget.id().to_string(), budget.clone());
            Ok(budget)
        }

        async fn get(&self, id: &BudgetId) -> Result<Option<Budget>, DomainError> {
            Ok(self.budgets.read().unwrap().get(id.as_str()).cloned())
        }

        async fn update(&self, budget: Budget) -> Result<Budget, DomainError> {
            let mut budgets = self.budgets.write().unwrap();

            if !budgets.contains_key(budget.id().as_str()) {
                return Err(DomainError::not_found(format!(
                    "Budget '{}' not found",
                    budget.id()
                )));
            }

            budgets.insert(budget.id().to_string(), budget.clone());
            Ok(budget)
        }

        async fn delete(&self, id: &BudgetId) -> Result<bool, DomainError> {
            Ok(self.budgets.write().unwrap().remove(id.as_str()).is_some())
        }

        async fn list(&self) -> Result<Vec<Budget>, DomainError> {
            Ok(self.budgets.read().unwrap().values().cloned().collect())
        }

        async fn find_applicable(
            &self,
            api_key_id: &str,
            model_id: Option<&str>,
        ) -> Result<Vec<Budget>, DomainError> {
            let budgets = self.budgets.read().unwrap();
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
            let budgets = self.budgets.read().unwrap();
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
            let budgets = self.budgets.read().unwrap();
            Ok(budgets
                .values()
                .filter(|b| b.enabled && b.applies_to_team(team_id))
                .cloned()
                .collect())
        }

        async fn get_expired_periods(&self, _now: u64) -> Result<Vec<Budget>, DomainError> {
            // Simplified - would need actual period calculation
            Ok(Vec::new())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::usage::{BudgetPeriod, UsageType};

    #[test]
    fn test_usage_query_builder() {
        let query = UsageQuery::new()
            .with_api_key("api-key-1")
            .with_model("gpt-4")
            .with_time_range(1000, 2000)
            .with_limit(100)
            .with_offset(10);

        assert_eq!(query.api_key_id, Some("api-key-1".to_string()));
        assert_eq!(query.model_id, Some("gpt-4".to_string()));
        assert_eq!(query.from_timestamp, Some(1000));
        assert_eq!(query.to_timestamp, Some(2000));
        assert_eq!(query.limit, Some(100));
        assert_eq!(query.offset, Some(10));
    }

    #[tokio::test]
    async fn test_mock_usage_repository() {
        let repo = mock::MockUsageRepository::new();

        let record = UsageRecord::new("rec-1", UsageType::ChatCompletion, "api-key-1")
            .with_model_id("gpt-4")
            .with_tokens(100, 50);

        repo.record(record.clone()).await.unwrap();

        let retrieved = repo.get(&UsageRecordId::from("rec-1")).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id().as_str(), "rec-1");
    }

    #[tokio::test]
    async fn test_mock_budget_repository() {
        let repo = mock::MockBudgetRepository::new();

        let budget = Budget::new("budget-1", "Test Budget", BudgetPeriod::Monthly)
            .with_hard_limit(100.0);

        repo.create(budget).await.unwrap();

        let retrieved = repo.get(&BudgetId::from("budget-1")).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "Test Budget");
    }
}
