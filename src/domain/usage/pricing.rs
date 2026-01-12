//! Model pricing configuration

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Pricing tier for volume discounts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricingTier {
    /// Minimum tokens for this tier
    pub min_tokens: u64,
    /// Price per 1K input tokens in micro-dollars
    pub input_price_per_1k_micros: i64,
    /// Price per 1K output tokens in micro-dollars
    pub output_price_per_1k_micros: i64,
}

impl PricingTier {
    /// Create a new pricing tier
    pub fn new(min_tokens: u64, input_per_1k: f64, output_per_1k: f64) -> Self {
        Self {
            min_tokens,
            input_price_per_1k_micros: (input_per_1k * 1_000_000.0) as i64,
            output_price_per_1k_micros: (output_per_1k * 1_000_000.0) as i64,
        }
    }

    /// Get input price per 1K tokens in USD
    pub fn input_price_per_1k(&self) -> f64 {
        self.input_price_per_1k_micros as f64 / 1_000_000.0
    }

    /// Get output price per 1K tokens in USD
    pub fn output_price_per_1k(&self) -> f64 {
        self.output_price_per_1k_micros as f64 / 1_000_000.0
    }
}

/// Pricing configuration for a model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPricing {
    /// Model ID this pricing applies to
    pub model_id: String,
    /// Provider name
    pub provider: String,
    /// Base price per 1K input tokens in micro-dollars
    pub input_price_per_1k_micros: i64,
    /// Base price per 1K output tokens in micro-dollars
    pub output_price_per_1k_micros: i64,
    /// Optional volume-based pricing tiers
    #[serde(default)]
    pub tiers: Vec<PricingTier>,
    /// Currency (default: USD)
    #[serde(default = "default_currency")]
    pub currency: String,
    /// Whether this pricing is active
    #[serde(default = "default_true")]
    pub active: bool,
    /// Effective date (unix timestamp)
    pub effective_from: Option<u64>,
    /// Expiry date (unix timestamp)
    pub effective_until: Option<u64>,
}

fn default_currency() -> String {
    "USD".to_string()
}

fn default_true() -> bool {
    true
}

impl ModelPricing {
    /// Create new model pricing
    pub fn new(
        model_id: impl Into<String>,
        provider: impl Into<String>,
        input_per_1k: f64,
        output_per_1k: f64,
    ) -> Self {
        Self {
            model_id: model_id.into(),
            provider: provider.into(),
            input_price_per_1k_micros: (input_per_1k * 1_000_000.0) as i64,
            output_price_per_1k_micros: (output_per_1k * 1_000_000.0) as i64,
            tiers: Vec::new(),
            currency: default_currency(),
            active: true,
            effective_from: None,
            effective_until: None,
        }
    }

    /// Add a pricing tier
    pub fn with_tier(mut self, tier: PricingTier) -> Self {
        self.tiers.push(tier);
        // Sort tiers by min_tokens descending for lookup
        self.tiers.sort_by(|a, b| b.min_tokens.cmp(&a.min_tokens));
        self
    }

    /// Set effective date range
    pub fn with_effective_dates(mut self, from: u64, until: u64) -> Self {
        self.effective_from = Some(from);
        self.effective_until = Some(until);
        self
    }

    /// Get input price per 1K tokens in USD
    pub fn input_price_per_1k(&self) -> f64 {
        self.input_price_per_1k_micros as f64 / 1_000_000.0
    }

    /// Get output price per 1K tokens in USD
    pub fn output_price_per_1k(&self) -> f64 {
        self.output_price_per_1k_micros as f64 / 1_000_000.0
    }

    /// Calculate cost for given token counts
    pub fn calculate_cost(&self, input_tokens: u32, output_tokens: u32) -> i64 {
        let total_tokens = (input_tokens + output_tokens) as u64;

        // Find applicable tier (first tier where total >= min_tokens)
        let (input_price, output_price) = self
            .tiers
            .iter()
            .find(|t| total_tokens >= t.min_tokens)
            .map(|t| (t.input_price_per_1k_micros, t.output_price_per_1k_micros))
            .unwrap_or((self.input_price_per_1k_micros, self.output_price_per_1k_micros));

        let input_cost = (input_tokens as i64 * input_price) / 1000;
        let output_cost = (output_tokens as i64 * output_price) / 1000;

        input_cost + output_cost
    }

    /// Calculate cost in USD
    pub fn calculate_cost_usd(&self, input_tokens: u32, output_tokens: u32) -> f64 {
        self.calculate_cost(input_tokens, output_tokens) as f64 / 1_000_000.0
    }

    /// Check if pricing is currently effective
    pub fn is_effective(&self, timestamp: u64) -> bool {
        if !self.active {
            return false;
        }

        if let Some(from) = self.effective_from {
            if timestamp < from {
                return false;
            }
        }

        if let Some(until) = self.effective_until {
            if timestamp > until {
                return false;
            }
        }

        true
    }
}

