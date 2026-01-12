//! Statistical analysis functions for A/B testing
//!
//! Provides statistical significance testing using Welch's t-test.

use crate::domain::experiment::StatisticalSignificance;

/// Calculate p-value using Welch's t-test for two independent samples
///
/// Welch's t-test is preferred over Student's t-test when the two samples
/// may have unequal variances and/or unequal sample sizes.
///
/// # Arguments
/// * `sample1` - First sample (control group)
/// * `sample2` - Second sample (treatment group)
///
/// # Returns
/// * `Some(p_value)` if calculation succeeds
/// * `None` if either sample has fewer than 2 elements
pub fn welch_t_test(sample1: &[f64], sample2: &[f64]) -> Option<f64> {
    if sample1.len() < 2 || sample2.len() < 2 {
        return None;
    }

    let n1 = sample1.len() as f64;
    let n2 = sample2.len() as f64;

    let mean1 = sample1.iter().sum::<f64>() / n1;
    let mean2 = sample2.iter().sum::<f64>() / n2;

    let var1 = sample1
        .iter()
        .map(|x| (x - mean1).powi(2))
        .sum::<f64>()
        / (n1 - 1.0);
    let var2 = sample2
        .iter()
        .map(|x| (x - mean2).powi(2))
        .sum::<f64>()
        / (n2 - 1.0);

    let se = ((var1 / n1) + (var2 / n2)).sqrt();

    if se == 0.0 {
        return None;
    }

    let t = (mean1 - mean2) / se;

    // Welch-Satterthwaite degrees of freedom
    let df_num = (var1 / n1 + var2 / n2).powi(2);
    let df_denom = ((var1 / n1).powi(2) / (n1 - 1.0)) + ((var2 / n2).powi(2) / (n2 - 1.0));

    if df_denom == 0.0 {
        return None;
    }

    let df = df_num / df_denom;

    Some(approximate_p_value(t.abs(), df))
}

/// Calculate mean of a sample
pub fn mean(sample: &[f64]) -> f64 {
    if sample.is_empty() {
        return 0.0;
    }
    sample.iter().sum::<f64>() / sample.len() as f64
}

/// Calculate variance of a sample (sample variance, n-1 denominator)
pub fn variance(sample: &[f64]) -> f64 {
    if sample.len() < 2 {
        return 0.0;
    }

    let m = mean(sample);
    let n = sample.len() as f64;
    sample.iter().map(|x| (x - m).powi(2)).sum::<f64>() / (n - 1.0)
}

/// Calculate standard deviation of a sample
pub fn std_dev(sample: &[f64]) -> f64 {
    variance(sample).sqrt()
}

/// Approximate p-value from t-statistic and degrees of freedom
///
/// Uses normal approximation for large df, and a rough approximation
/// for smaller df. For production use, consider using a proper
/// statistics crate like `statrs`.
fn approximate_p_value(t: f64, df: f64) -> f64 {
    // Two-tailed p-value
    if df > 30.0 {
        // For large df, use normal approximation
        2.0 * (1.0 - normal_cdf(t))
    } else {
        // Rough approximation for smaller df using a correction factor
        let correction = 1.0 - 1.0 / (4.0 * df);
        2.0 * (1.0 - normal_cdf(t * correction.sqrt()))
    }
}

/// Standard normal cumulative distribution function
fn normal_cdf(x: f64) -> f64 {
    0.5 * (1.0 + erf(x / std::f64::consts::SQRT_2))
}

/// Error function approximation
///
/// Uses Horner's method for the polynomial approximation.
/// Accurate to about 1.5e-7.
fn erf(x: f64) -> f64 {
    // Coefficients for the approximation
    let a1 = 0.254829592;
    let a2 = -0.284496736;
    let a3 = 1.421413741;
    let a4 = -1.453152027;
    let a5 = 1.061405429;
    let p = 0.3275911;

    let sign = if x < 0.0 { -1.0 } else { 1.0 };
    let x = x.abs();

    let t = 1.0 / (1.0 + p * x);
    let y = 1.0 - (((((a5 * t + a4) * t) + a3) * t + a2) * t + a1) * t * (-x * x).exp();

    sign * y
}

/// Calculate statistical significance for a metric comparison
///
/// # Arguments
/// * `control_samples` - Metric values from the control group
/// * `treatment_samples` - Metric values from the treatment group
/// * `control_id` - ID of the control variant
/// * `treatment_id` - ID of the treatment variant
/// * `metric` - Name of the metric being compared
/// * `confidence_level` - Required confidence level (e.g., 0.95 for 95%)
///
/// # Returns
/// * `Some(StatisticalSignificance)` with results if calculation succeeds
/// * `None` if samples are too small
pub fn calculate_significance(
    control_samples: &[f64],
    treatment_samples: &[f64],
    control_id: &str,
    treatment_id: &str,
    metric: &str,
    confidence_level: f64,
) -> Option<StatisticalSignificance> {
    let p_value = welch_t_test(control_samples, treatment_samples)?;

    let control_mean = mean(control_samples);
    let treatment_mean = mean(treatment_samples);

    Some(StatisticalSignificance::new(
        p_value,
        confidence_level,
        control_id,
        treatment_id,
        metric,
        control_mean,
        treatment_mean,
    ))
}

