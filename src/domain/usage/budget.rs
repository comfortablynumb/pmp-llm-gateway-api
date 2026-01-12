//! Budget management entities

use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use crate::domain::storage::{StorageEntity, StorageKey};

/// Budget identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BudgetId(String);

impl BudgetId {
    /// Create a new budget ID
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Get the inner string value
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for BudgetId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for BudgetId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<&String> for BudgetId {
    fn from(s: &String) -> Self {
        Self(s.clone())
    }
}

impl std::fmt::Display for BudgetId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl StorageKey for BudgetId {
    fn as_str(&self) -> &str {
        &self.0
    }
}

impl StorageEntity for Budget {
    type Key = BudgetId;

    fn key(&self) -> &Self::Key {
        &self.id
    }
}

/// Budget scope - defines what the budget applies to
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum BudgetScope {
    /// Budget applies to all API keys (no filtering)
    #[default]
    AllApiKeys,
    /// Budget applies only to specific API keys
    SpecificApiKeys,
    /// Budget applies to all API keys belonging to specific teams
    Teams,
    /// Budget applies to specific API keys AND team-level budgets
    Mixed,
}

/// Budget period for reset
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BudgetPeriod {
    /// Daily budget (resets at midnight UTC)
    Daily,
    /// Weekly budget (resets on Monday midnight UTC)
    Weekly,
    /// Monthly budget (resets on 1st of month midnight UTC)
    Monthly,
    /// No automatic reset
    Lifetime,
}

impl std::fmt::Display for BudgetPeriod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Daily => write!(f, "daily"),
            Self::Weekly => write!(f, "weekly"),
            Self::Monthly => write!(f, "monthly"),
            Self::Lifetime => write!(f, "lifetime"),
        }
    }
}

/// Budget status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BudgetStatus {
    /// Budget is active and within limits
    Active,
    /// Budget has exceeded soft limit (warning)
    Warning,
    /// Budget has exceeded hard limit (blocked)
    Exceeded,
    /// Budget is paused
    Paused,
}

impl std::fmt::Display for BudgetStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Active => write!(f, "active"),
            Self::Warning => write!(f, "warning"),
            Self::Exceeded => write!(f, "exceeded"),
            Self::Paused => write!(f, "paused"),
        }
    }
}

/// Budget alert configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetAlert {
    /// Threshold percentage (0-100)
    pub threshold_percent: u8,
    /// Whether this alert has been triggered
    pub triggered: bool,
    /// When the alert was triggered
    pub triggered_at: Option<u64>,
    /// Alert message
    pub message: Option<String>,
}

impl BudgetAlert {
    /// Create a new alert at the given threshold
    pub fn at_percent(threshold: u8) -> Self {
        Self {
            threshold_percent: threshold.min(100),
            triggered: false,
            triggered_at: None,
            message: None,
        }
    }

    /// Mark the alert as triggered
    pub fn trigger(&mut self) {
        self.triggered = true;
        self.triggered_at = Some(
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        );
    }

    /// Reset the alert
    pub fn reset(&mut self) {
        self.triggered = false;
        self.triggered_at = None;
    }
}

/// A budget configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Budget {
    /// Unique ID
    id: BudgetId,
    /// Human-readable name
    pub name: String,
    /// Description
    pub description: Option<String>,
    /// Budget period
    pub period: BudgetPeriod,
    /// Hard limit in micro-dollars (requests blocked when exceeded)
    pub hard_limit_micros: i64,
    /// Soft limit in micro-dollars (warning when exceeded)
    pub soft_limit_micros: Option<i64>,
    /// Current usage in micro-dollars for the period
    pub current_usage_micros: i64,
    /// Current status
    pub status: BudgetStatus,
    /// Budget scope
    pub scope: BudgetScope,
    /// Associated API key IDs (used when scope is SpecificApiKeys or Mixed)
    pub api_key_ids: Vec<String>,
    /// Associated team IDs (used when scope is Teams or Mixed)
    pub team_ids: Vec<String>,
    /// Associated model IDs (empty = all models)
    pub model_ids: Vec<String>,
    /// Alert configurations
    pub alerts: Vec<BudgetAlert>,
    /// When the current period started
    pub period_start: u64,
    /// When the budget was created
    pub created_at: u64,
    /// When the budget was last updated
    pub updated_at: u64,
    /// Whether the budget is enabled
    pub enabled: bool,
}

