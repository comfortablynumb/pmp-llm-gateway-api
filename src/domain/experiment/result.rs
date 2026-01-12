//! Experiment result types for metrics and statistical analysis

use serde::{Deserialize, Serialize};

use super::entity::ExperimentStatus;
use super::record::ExperimentRecord;

// ============================================================================
// LatencyStats
// ============================================================================

/// Latency statistics for a variant
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LatencyStats {
    /// Average latency in milliseconds
    pub avg_ms: f64,
    /// Minimum latency in milliseconds
    pub min_ms: u64,
    /// Maximum latency in milliseconds
    pub max_ms: u64,
    /// 50th percentile (median) latency
    pub p50_ms: u64,
    /// 95th percentile latency
    pub p95_ms: u64,
    /// 99th percentile latency
    pub p99_ms: u64,
}

impl LatencyStats {
    /// Calculate latency statistics from a list of samples
    pub fn from_samples(mut samples: Vec<u64>) -> Self {
        if samples.is_empty() {
            return Self::default();
        }

        samples.sort_unstable();
        let len = samples.len();
        let sum: u64 = samples.iter().sum();

        Self {
            avg_ms: sum as f64 / len as f64,
            min_ms: samples[0],
            max_ms: samples[len - 1],
            p50_ms: percentile(&samples, 50.0),
            p95_ms: percentile(&samples, 95.0),
            p99_ms: percentile(&samples, 99.0),
        }
    }
}

/// Calculate a percentile from a sorted list
fn percentile(sorted_samples: &[u64], p: f64) -> u64 {
    if sorted_samples.is_empty() {
        return 0;
    }

    if sorted_samples.len() == 1 {
        return sorted_samples[0];
    }

    let index = (p / 100.0 * (sorted_samples.len() - 1) as f64) as usize;
    sorted_samples[index.min(sorted_samples.len() - 1)]
}

// ============================================================================
// VariantMetrics
// ============================================================================

/// Aggregated metrics for a variant
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VariantMetrics {
    /// Variant ID
    pub variant_id: String,
    /// Variant name
    pub variant_name: String,
    /// Total number of requests
    pub total_requests: u64,
    /// Number of successful requests
    pub successful_requests: u64,
    /// Number of failed requests
    pub failed_requests: u64,
    /// Success rate (0.0 - 1.0)
    pub success_rate: f64,
    /// Total input tokens used
    pub total_input_tokens: u64,
    /// Total output tokens used
    pub total_output_tokens: u64,
    /// Total tokens used
    pub total_tokens: u64,
    /// Total cost in micro-dollars
    pub total_cost_micros: i64,
    /// Average cost per request in micro-dollars
    pub avg_cost_micros: f64,
    /// Latency statistics
    pub latency: LatencyStats,
}

impl VariantMetrics {
    /// Create new metrics for a variant
    pub fn new(variant_id: impl Into<String>, variant_name: impl Into<String>) -> Self {
        Self {
            variant_id: variant_id.into(),
            variant_name: variant_name.into(),
            ..Default::default()
        }
    }

    /// Add a record to the metrics
    pub fn add_record(&mut self, record: &ExperimentRecord) {
        self.total_requests += 1;

        if record.success {
            self.successful_requests += 1;
        } else {
            self.failed_requests += 1;
        }

        self.total_input_tokens += record.input_tokens as u64;
        self.total_output_tokens += record.output_tokens as u64;
        self.total_tokens += record.total_tokens as u64;
        self.total_cost_micros += record.cost_micros;

        self.update_rates();
    }

    /// Update derived rates
    fn update_rates(&mut self) {
        if self.total_requests > 0 {
            self.success_rate = self.successful_requests as f64 / self.total_requests as f64;
            self.avg_cost_micros = self.total_cost_micros as f64 / self.total_requests as f64;
        }
    }

    /// Set latency statistics from samples
    pub fn set_latency_from_samples(&mut self, samples: Vec<u64>) {
        self.latency = LatencyStats::from_samples(samples);
    }

    /// Get the total cost in USD
    pub fn total_cost_usd(&self) -> f64 {
        self.total_cost_micros as f64 / 1_000_000.0
    }

    /// Get the average cost per request in USD
    pub fn avg_cost_usd(&self) -> f64 {
        self.avg_cost_micros / 1_000_000.0
    }
}

// ============================================================================
// StatisticalSignificance
// ============================================================================

/// Results of statistical significance testing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatisticalSignificance {
    /// P-value from the statistical test
    pub p_value: f64,
    /// Whether the result is statistically significant
    pub is_significant: bool,
    /// Confidence level used (e.g., 0.95 for 95%)
    pub confidence_level: f64,
    /// ID of the control variant
    pub control_variant_id: String,
    /// ID of the treatment variant being compared
    pub treatment_variant_id: String,
    /// Name of the metric being compared
    pub metric: String,
    /// Mean value for the control variant
    pub control_mean: f64,
    /// Mean value for the treatment variant
    pub treatment_mean: f64,
    /// Relative change from control to treatment (percentage)
    pub relative_change: f64,
}

