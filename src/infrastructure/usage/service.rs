//! Usage tracking and budget management services

use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;
use std::time::SystemTime;

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::usage::{
    Budget, BudgetAlert, BudgetId, BudgetPeriod, BudgetRepository, BudgetStatus, ModelPricing,
    UsageAggregate, UsageQuery, UsageRecord, UsageRecordId, UsageRepository, UsageSummary,
    UsageType,
};
use crate::domain::DomainError;

/// Parameters for recording usage
#[derive(Debug, Clone)]
pub struct RecordUsageParams {
    pub usage_type: UsageType,
    pub api_key_id: String,
    pub model_id: Option<String>,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub latency_ms: u64,
    pub success: bool,
    pub error: Option<String>,
    pub metadata: HashMap<String, String>,
}

impl RecordUsageParams {
    pub fn new(usage_type: UsageType, api_key_id: impl Into<String>) -> Self {
        Self {
            usage_type,
            api_key_id: api_key_id.into(),
            model_id: None,
            input_tokens: 0,
            output_tokens: 0,
            latency_ms: 0,
            success: true,
            error: None,
            metadata: HashMap::new(),
        }
    }

    pub fn with_model(mut self, model_id: impl Into<String>) -> Self {
        self.model_id = Some(model_id.into());
        self
    }

    pub fn with_tokens(mut self, input: u32, output: u32) -> Self {
        self.input_tokens = input;
        self.output_tokens = output;
        self
    }

    pub fn with_latency(mut self, latency_ms: u64) -> Self {
        self.latency_ms = latency_ms;
        self
    }

    pub fn with_error(mut self, error: impl Into<String>) -> Self {
        self.success = false;
        self.error = Some(error.into());
        self
    }
}

/// Trait for usage tracking service
#[async_trait]
pub trait UsageTrackingServiceTrait: Send + Sync + Debug {
    /// Record a usage event
    async fn record(&self, params: RecordUsageParams) -> Result<UsageRecord, DomainError>;

    /// Get a usage record by ID
    async fn get(&self, id: &UsageRecordId) -> Result<Option<UsageRecord>, DomainError>;

    /// Query usage records
    async fn query(&self, query: &UsageQuery) -> Result<Vec<UsageRecord>, DomainError>;

    /// Count usage records matching query
    async fn count(&self, query: &UsageQuery) -> Result<usize, DomainError>;

    /// Get aggregated usage
    async fn aggregate(&self, query: &UsageQuery) -> Result<UsageAggregate, DomainError>;

    /// Get usage summary with daily breakdown
    async fn summary(&self, query: &UsageQuery) -> Result<UsageSummary, DomainError>;

    /// Delete records older than timestamp
    async fn delete_before(&self, timestamp: u64) -> Result<usize, DomainError>;

    /// Delete all records for an API key
    async fn delete_by_api_key(&self, api_key_id: &str) -> Result<usize, DomainError>;

    /// Get pricing for a model
    fn get_pricing(&self, model_id: &str) -> Option<&ModelPricing>;

    /// Calculate cost for tokens
    fn calculate_cost(&self, model_id: &str, input_tokens: u32, output_tokens: u32) -> i64;
}

/// Usage tracking service implementation
#[derive(Debug)]
pub struct UsageTrackingService<R: UsageRepository> {
    repository: Arc<R>,
    pricing: HashMap<String, ModelPricing>,
}

impl<R: UsageRepository> UsageTrackingService<R> {
    /// Create a new usage tracking service
    pub fn new(repository: Arc<R>) -> Self {
        Self {
            repository,
            pricing: crate::domain::usage::default_model_pricing(),
        }
    }

    /// Create with custom pricing
    pub fn with_pricing(repository: Arc<R>, pricing: HashMap<String, ModelPricing>) -> Self {
        Self { repository, pricing }
    }

    /// Add or update pricing for a model
    pub fn set_pricing(&mut self, pricing: ModelPricing) {
        self.pricing.insert(pricing.model_id.clone(), pricing);
    }

    fn generate_id(&self) -> String {
        format!("usage-{}", Uuid::new_v4())
    }
}