impl Budget {
    /// Create a new budget
    pub fn new(id: impl Into<BudgetId>, name: impl Into<String>, period: BudgetPeriod) -> Self {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            id: id.into(),
            name: name.into(),
            description: None,
            period,
            hard_limit_micros: 0,
            soft_limit_micros: None,
            current_usage_micros: 0,
            status: BudgetStatus::Active,
            scope: BudgetScope::AllApiKeys,
            api_key_ids: Vec::new(),
            team_ids: Vec::new(),
            model_ids: Vec::new(),
            alerts: Vec::new(),
            period_start: now,
            created_at: now,
            updated_at: now,
            enabled: true,
        }
    }

    /// Set the hard limit in USD
    pub fn with_hard_limit(mut self, limit_usd: f64) -> Self {
        self.hard_limit_micros = (limit_usd * 1_000_000.0) as i64;
        self
    }

    /// Set the hard limit in micro-dollars
    pub fn with_hard_limit_micros(mut self, limit: i64) -> Self {
        self.hard_limit_micros = limit;
        self
    }

    /// Set the soft limit in USD
    pub fn with_soft_limit(mut self, limit_usd: f64) -> Self {
        self.soft_limit_micros = Some((limit_usd * 1_000_000.0) as i64);
        self
    }

    /// Set description
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Add an API key filter (updates scope accordingly)
    pub fn with_api_key(mut self, api_key_id: impl Into<String>) -> Self {
        self.api_key_ids.push(api_key_id.into());
        self.update_scope();
        self
    }

    /// Add a team filter (updates scope accordingly)
    pub fn with_team(mut self, team_id: impl Into<String>) -> Self {
        self.team_ids.push(team_id.into());
        self.update_scope();
        self
    }

    /// Set multiple API key filters (updates scope accordingly)
    pub fn with_api_keys(mut self, api_key_ids: Vec<String>) -> Self {
        self.api_key_ids = api_key_ids;
        self.update_scope();
        self
    }

    /// Set multiple team filters (updates scope accordingly)
    pub fn with_teams(mut self, team_ids: Vec<String>) -> Self {
        self.team_ids = team_ids;
        self.update_scope();
        self
    }

    /// Update the scope based on current api_key_ids and team_ids
    fn update_scope(&mut self) {
        self.scope = match (!self.api_key_ids.is_empty(), !self.team_ids.is_empty()) {
            (false, false) => BudgetScope::AllApiKeys,
            (true, false) => BudgetScope::SpecificApiKeys,
            (false, true) => BudgetScope::Teams,
            (true, true) => BudgetScope::Mixed,
        };
    }

    /// Add a model filter
    pub fn with_model(mut self, model_id: impl Into<String>) -> Self {
        self.model_ids.push(model_id.into());
        self
    }

    /// Add an alert at threshold percentage
    pub fn with_alert_at(mut self, threshold_percent: u8) -> Self {
        self.alerts.push(BudgetAlert::at_percent(threshold_percent));
        self.alerts.sort_by(|a, b| a.threshold_percent.cmp(&b.threshold_percent));
        self
    }

    /// Get the budget ID
    pub fn id(&self) -> &BudgetId {
        &self.id
    }

    /// Get hard limit in USD
    pub fn hard_limit_usd(&self) -> f64 {
        self.hard_limit_micros as f64 / 1_000_000.0
    }

    /// Get soft limit in USD
    pub fn soft_limit_usd(&self) -> Option<f64> {
        self.soft_limit_micros.map(|m| m as f64 / 1_000_000.0)
    }

    /// Get current usage in USD
    pub fn current_usage_usd(&self) -> f64 {
        self.current_usage_micros as f64 / 1_000_000.0
    }

    /// Get remaining budget in micro-dollars
    pub fn remaining_micros(&self) -> i64 {
        self.hard_limit_micros - self.current_usage_micros
    }

    /// Get remaining budget in USD
    pub fn remaining_usd(&self) -> f64 {
        self.remaining_micros() as f64 / 1_000_000.0
    }

    /// Get usage percentage
    pub fn usage_percent(&self) -> f64 {
        if self.hard_limit_micros == 0 {
            return 0.0;
        }

        (self.current_usage_micros as f64 / self.hard_limit_micros as f64) * 100.0
    }

    /// Check if budget applies to the given API key (without team context)
    pub fn applies_to_api_key(&self, api_key_id: &str) -> bool {
        match self.scope {
            BudgetScope::AllApiKeys => true,
            BudgetScope::SpecificApiKeys | BudgetScope::Mixed => {
                self.api_key_ids.contains(&api_key_id.to_string())
            }
            BudgetScope::Teams => false, // Team scope requires team context
        }
    }

    /// Check if budget applies to the given team
    pub fn applies_to_team(&self, team_id: &str) -> bool {
        match self.scope {
            BudgetScope::AllApiKeys => true,
            BudgetScope::Teams | BudgetScope::Mixed => {
                self.team_ids.contains(&team_id.to_string())
            }
            BudgetScope::SpecificApiKeys => false,
        }
    }

    /// Check if budget applies to the given API key with team context
    pub fn applies_to_api_key_with_team(&self, api_key_id: &str, team_id: Option<&str>) -> bool {
        match self.scope {
            BudgetScope::AllApiKeys => true,
            BudgetScope::SpecificApiKeys => {
                self.api_key_ids.contains(&api_key_id.to_string())
            }
            BudgetScope::Teams => {
                team_id.map_or(false, |t| self.team_ids.contains(&t.to_string()))
            }
            BudgetScope::Mixed => {
                self.api_key_ids.contains(&api_key_id.to_string())
                    || team_id.map_or(false, |t| self.team_ids.contains(&t.to_string()))
            }
        }
    }

    /// Check if budget applies to the given model
    pub fn applies_to_model(&self, model_id: &str) -> bool {
        self.model_ids.is_empty() || self.model_ids.contains(&model_id.to_string())
    }

    /// Check if the budget allows the given cost
    pub fn allows_cost(&self, cost_micros: i64) -> bool {
        if !self.enabled || self.status == BudgetStatus::Paused {
            return true;
        }

        if self.status == BudgetStatus::Exceeded {
            return false;
        }

        self.current_usage_micros + cost_micros <= self.hard_limit_micros
    }

    /// Add usage to the budget and update status
    pub fn add_usage(&mut self, cost_micros: i64) -> BudgetStatus {
        self.current_usage_micros += cost_micros;
        self.updated_at = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        self.update_status();
        self.check_alerts();

        self.status
    }

    /// Update the budget status based on current usage
    fn update_status(&mut self) {
        if self.status == BudgetStatus::Paused {
            return;
        }

        if self.current_usage_micros >= self.hard_limit_micros {
            self.status = BudgetStatus::Exceeded;
        } else if let Some(soft_limit) = self.soft_limit_micros {
            if self.current_usage_micros >= soft_limit {
                self.status = BudgetStatus::Warning;
            } else {
                self.status = BudgetStatus::Active;
            }
        } else {
            self.status = BudgetStatus::Active;
        }
    }

    /// Check and trigger alerts
    fn check_alerts(&mut self) {
        let usage_percent = self.usage_percent() as u8;

        for alert in &mut self.alerts {
            if !alert.triggered && usage_percent >= alert.threshold_percent {
                alert.trigger();
            }
        }
    }

    /// Reset the budget for a new period
    pub fn reset_period(&mut self) {
        self.current_usage_micros = 0;
        self.status = BudgetStatus::Active;
        self.period_start = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        for alert in &mut self.alerts {
            alert.reset();
        }
    }

    /// Get triggered alerts
    pub fn triggered_alerts(&self) -> Vec<&BudgetAlert> {
        self.alerts.iter().filter(|a| a.triggered).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_budget_creation() {
        let budget = Budget::new("budget-1", "Monthly Budget", BudgetPeriod::Monthly)
            .with_hard_limit(100.0)
            .with_soft_limit(80.0)
            .with_description("Test budget");

        assert_eq!(budget.id().as_str(), "budget-1");
        assert_eq!(budget.name, "Monthly Budget");
        assert_eq!(budget.period, BudgetPeriod::Monthly);
        assert!((budget.hard_limit_usd() - 100.0).abs() < 0.01);
        assert!((budget.soft_limit_usd().unwrap() - 80.0).abs() < 0.01);
        assert_eq!(budget.status, BudgetStatus::Active);
    }

    #[test]
    fn test_budget_usage() {
        let mut budget = Budget::new("budget-1", "Test", BudgetPeriod::Monthly)
            .with_hard_limit(100.0)
            .with_soft_limit(80.0);

        // Add some usage
        budget.add_usage(50_000_000); // $50
        assert_eq!(budget.status, BudgetStatus::Active);
        assert!((budget.usage_percent() - 50.0).abs() < 0.1);

        // Exceed soft limit
        budget.add_usage(35_000_000); // $35 more = $85 total
        assert_eq!(budget.status, BudgetStatus::Warning);

        // Exceed hard limit
        budget.add_usage(20_000_000); // $20 more = $105 total
        assert_eq!(budget.status, BudgetStatus::Exceeded);
    }

    #[test]
    fn test_budget_allows_cost() {
        let mut budget = Budget::new("budget-1", "Test", BudgetPeriod::Monthly)
            .with_hard_limit(100.0);

        assert!(budget.allows_cost(50_000_000)); // $50
        assert!(budget.allows_cost(100_000_000)); // $100

        budget.add_usage(90_000_000); // $90

        assert!(budget.allows_cost(10_000_000)); // $10 more OK
        assert!(!budget.allows_cost(15_000_000)); // $15 more exceeds
    }

    #[test]
    fn test_budget_alerts() {
        let mut budget = Budget::new("budget-1", "Test", BudgetPeriod::Monthly)
            .with_hard_limit(100.0)
            .with_alert_at(50)
            .with_alert_at(75)
            .with_alert_at(90);

        // No alerts triggered
        assert!(budget.triggered_alerts().is_empty());

        // Add 60% usage - should trigger 50% alert
        budget.add_usage(60_000_000);
        assert_eq!(budget.triggered_alerts().len(), 1);
        assert_eq!(budget.triggered_alerts()[0].threshold_percent, 50);

        // Add more to 80% - should trigger 75% alert too
        budget.add_usage(20_000_000);
        assert_eq!(budget.triggered_alerts().len(), 2);
    }

    #[test]
    fn test_budget_reset() {
        let mut budget = Budget::new("budget-1", "Test", BudgetPeriod::Monthly)
            .with_hard_limit(100.0)
            .with_alert_at(50);

        budget.add_usage(60_000_000);
        assert!(!budget.triggered_alerts().is_empty());

        budget.reset_period();

        assert_eq!(budget.current_usage_micros, 0);
        assert_eq!(budget.status, BudgetStatus::Active);
        assert!(budget.triggered_alerts().is_empty());
    }

    #[test]
    fn test_budget_filters() {
        let budget = Budget::new("budget-1", "Test", BudgetPeriod::Monthly)
            .with_api_key("api-key-1")
            .with_model("gpt-4");

        assert_eq!(budget.scope, BudgetScope::SpecificApiKeys);
        assert!(budget.applies_to_api_key("api-key-1"));
        assert!(!budget.applies_to_api_key("api-key-2"));

        assert!(budget.applies_to_model("gpt-4"));
        assert!(!budget.applies_to_model("gpt-3.5"));
    }

    #[test]
    fn test_budget_filters_empty() {
        let budget = Budget::new("budget-1", "Test", BudgetPeriod::Monthly);

        assert_eq!(budget.scope, BudgetScope::AllApiKeys);
        // Empty filters apply to all
        assert!(budget.applies_to_api_key("any-key"));
        assert!(budget.applies_to_model("any-model"));
    }

    #[test]
    fn test_budget_team_filters() {
        let budget = Budget::new("budget-1", "Test", BudgetPeriod::Monthly)
            .with_team("team-1");

        assert_eq!(budget.scope, BudgetScope::Teams);
        assert!(budget.applies_to_team("team-1"));
        assert!(!budget.applies_to_team("team-2"));
        // Team scope doesn't apply to API keys without team context
        assert!(!budget.applies_to_api_key("api-key-1"));
    }

    #[test]
    fn test_budget_mixed_scope() {
        let budget = Budget::new("budget-1", "Test", BudgetPeriod::Monthly)
            .with_api_key("api-key-1")
            .with_team("team-1");

        assert_eq!(budget.scope, BudgetScope::Mixed);
        assert!(budget.applies_to_api_key("api-key-1"));
        assert!(!budget.applies_to_api_key("api-key-2"));
        assert!(budget.applies_to_team("team-1"));
        assert!(!budget.applies_to_team("team-2"));
    }

    #[test]
    fn test_budget_applies_with_team_context() {
        // Team-scoped budget
        let team_budget = Budget::new("budget-1", "Test", BudgetPeriod::Monthly)
            .with_team("team-1");

        // Should apply when team matches
        assert!(team_budget.applies_to_api_key_with_team("any-key", Some("team-1")));
        assert!(!team_budget.applies_to_api_key_with_team("any-key", Some("team-2")));
        assert!(!team_budget.applies_to_api_key_with_team("any-key", None));

        // API key scoped budget
        let key_budget = Budget::new("budget-2", "Test", BudgetPeriod::Monthly)
            .with_api_key("api-key-1");

        // Should only apply when API key matches, ignores team
        assert!(key_budget.applies_to_api_key_with_team("api-key-1", Some("any-team")));
        assert!(key_budget.applies_to_api_key_with_team("api-key-1", None));
        assert!(!key_budget.applies_to_api_key_with_team("api-key-2", Some("team-1")));

        // Mixed scope budget
        let mixed_budget = Budget::new("budget-3", "Test", BudgetPeriod::Monthly)
            .with_api_key("api-key-1")
            .with_team("team-1");

        // Should apply when either matches
        assert!(mixed_budget.applies_to_api_key_with_team("api-key-1", None));
        assert!(mixed_budget.applies_to_api_key_with_team("api-key-2", Some("team-1")));
        assert!(mixed_budget.applies_to_api_key_with_team("api-key-1", Some("team-1")));
        assert!(!mixed_budget.applies_to_api_key_with_team("api-key-2", Some("team-2")));
        assert!(!mixed_budget.applies_to_api_key_with_team("api-key-2", None));

        // All API keys budget
        let all_budget = Budget::new("budget-4", "Test", BudgetPeriod::Monthly);

        assert!(all_budget.applies_to_api_key_with_team("any-key", None));
        assert!(all_budget.applies_to_api_key_with_team("any-key", Some("any-team")));
    }

    #[test]
    fn test_budget_with_multiple_teams() {
        let budget = Budget::new("budget-1", "Test", BudgetPeriod::Monthly)
            .with_teams(vec!["team-1".to_string(), "team-2".to_string()]);

        assert_eq!(budget.scope, BudgetScope::Teams);
        assert!(budget.applies_to_team("team-1"));
        assert!(budget.applies_to_team("team-2"));
        assert!(!budget.applies_to_team("team-3"));
    }

    #[test]
    fn test_budget_with_multiple_api_keys() {
        let budget = Budget::new("budget-1", "Test", BudgetPeriod::Monthly)
            .with_api_keys(vec!["key-1".to_string(), "key-2".to_string()]);

        assert_eq!(budget.scope, BudgetScope::SpecificApiKeys);
        assert!(budget.applies_to_api_key("key-1"));
        assert!(budget.applies_to_api_key("key-2"));
        assert!(!budget.applies_to_api_key("key-3"));
    }

    #[test]
    fn test_budget_period_display() {
        assert_eq!(BudgetPeriod::Daily.to_string(), "daily");
        assert_eq!(BudgetPeriod::Monthly.to_string(), "monthly");
    }

    #[test]
    fn test_budget_status_display() {
        assert_eq!(BudgetStatus::Active.to_string(), "active");
        assert_eq!(BudgetStatus::Exceeded.to_string(), "exceeded");
    }
}