/// Default pricing for common models
pub fn default_model_pricing() -> HashMap<String, ModelPricing> {
    let mut pricing = HashMap::new();

    // OpenAI GPT-4o
    pricing.insert(
        "gpt-4o".to_string(),
        ModelPricing::new("gpt-4o", "openai", 0.005, 0.015),
    );

    // OpenAI GPT-4o-mini
    pricing.insert(
        "gpt-4o-mini".to_string(),
        ModelPricing::new("gpt-4o-mini", "openai", 0.00015, 0.0006),
    );

    // OpenAI GPT-4-turbo
    pricing.insert(
        "gpt-4-turbo".to_string(),
        ModelPricing::new("gpt-4-turbo", "openai", 0.01, 0.03),
    );

    // OpenAI GPT-3.5-turbo
    pricing.insert(
        "gpt-3.5-turbo".to_string(),
        ModelPricing::new("gpt-3.5-turbo", "openai", 0.0005, 0.0015),
    );

    // Anthropic Claude 3.5 Sonnet
    pricing.insert(
        "claude-3-5-sonnet-20241022".to_string(),
        ModelPricing::new("claude-3-5-sonnet-20241022", "anthropic", 0.003, 0.015),
    );

    // Anthropic Claude 3 Opus
    pricing.insert(
        "claude-3-opus-20240229".to_string(),
        ModelPricing::new("claude-3-opus-20240229", "anthropic", 0.015, 0.075),
    );

    // Anthropic Claude 3 Haiku
    pricing.insert(
        "claude-3-haiku-20240307".to_string(),
        ModelPricing::new("claude-3-haiku-20240307", "anthropic", 0.00025, 0.00125),
    );

    // OpenAI Embeddings
    pricing.insert(
        "text-embedding-3-small".to_string(),
        ModelPricing::new("text-embedding-3-small", "openai", 0.00002, 0.0),
    );

    pricing.insert(
        "text-embedding-3-large".to_string(),
        ModelPricing::new("text-embedding-3-large", "openai", 0.00013, 0.0),
    );

    pricing
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_pricing_creation() {
        let pricing = ModelPricing::new("gpt-4", "openai", 0.03, 0.06);

        assert_eq!(pricing.model_id, "gpt-4");
        assert_eq!(pricing.provider, "openai");
        assert!((pricing.input_price_per_1k() - 0.03).abs() < 0.0001);
        assert!((pricing.output_price_per_1k() - 0.06).abs() < 0.0001);
    }

    #[test]
    fn test_calculate_cost() {
        let pricing = ModelPricing::new("gpt-4", "openai", 0.03, 0.06);

        // 1000 input + 500 output
        // Cost = (1000 * 0.03 / 1000) + (500 * 0.06 / 1000) = 0.03 + 0.03 = 0.06
        let cost = pricing.calculate_cost_usd(1000, 500);
        assert!((cost - 0.06).abs() < 0.0001);
    }

    #[test]
    fn test_calculate_cost_with_tiers() {
        let pricing = ModelPricing::new("gpt-4", "openai", 0.03, 0.06)
            .with_tier(PricingTier::new(10000, 0.025, 0.05))
            .with_tier(PricingTier::new(100000, 0.02, 0.04));

        // Under first tier - use base price
        let cost1 = pricing.calculate_cost_usd(1000, 500);
        assert!((cost1 - 0.06).abs() < 0.0001);

        // Over 10K - use first tier price
        // 8000 input + 3000 output = 11000 total
        // Cost = (8000 * 0.025 / 1000) + (3000 * 0.05 / 1000) = 0.2 + 0.15 = 0.35
        let cost2 = pricing.calculate_cost_usd(8000, 3000);
        assert!((cost2 - 0.35).abs() < 0.0001);
    }

    #[test]
    fn test_is_effective() {
        let now = 1700000000u64;

        let active = ModelPricing::new("test", "test", 0.01, 0.01);
        assert!(active.is_effective(now));

        let future = ModelPricing::new("test", "test", 0.01, 0.01)
            .with_effective_dates(now + 1000, now + 2000);
        assert!(!future.is_effective(now));

        let past = ModelPricing::new("test", "test", 0.01, 0.01)
            .with_effective_dates(now - 2000, now - 1000);
        assert!(!past.is_effective(now));

        let current = ModelPricing::new("test", "test", 0.01, 0.01)
            .with_effective_dates(now - 1000, now + 1000);
        assert!(current.is_effective(now));
    }

    #[test]
    fn test_default_pricing() {
        let pricing = default_model_pricing();

        assert!(pricing.contains_key("gpt-4o"));
        assert!(pricing.contains_key("gpt-4o-mini"));
        assert!(pricing.contains_key("claude-3-5-sonnet-20241022"));
    }

    #[test]
    fn test_pricing_tier() {
        let tier = PricingTier::new(10000, 0.025, 0.05);

        assert_eq!(tier.min_tokens, 10000);
        assert!((tier.input_price_per_1k() - 0.025).abs() < 0.0001);
        assert!((tier.output_price_per_1k() - 0.05).abs() < 0.0001);
    }
}