#[async_trait]
impl<R: UsageRepository + 'static> UsageTrackingServiceTrait for UsageTrackingService<R> {
    async fn record(&self, params: RecordUsageParams) -> Result<UsageRecord, DomainError> {
        let cost = params
            .model_id
            .as_ref()
            .map(|m| self.calculate_cost(m, params.input_tokens, params.output_tokens))
            .unwrap_or(0);

        let mut record = UsageRecord::new(self.generate_id(), params.usage_type, params.api_key_id)
            .with_tokens(params.input_tokens, params.output_tokens)
            .with_cost_micros(cost)
            .with_latency_ms(params.latency_ms);

        if let Some(model_id) = params.model_id {
            record = record.with_model_id(model_id);
        }

        if let Some(error) = params.error {
            record = record.with_error(error);
        }

        for (key, value) in params.metadata {
            record = record.with_metadata(key, value);
        }

        self.repository.record(record.clone()).await?;

        Ok(record)
    }

    async fn get(&self, id: &UsageRecordId) -> Result<Option<UsageRecord>, DomainError> {
        self.repository.get(id).await
    }

    async fn query(&self, query: &UsageQuery) -> Result<Vec<UsageRecord>, DomainError> {
        self.repository.query(query).await
    }

    async fn count(&self, query: &UsageQuery) -> Result<usize, DomainError> {
        self.repository.count(query).await
    }

    async fn aggregate(&self, query: &UsageQuery) -> Result<UsageAggregate, DomainError> {
        self.repository.aggregate(query).await
    }

    async fn summary(&self, query: &UsageQuery) -> Result<UsageSummary, DomainError> {
        self.repository.summary(query).await
    }

    async fn delete_before(&self, timestamp: u64) -> Result<usize, DomainError> {
        self.repository.delete_before(timestamp).await
    }

    async fn delete_by_api_key(&self, api_key_id: &str) -> Result<usize, DomainError> {
        self.repository.delete_by_api_key(api_key_id).await
    }

    fn get_pricing(&self, model_id: &str) -> Option<&ModelPricing> {
        self.pricing.get(model_id)
    }

    fn calculate_cost(&self, model_id: &str, input_tokens: u32, output_tokens: u32) -> i64 {
        self.pricing
            .get(model_id)
            .map(|p| p.calculate_cost(input_tokens, output_tokens))
            .unwrap_or(0)
    }
}

/// Budget check result
#[derive(Debug, Clone)]
pub struct BudgetCheckResult {
    /// Whether the request is allowed
    pub allowed: bool,
    /// Budgets that would be exceeded
    pub exceeded_budgets: Vec<BudgetId>,
    /// Budgets in warning state
    pub warning_budgets: Vec<BudgetId>,
    /// Estimated cost in micro-dollars
    pub estimated_cost_micros: i64,
}

impl BudgetCheckResult {
    fn new(estimated_cost: i64) -> Self {
        Self {
            allowed: true,
            exceeded_budgets: Vec::new(),
            warning_budgets: Vec::new(),
            estimated_cost_micros: estimated_cost,
        }
    }
}

/// Alert notification
#[derive(Debug, Clone)]
pub struct AlertNotification {
    pub budget_id: BudgetId,
    pub budget_name: String,
    pub alert: BudgetAlert,
    pub current_usage_micros: i64,
    pub limit_micros: i64,
}

/// Trait for budget service
#[async_trait]
pub trait BudgetServiceTrait: Send + Sync + Debug {
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

    /// List budgets filtered by team
    async fn list_by_team(&self, team_id: &str) -> Result<Vec<Budget>, DomainError>;

    /// Check if a request is allowed based on budgets (no team context)
    async fn check_budget(
        &self,
        api_key_id: &str,
        model_id: Option<&str>,
        estimated_cost_micros: i64,
    ) -> Result<BudgetCheckResult, DomainError>;

    /// Check if a request is allowed based on budgets (with team context)
    async fn check_budget_with_team(
        &self,
        api_key_id: &str,
        team_id: Option<&str>,
        model_id: Option<&str>,
        estimated_cost_micros: i64,
    ) -> Result<BudgetCheckResult, DomainError>;

    /// Record usage against applicable budgets (no team context)
    async fn record_usage(
        &self,
        api_key_id: &str,
        model_id: Option<&str>,
        cost_micros: i64,
    ) -> Result<Vec<AlertNotification>, DomainError>;