/// Calculate multiple significance tests for common metrics
pub fn calculate_all_significance(
    control_latencies: &[f64],
    treatment_latencies: &[f64],
    control_costs: &[f64],
    treatment_costs: &[f64],
    control_id: &str,
    treatment_id: &str,
    confidence_level: f64,
) -> Vec<StatisticalSignificance> {
    let mut results = Vec::new();

    if let Some(latency_sig) = calculate_significance(
        control_latencies,
        treatment_latencies,
        control_id,
        treatment_id,
        "latency_ms",
        confidence_level,
    ) {
        results.push(latency_sig);
    }

    if let Some(cost_sig) = calculate_significance(
        control_costs,
        treatment_costs,
        control_id,
        treatment_id,
        "cost_micros",
        confidence_level,
    ) {
        results.push(cost_sig);
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mean() {
        assert_eq!(mean(&[1.0, 2.0, 3.0, 4.0, 5.0]), 3.0);
        assert_eq!(mean(&[]), 0.0);
        assert_eq!(mean(&[42.0]), 42.0);
    }

    #[test]
    fn test_variance() {
        // Variance of [1, 2, 3, 4, 5] = 2.5 (sample variance)
        let var = variance(&[1.0, 2.0, 3.0, 4.0, 5.0]);
        assert!((var - 2.5).abs() < 0.001);

        assert_eq!(variance(&[]), 0.0);
        assert_eq!(variance(&[42.0]), 0.0);
    }

    #[test]
    fn test_std_dev() {
        let sd = std_dev(&[1.0, 2.0, 3.0, 4.0, 5.0]);
        assert!((sd - 1.5811).abs() < 0.001);
    }

    #[test]
    fn test_welch_t_test_insufficient_samples() {
        // Not enough samples
        assert!(welch_t_test(&[], &[1.0, 2.0]).is_none());
        assert!(welch_t_test(&[1.0], &[1.0, 2.0]).is_none());
        assert!(welch_t_test(&[1.0, 2.0], &[]).is_none());
        assert!(welch_t_test(&[1.0, 2.0], &[1.0]).is_none());
    }

    #[test]
    fn test_welch_t_test_identical_samples() {
        // Identical samples should give high p-value (not significant)
        let sample = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let p_value = welch_t_test(&sample, &sample.clone());

        // P-value should be 1.0 for identical samples (no difference)
        // Note: might be NaN if variance is 0, so we check for that
        if let Some(p) = p_value {
            assert!(p > 0.9 || p.is_nan(), "Identical samples should have high p-value");
        }
    }

    #[test]
    fn test_welch_t_test_significantly_different() {
        // Very different samples should give low p-value
        let control = vec![100.0, 102.0, 98.0, 101.0, 99.0, 100.0, 101.0, 99.0, 100.0, 100.0];
        let treatment = vec![150.0, 152.0, 148.0, 151.0, 149.0, 150.0, 151.0, 149.0, 150.0, 150.0];

        let p_value = welch_t_test(&control, &treatment).unwrap();
        assert!(
            p_value < 0.01,
            "Significantly different samples should have low p-value, got {}",
            p_value
        );
    }

    #[test]
    fn test_welch_t_test_similar_samples() {
        // Similar samples should give high p-value
        let control = vec![100.0, 102.0, 98.0, 101.0, 99.0];
        let treatment = vec![101.0, 99.0, 100.0, 102.0, 98.0];

        let p_value = welch_t_test(&control, &treatment).unwrap();
        assert!(
            p_value > 0.5,
            "Similar samples should have high p-value, got {}",
            p_value
        );
    }

    #[test]
    fn test_normal_cdf() {
        // Known values
        assert!((normal_cdf(0.0) - 0.5).abs() < 0.001);
        // normal_cdf(3.0) ≈ 0.9987 (actual standard normal table value is ~0.99865)
        assert!(normal_cdf(3.0) > 0.998);
        assert!(normal_cdf(-3.0) < 0.002);
    }

    #[test]
    fn test_erf() {
        // Known values
        assert!((erf(0.0)).abs() < 0.001);
        assert!(erf(3.0) > 0.999);
        assert!(erf(-3.0) < -0.999);
    }

    #[test]
    fn test_calculate_significance() {
        let control = vec![100.0, 110.0, 105.0, 95.0, 100.0, 105.0, 100.0, 95.0, 110.0, 100.0];
        let treatment = vec![80.0, 85.0, 90.0, 75.0, 85.0, 80.0, 90.0, 85.0, 80.0, 85.0];

        let result = calculate_significance(
            &control,
            &treatment,
            "control",
            "treatment",
            "latency_ms",
            0.95,
        );

        assert!(result.is_some());
        let sig = result.unwrap();

        assert_eq!(sig.control_variant_id, "control");
        assert_eq!(sig.treatment_variant_id, "treatment");
        assert_eq!(sig.metric, "latency_ms");
        assert!(sig.p_value >= 0.0 && sig.p_value <= 1.0);

        // Treatment has lower latency (better)
        assert!(sig.treatment_mean < sig.control_mean);
    }

    #[test]
    fn test_calculate_all_significance() {
        let control_latencies = vec![100.0, 110.0, 105.0, 95.0, 100.0];
        let treatment_latencies = vec![80.0, 85.0, 90.0, 75.0, 85.0];
        let control_costs = vec![10.0, 11.0, 10.5, 9.5, 10.0];
        let treatment_costs = vec![12.0, 13.0, 12.5, 11.5, 12.0];

        let results = calculate_all_significance(
            &control_latencies,
            &treatment_latencies,
            &control_costs,
            &treatment_costs,
            "control",
            "treatment",
            0.95,
        );

        assert_eq!(results.len(), 2);
        assert!(results.iter().any(|s| s.metric == "latency_ms"));
        assert!(results.iter().any(|s| s.metric == "cost_micros"));
    }
}