impl StatisticalSignificance {
    /// Create a new significance result
    pub fn new(
        p_value: f64,
        confidence_level: f64,
        control_variant_id: impl Into<String>,
        treatment_variant_id: impl Into<String>,
        metric: impl Into<String>,
        control_mean: f64,
        treatment_mean: f64,
    ) -> Self {
        let relative_change = if control_mean != 0.0 {
            (treatment_mean - control_mean) / control_mean * 100.0
        } else {
            0.0
        };

        Self {
            p_value,
            is_significant: p_value < (1.0 - confidence_level),
            confidence_level,
            control_variant_id: control_variant_id.into(),
            treatment_variant_id: treatment_variant_id.into(),
            metric: metric.into(),
            control_mean,
            treatment_mean,
            relative_change,
        }
    }

    /// Check if the treatment is better than control (for metrics where lower is better)
    pub fn treatment_is_better_lower(&self) -> bool {
        self.is_significant && self.treatment_mean < self.control_mean
    }

    /// Check if the treatment is better than control (for metrics where higher is better)
    pub fn treatment_is_better_higher(&self) -> bool {
        self.is_significant && self.treatment_mean > self.control_mean
    }
}

// ============================================================================
// ExperimentResult
// ============================================================================

/// Complete results for an experiment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentResult {
    /// ID of the experiment
    pub experiment_id: String,
    /// Name of the experiment
    pub experiment_name: String,
    /// Current status of the experiment
    pub status: ExperimentStatus,
    /// Duration of the experiment in hours (if started)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_hours: Option<f64>,
    /// Total number of requests across all variants
    pub total_requests: u64,
    /// Metrics for each variant
    pub variant_metrics: Vec<VariantMetrics>,
    /// Statistical significance tests
    pub significance_tests: Vec<StatisticalSignificance>,
    /// ID of the winning variant (if determined)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub winner_variant_id: Option<String>,
    /// Recommendation based on results
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recommendation: Option<String>,
}

impl ExperimentResult {
    /// Create a new experiment result
    pub fn new(
        experiment_id: impl Into<String>,
        experiment_name: impl Into<String>,
        status: ExperimentStatus,
    ) -> Self {
        Self {
            experiment_id: experiment_id.into(),
            experiment_name: experiment_name.into(),
            status,
            duration_hours: None,
            total_requests: 0,
            variant_metrics: Vec::new(),
            significance_tests: Vec::new(),
            winner_variant_id: None,
            recommendation: None,
        }
    }

    /// Set the duration
    pub fn with_duration_hours(mut self, hours: f64) -> Self {
        self.duration_hours = Some(hours);
        self
    }

    /// Add variant metrics
    pub fn with_variant_metrics(mut self, metrics: VariantMetrics) -> Self {
        self.total_requests += metrics.total_requests;
        self.variant_metrics.push(metrics);
        self
    }

    /// Add a significance test result
    pub fn with_significance_test(mut self, test: StatisticalSignificance) -> Self {
        self.significance_tests.push(test);
        self
    }

    /// Set the winner
    pub fn with_winner(mut self, variant_id: impl Into<String>) -> Self {
        self.winner_variant_id = Some(variant_id.into());
        self
    }

    /// Set the recommendation
    pub fn with_recommendation(mut self, recommendation: impl Into<String>) -> Self {
        self.recommendation = Some(recommendation.into());
        self
    }

    /// Get metrics for a specific variant
    pub fn get_variant_metrics(&self, variant_id: &str) -> Option<&VariantMetrics> {
        self.variant_metrics
            .iter()
            .find(|m| m.variant_id == variant_id)
    }