    /// Record usage against applicable budgets (with team context)
    async fn record_usage_with_team(
        &self,
        api_key_id: &str,
        team_id: Option<&str>,
        model_id: Option<&str>,
        cost_micros: i64,
    ) -> Result<Vec<AlertNotification>, DomainError>;

    /// Reset expired budget periods
    async fn reset_expired_periods(&self) -> Result<usize, DomainError>;
}

/// Budget service implementation
#[derive(Debug)]
pub struct BudgetService<R: BudgetRepository> {
    repository: Arc<R>,
}

impl<R: BudgetRepository> BudgetService<R> {
    /// Create a new budget service
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }

    fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    fn calculate_period_end(period: BudgetPeriod, period_start: u64) -> u64 {
        match period {
            BudgetPeriod::Daily => period_start + 86400,
            BudgetPeriod::Weekly => period_start + 604800,
            BudgetPeriod::Monthly => period_start + 2592000, // ~30 days
            BudgetPeriod::Lifetime => u64::MAX,
        }
    }
}

#[async_trait]
impl<R: BudgetRepository + 'static> BudgetServiceTrait for BudgetService<R> {
    async fn create(&self, budget: Budget) -> Result<Budget, DomainError> {
        crate::domain::usage::validate_budget_id(budget.id().as_str())
            .map_err(|e| DomainError::validation(e.to_string()))?;

        if budget.hard_limit_micros <= 0 {
            return Err(DomainError::validation("Hard limit must be positive"));
        }

        if let Some(soft_limit) = budget.soft_limit_micros {
            if soft_limit >= budget.hard_limit_micros {
                return Err(DomainError::validation(
                    "Soft limit must be less than hard limit",
                ));
            }
        }

        self.repository.create(budget).await
    }

    async fn get(&self, id: &BudgetId) -> Result<Option<Budget>, DomainError> {
        self.repository.get(id).await
    }

    async fn update(&self, budget: Budget) -> Result<Budget, DomainError> {
        if budget.hard_limit_micros <= 0 {
            return Err(DomainError::validation("Hard limit must be positive"));
        }

        if let Some(soft_limit) = budget.soft_limit_micros {
            if soft_limit >= budget.hard_limit_micros {
                return Err(DomainError::validation(
                    "Soft limit must be less than hard limit",
                ));
            }
        }

        self.repository.update(budget).await
    }

    async fn delete(&self, id: &BudgetId) -> Result<bool, DomainError> {
        self.repository.delete(id).await
    }

    async fn list(&self) -> Result<Vec<Budget>, DomainError> {
        self.repository.list().await
    }

    async fn list_by_team(&self, team_id: &str) -> Result<Vec<Budget>, DomainError> {
        self.repository.find_by_team(team_id).await
    }

    async fn check_budget(
        &self,
        api_key_id: &str,
        model_id: Option<&str>,
        estimated_cost_micros: i64,
    ) -> Result<BudgetCheckResult, DomainError> {
        let budgets = self
            .repository
            .find_applicable(api_key_id, model_id)
            .await?;

        let mut result = BudgetCheckResult::new(estimated_cost_micros);

        for budget in budgets {
            if !budget.allows_cost(estimated_cost_micros) {
                result.allowed = false;
                result.exceeded_budgets.push(budget.id().clone());
            } else if budget.status == BudgetStatus::Warning {
                result.warning_budgets.push(budget.id().clone());
            }
        }

        Ok(result)
    }

    async fn check_budget_with_team(
        &self,
        api_key_id: &str,
        team_id: Option<&str>,
        model_id: Option<&str>,
        estimated_cost_micros: i64,
    ) -> Result<BudgetCheckResult, DomainError> {
        let budgets = self
            .repository
            .find_applicable_with_team(api_key_id, team_id, model_id)
            .await?;

        let mut result = BudgetCheckResult::new(estimated_cost_micros);

        for budget in budgets {
            if !budget.allows_cost(estimated_cost_micros) {
                result.allowed = false;
                result.exceeded_budgets.push(budget.id().clone());
            } else if budget.status == BudgetStatus::Warning {
                result.warning_budgets.push(budget.id().clone());
            }
        }

        Ok(result)
    }

    async fn record_usage(
        &self,
        api_key_id: &str,
        model_id: Option<&str>,
        cost_micros: i64,
    ) -> Result<Vec<AlertNotification>, DomainError> {
        let budgets = self
            .repository
            .find_applicable(api_key_id, model_id)
            .await?;

        let mut notifications = Vec::new();

        for mut budget in budgets {
            let alerts_before: Vec<_> = budget.alerts.iter().map(|a| a.triggered).collect();

            budget.add_usage(cost_micros);

            // Check for newly triggered alerts
            for (i, alert) in budget.alerts.iter().enumerate() {
                if alert.triggered && !alerts_before.get(i).copied().unwrap_or(false) {
                    notifications.push(AlertNotification {
                        budget_id: budget.id().clone(),
                        budget_name: budget.name.clone(),
                        alert: alert.clone(),
                        current_usage_micros: budget.current_usage_micros,
                        limit_micros: budget.hard_limit_micros,
                    });
                }
            }

            self.repository.update(budget).await?;
        }

        Ok(notifications)
    }

    async fn record_usage_with_team(
        &self,
        api_key_id: &str,
        team_id: Option<&str>,
        model_id: Option<&str>,
        cost_micros: i64,
    ) -> Result<Vec<AlertNotification>, DomainError> {
        let budgets = self
            .repository
            .find_applicable_with_team(api_key_id, team_id, model_id)
            .await?;

        let mut notifications = Vec::new();

        for mut budget in budgets {
            let alerts_before: Vec<_> = budget.alerts.iter().map(|a| a.triggered).collect();

            budget.add_usage(cost_micros);

            // Check for newly triggered alerts
            for (i, alert) in budget.alerts.iter().enumerate() {
                if alert.triggered && !alerts_before.get(i).copied().unwrap_or(false) {
                    notifications.push(AlertNotification {
                        budget_id: budget.id().clone(),
                        budget_name: budget.name.clone(),
                        alert: alert.clone(),
                        current_usage_micros: budget.current_usage_micros,
                        limit_micros: budget.hard_limit_micros,
                    });
                }
            }

            self.repository.update(budget).await?;
        }

        Ok(notifications)
    }

    async fn reset_expired_periods(&self) -> Result<usize, DomainError> {
        let now = Self::current_timestamp();
        let expired = self.repository.get_expired_periods(now).await?;
        let count = expired.len();

        for mut budget in expired {
            let period_end = Self::calculate_period_end(budget.period, budget.period_start);

            if now >= period_end {
                budget.reset_period();
                self.repository.update(budget).await?;
            }
        }

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::usage::InMemoryBudgetRepository;
    use crate::infrastructure::usage::InMemoryUsageRepository;

    #[tokio::test]
    async fn test_usage_tracking_service_record() {
        let repo = Arc::new(InMemoryUsageRepository::new(100));
        let service = UsageTrackingService::new(repo);

        let params = RecordUsageParams::new(UsageType::ChatCompletion, "api-key-1")
            .with_model("gpt-4o")
            .with_tokens(100, 50)
            .with_latency(250);

        let record = service.record(params).await.unwrap();

        assert!(record.id().as_str().starts_with("usage-"));
        assert_eq!(record.api_key_id, "api-key-1");
        assert_eq!(record.model_id, Some("gpt-4o".to_string()));
        assert_eq!(record.input_tokens, 100);
        assert_eq!(record.output_tokens, 50);
        assert!(record.cost_micros > 0);
        assert!(record.success);
    }

    #[tokio::test]
    async fn test_usage_tracking_service_query() {
        let repo = Arc::new(InMemoryUsageRepository::new(100));
        let service = UsageTrackingService::new(repo);

        for i in 0..5 {
            let params =
                RecordUsageParams::new(UsageType::ChatCompletion, format!("key-{}", i % 2))
                    .with_model("gpt-4o")
                    .with_tokens(100, 50);

            service.record(params).await.unwrap();
        }

        let query = UsageQuery::new().with_api_key("key-0");
        let results = service.query(&query).await.unwrap();

        assert_eq!(results.len(), 3);
    }

    #[tokio::test]
    async fn test_usage_tracking_service_aggregate() {
        let repo = Arc::new(InMemoryUsageRepository::new(100));
        let service = UsageTrackingService::new(repo);

        let params1 = RecordUsageParams::new(UsageType::ChatCompletion, "api-key-1")
            .with_model("gpt-4o")
            .with_tokens(100, 50);

        let params2 = RecordUsageParams::new(UsageType::ChatCompletion, "api-key-1")
            .with_model("gpt-4o")
            .with_tokens(200, 100);

        service.record(params1).await.unwrap();
        service.record(params2).await.unwrap();

        let query = UsageQuery::new().with_api_key("api-key-1");
        let aggregate = service.aggregate(&query).await.unwrap();

        assert_eq!(aggregate.total_requests, 2);
        assert_eq!(aggregate.total_input_tokens, 300);
        assert_eq!(aggregate.total_output_tokens, 150);
    }

    #[tokio::test]
    async fn test_usage_tracking_service_calculate_cost() {
        let repo = Arc::new(InMemoryUsageRepository::new(100));
        let service = UsageTrackingService::new(repo);

        let cost = service.calculate_cost("gpt-4o", 1000, 500);
        assert!(cost > 0);

        let unknown_cost = service.calculate_cost("unknown-model", 1000, 500);
        assert_eq!(unknown_cost, 0);
    }

    #[tokio::test]
    async fn test_budget_service_create() {
        let repo = Arc::new(InMemoryBudgetRepository::new());
        let service = BudgetService::new(repo);

        let budget = Budget::new("budget-1", "Test Budget", BudgetPeriod::Monthly)
            .with_hard_limit(100.0)
            .with_soft_limit(80.0);

        let created = service.create(budget).await.unwrap();

        assert_eq!(created.id().as_str(), "budget-1");
        assert_eq!(created.name, "Test Budget");
    }

    #[tokio::test]
    async fn test_budget_service_validation() {
        let repo = Arc::new(InMemoryBudgetRepository::new());
        let service = BudgetService::new(repo);

        let invalid_budget = Budget::new("budget-1", "Test", BudgetPeriod::Monthly);

        let result = service.create(invalid_budget).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_budget_service_check_budget() {
        let repo = Arc::new(InMemoryBudgetRepository::new());
        let service = BudgetService::new(repo);

        let budget = Budget::new("budget-1", "Test Budget", BudgetPeriod::Monthly)
            .with_hard_limit(100.0)
            .with_api_key("api-key-1");

        service.create(budget).await.unwrap();

        let result = service
            .check_budget("api-key-1", None, 50_000_000)
            .await
            .unwrap();
        assert!(result.allowed);

        let result = service
            .check_budget("api-key-1", None, 150_000_000)
            .await
            .unwrap();
        assert!(!result.allowed);
        assert_eq!(result.exceeded_budgets.len(), 1);
    }

    #[tokio::test]
    async fn test_budget_service_record_usage() {
        let repo = Arc::new(InMemoryBudgetRepository::new());
        let service = BudgetService::new(repo.clone());

        let budget = Budget::new("budget-1", "Test Budget", BudgetPeriod::Monthly)
            .with_hard_limit(100.0)
            .with_alert_at(50)
            .with_api_key("api-key-1");

        service.create(budget).await.unwrap();

        let notifications = service
            .record_usage("api-key-1", None, 60_000_000)
            .await
            .unwrap();

        assert_eq!(notifications.len(), 1);
        assert_eq!(notifications[0].alert.threshold_percent, 50);

        let budget = service.get(&BudgetId::from("budget-1")).await.unwrap().unwrap();
        assert_eq!(budget.current_usage_micros, 60_000_000);
    }

    #[tokio::test]
    async fn test_budget_service_non_applicable_budget() {
        let repo = Arc::new(InMemoryBudgetRepository::new());
        let service = BudgetService::new(repo);

        let budget = Budget::new("budget-1", "Test Budget", BudgetPeriod::Monthly)
            .with_hard_limit(100.0)
            .with_api_key("api-key-1");

        service.create(budget).await.unwrap();

        let result = service
            .check_budget("api-key-2", None, 50_000_000)
            .await
            .unwrap();
        assert!(result.allowed);
        assert!(result.exceeded_budgets.is_empty());
    }
}
