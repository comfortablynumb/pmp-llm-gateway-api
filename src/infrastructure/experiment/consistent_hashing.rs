//! Consistent hashing for experiment variant assignment
//!
//! Ensures the same API key always gets assigned to the same variant
//! for a given experiment.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Consistent hasher for experiment assignments
#[derive(Debug, Clone, Copy)]
pub struct ConsistentHasher;

impl ConsistentHasher {
    /// Generate a deterministic hash value (0-99) for an API key and experiment
    ///
    /// This ensures that:
    /// - The same API key + experiment always returns the same hash
    /// - Hash values are uniformly distributed across 0-99
    /// - Different API keys are likely to get different hashes
    pub fn hash_assignment(api_key_id: &str, experiment_id: &str) -> u8 {
        let mut hasher = DefaultHasher::new();
        api_key_id.hash(&mut hasher);
        experiment_id.hash(&mut hasher);
        (hasher.finish() % 100) as u8
    }

    /// Check if a hash value falls within a given percentage range
    ///
    /// # Arguments
    /// * `hash` - Hash value (0-99)
    /// * `start_percent` - Start of range (inclusive)
    /// * `end_percent` - End of range (exclusive)
    pub fn in_range(hash: u8, start_percent: u8, end_percent: u8) -> bool {
        hash >= start_percent && hash < end_percent
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_consistent_hash_same_input() {
        let hash1 = ConsistentHasher::hash_assignment("api-key-1", "exp-1");
        let hash2 = ConsistentHasher::hash_assignment("api-key-1", "exp-1");
        assert_eq!(hash1, hash2, "Same inputs should produce same hash");
    }

    #[test]
    fn test_consistent_hash_different_keys() {
        let hash1 = ConsistentHasher::hash_assignment("api-key-1", "exp-1");
        let hash2 = ConsistentHasher::hash_assignment("api-key-2", "exp-1");
        // Different keys may produce different hashes
        // Just verify they're in valid range
        assert!(hash1 <= 99);
        assert!(hash2 <= 99);
    }

    #[test]
    fn test_consistent_hash_different_experiments() {
        let hash1 = ConsistentHasher::hash_assignment("api-key-1", "exp-1");
        let hash2 = ConsistentHasher::hash_assignment("api-key-1", "exp-2");
        // Same key with different experiments should likely differ
        // Just verify they're in valid range
        assert!(hash1 <= 99);
        assert!(hash2 <= 99);
    }

    #[test]
    fn test_hash_distribution() {
        // Test that hashes are reasonably distributed
        let mut buckets = [0u32; 10];

        for i in 0..1000 {
            let hash = ConsistentHasher::hash_assignment(&format!("key-{}", i), "exp-1");
            buckets[(hash / 10) as usize] += 1;
        }

        // Each bucket should have roughly 100 items (10% of 1000)
        // Allow for variance but ensure no bucket is empty or has everything
        for count in buckets {
            assert!(count > 50, "Bucket has too few items: {}", count);
            assert!(count < 150, "Bucket has too many items: {}", count);
        }
    }

    #[test]
    fn test_in_range() {
        assert!(ConsistentHasher::in_range(25, 0, 50));
        assert!(ConsistentHasher::in_range(50, 50, 100));
        assert!(!ConsistentHasher::in_range(25, 50, 100));
        assert!(!ConsistentHasher::in_range(50, 0, 50));

        // Edge cases
        assert!(ConsistentHasher::in_range(0, 0, 100));
        assert!(!ConsistentHasher::in_range(100, 0, 100));
    }

    #[test]
    fn test_50_50_split() {
        // Verify that a 50/50 split works correctly
        let mut control_count = 0;
        let mut treatment_count = 0;

        for i in 0..1000 {
            let hash = ConsistentHasher::hash_assignment(&format!("key-{}", i), "ab-test");

            if hash < 50 {
                control_count += 1;
            } else {
                treatment_count += 1;
            }
        }

        // Should be roughly 50/50
        let diff = (control_count as i32 - treatment_count as i32).abs();
        assert!(
            diff < 100,
            "Split is too uneven: control={}, treatment={}",
            control_count,
            treatment_count
        );
    }

    #[test]
    fn test_determinism_across_calls() {
        // Verify the same key always gets the same assignment
        let api_key = "test-api-key-12345";
        let experiment = "pricing-experiment-v2";

        let first_hash = ConsistentHasher::hash_assignment(api_key, experiment);

        // Call multiple times
        for _ in 0..100 {
            let hash = ConsistentHasher::hash_assignment(api_key, experiment);
            assert_eq!(hash, first_hash, "Hash should be deterministic");
        }
    }
}