    /// Check if any significance test shows a significant result
    pub fn has_significant_result(&self) -> bool {
        self.significance_tests.iter().any(|t| t.is_significant)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod latency_stats_tests {
        use super::*;

        #[test]
        fn test_empty_samples() {
            let stats = LatencyStats::from_samples(vec![]);
            assert_eq!(stats.avg_ms, 0.0);
            assert_eq!(stats.min_ms, 0);
            assert_eq!(stats.max_ms, 0);
        }

        #[test]
        fn test_single_sample() {
            let stats = LatencyStats::from_samples(vec![100]);
            assert_eq!(stats.avg_ms, 100.0);
            assert_eq!(stats.min_ms, 100);
            assert_eq!(stats.max_ms, 100);
            assert_eq!(stats.p50_ms, 100);
        }

        #[test]
        fn test_multiple_samples() {
            let stats = LatencyStats::from_samples(vec![100, 200, 300, 400, 500]);
            assert_eq!(stats.avg_ms, 300.0);
            assert_eq!(stats.min_ms, 100);
            assert_eq!(stats.max_ms, 500);
            assert_eq!(stats.p50_ms, 300);
        }

        #[test]
        fn test_unsorted_samples() {
            let stats = LatencyStats::from_samples(vec![500, 100, 300, 200, 400]);
            assert_eq!(stats.min_ms, 100);
            assert_eq!(stats.max_ms, 500);
        }
    }

    mod variant_metrics_tests {
        use super::*;

        #[test]
        fn test_new_metrics() {
            let metrics = VariantMetrics::new("control", "Control Group");
            assert_eq!(metrics.variant_id, "control");
            assert_eq!(metrics.variant_name, "Control Group");
            assert_eq!(metrics.total_requests, 0);
        }

        #[test]
        fn test_add_successful_record() {
            let mut metrics = VariantMetrics::new("control", "Control");
            let record = ExperimentRecord::new("rec-1", "exp-1", "control", "api-1")
                .with_tokens(100, 50)
                .with_cost_micros(1500);

            metrics.add_record(&record);

            assert_eq!(metrics.total_requests, 1);
            assert_eq!(metrics.successful_requests, 1);
            assert_eq!(metrics.failed_requests, 0);
            assert_eq!(metrics.success_rate, 1.0);
            assert_eq!(metrics.total_input_tokens, 100);
            assert_eq!(metrics.total_output_tokens, 50);
            assert_eq!(metrics.total_cost_micros, 1500);
        }

        #[test]
        fn test_add_failed_record() {
            let mut metrics = VariantMetrics::new("control", "Control");
            let record = ExperimentRecord::new("rec-1", "exp-1", "control", "api-1")
                .with_error("Timeout");

            metrics.add_record(&record);

            assert_eq!(metrics.total_requests, 1);
            assert_eq!(metrics.successful_requests, 0);
            assert_eq!(metrics.failed_requests, 1);
            assert_eq!(metrics.success_rate, 0.0);
        }

        #[test]
        fn test_multiple_records() {
            let mut metrics = VariantMetrics::new("control", "Control");

            for i in 0..10 {
                let mut record =
                    ExperimentRecord::new(format!("rec-{}", i), "exp-1", "control", "api-1")
                        .with_tokens(100, 50)
                        .with_cost_micros(1000);

                if i >= 8 {
                    record = record.with_error("Error");
                }

                metrics.add_record(&record);
            }

            assert_eq!(metrics.total_requests, 10);
            assert_eq!(metrics.successful_requests, 8);
            assert_eq!(metrics.failed_requests, 2);
            assert!((metrics.success_rate - 0.8).abs() < 0.001);
        }
    }

    mod statistical_significance_tests {
        use super::*;

        #[test]
        fn test_significant_result() {
            let sig = StatisticalSignificance::new(
                0.01, // p < 0.05, so significant
                0.95,
                "control",
                "treatment",
                "latency_ms",
                200.0,
                150.0,
            );

            assert!(sig.is_significant);
            assert!(sig.treatment_is_better_lower());
            assert!(!sig.treatment_is_better_higher());
        }

        #[test]
        fn test_non_significant_result() {
            let sig = StatisticalSignificance::new(
                0.15, // p > 0.05, not significant
                0.95,
                "control",
                "treatment",
                "latency_ms",
                200.0,
                195.0,
            );

            assert!(!sig.is_significant);
        }

        #[test]
        fn test_relative_change() {
            let sig = StatisticalSignificance::new(
                0.01, 0.95, "control", "treatment", "latency_ms", 200.0, 150.0,
            );

            // (150 - 200) / 200 * 100 = -25%
            assert!((sig.relative_change - (-25.0)).abs() < 0.001);
        }
    }

    mod experiment_result_tests {
        use super::*;

        #[test]
        fn test_new_result() {
            let result = ExperimentResult::new("exp-1", "Test Experiment", ExperimentStatus::Active);

            assert_eq!(result.experiment_id, "exp-1");
            assert_eq!(result.experiment_name, "Test Experiment");
            assert_eq!(result.status, ExperimentStatus::Active);
            assert_eq!(result.total_requests, 0);
        }

        #[test]
        fn test_result_with_metrics() {
            let mut metrics = VariantMetrics::new("control", "Control");
            metrics.total_requests = 100;

            let result =
                ExperimentResult::new("exp-1", "Test", ExperimentStatus::Active)
                    .with_variant_metrics(metrics);

            assert_eq!(result.total_requests, 100);
            assert_eq!(result.variant_metrics.len(), 1);
        }

        #[test]
        fn test_has_significant_result() {
            let sig = StatisticalSignificance::new(
                0.01, 0.95, "control", "treatment", "latency_ms", 200.0, 150.0,
            );

            let result =
                ExperimentResult::new("exp-1", "Test", ExperimentStatus::Completed)
                    .with_significance_test(sig);

            assert!(result.has_significant_result());
        }
    }
}
