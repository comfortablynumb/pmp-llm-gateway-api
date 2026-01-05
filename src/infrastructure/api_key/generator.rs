//! API Key generation
//!
//! Generates cryptographically secure API keys with hashing.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::RngCore;
use sha2::{Digest, Sha256};

/// Result of generating a new API key
#[derive(Debug, Clone)]
pub struct GeneratedApiKey {
    /// The full API key (only shown once at creation)
    pub key: String,
    /// The key prefix for identification
    pub prefix: String,
    /// The hashed key for storage
    pub hash: String,
}

/// Generator for secure API keys
#[derive(Debug, Clone)]
pub struct ApiKeyGenerator {
    /// Prefix for all generated keys (e.g., "pk_live_", "pk_test_")
    prefix: String,
    /// Number of random bytes to generate
    key_bytes: usize,
}

impl ApiKeyGenerator {
    /// Create a new API key generator
    pub fn new(prefix: impl Into<String>) -> Self {
        Self {
            prefix: prefix.into(),
            key_bytes: 32,
        }
    }

    /// Create a generator for production keys
    pub fn production() -> Self {
        Self::new("pk_live_")
    }

    /// Create a generator for test keys
    pub fn test() -> Self {
        Self::new("pk_test_")
    }

    /// Set the number of random bytes
    pub fn with_key_bytes(mut self, bytes: usize) -> Self {
        self.key_bytes = bytes;
        self
    }

    /// Generate a new API key
    pub fn generate(&self) -> GeneratedApiKey {
        let mut random_bytes = vec![0u8; self.key_bytes];
        rand::thread_rng().fill_bytes(&mut random_bytes);

        let encoded = URL_SAFE_NO_PAD.encode(&random_bytes);
        let key = format!("{}{}", self.prefix, encoded);

        // The unique prefix includes type prefix + first 8 chars of random portion
        let unique_prefix = format!("{}{}", self.prefix, &encoded[..8.min(encoded.len())]);

        let hash = self.hash_key(&key);

        GeneratedApiKey {
            key,
            prefix: unique_prefix,
            hash,
        }
    }

    /// Generate a key from a known secret (for testing purposes)
    ///
    /// This allows creating deterministic keys for integration testing.
    pub fn from_secret(&self, secret: &str) -> GeneratedApiKey {
        let key = format!("{}{}", self.prefix, secret);

        // Extract prefix from the secret portion
        let secret_prefix = &secret[..8.min(secret.len())];
        let unique_prefix = format!("{}{}", self.prefix, secret_prefix);

        let hash = self.hash_key(&key);

        GeneratedApiKey {
            key,
            prefix: unique_prefix,
            hash,
        }
    }

    /// Hash an API key for storage
    pub fn hash_key(&self, key: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(key.as_bytes());
        let result = hasher.finalize();
        format!("sha256${}", URL_SAFE_NO_PAD.encode(result))
    }

    /// Verify an API key against a stored hash
    pub fn verify_key(&self, key: &str, stored_hash: &str) -> bool {
        let computed_hash = self.hash_key(key);
        constant_time_compare(&computed_hash, stored_hash)
    }

    /// Extract the unique prefix from a key (type prefix + 8 random chars)
    pub fn extract_prefix(key: &str) -> Option<&str> {
        // Find the last underscore in the prefix part
        if let Some(pos) = key.find('_') {
            // Check for patterns like "pk_live_" or "pk_test_"
            if let Some(second_pos) = key[pos + 1..].find('_') {
                let type_prefix_end = pos + 1 + second_pos + 1;

                // Add 8 chars of random portion to create unique prefix
                let unique_prefix_end = (type_prefix_end + 8).min(key.len());
                return Some(&key[..unique_prefix_end]);
            }

            return Some(&key[..pos + 1]);
        }

        None
    }
}

/// Constant-time string comparison to prevent timing attacks
fn constant_time_compare(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }

    let a_bytes = a.as_bytes();
    let b_bytes = b.as_bytes();

    let mut result = 0u8;

    for i in 0..a.len() {
        result |= a_bytes[i] ^ b_bytes[i];
    }

    result == 0
}

impl Default for ApiKeyGenerator {
    fn default() -> Self {
        Self::production()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_key() {
        let generator = ApiKeyGenerator::production();
        let generated = generator.generate();

        assert!(generated.key.starts_with("pk_live_"));
        assert!(generated.prefix.starts_with("pk_live_"));
        // Unique prefix is type prefix + 8 random chars
        assert_eq!(generated.prefix.len(), "pk_live_".len() + 8);
        assert!(generated.hash.starts_with("sha256$"));
    }

    #[test]
    fn test_generate_test_key() {
        let generator = ApiKeyGenerator::test();
        let generated = generator.generate();

        assert!(generated.key.starts_with("pk_test_"));
        assert!(generated.prefix.starts_with("pk_test_"));
        assert_eq!(generated.prefix.len(), "pk_test_".len() + 8);
    }

    #[test]
    fn test_generate_custom_prefix() {
        let generator = ApiKeyGenerator::new("custom_");
        let generated = generator.generate();

        assert!(generated.key.starts_with("custom_"));
    }

    #[test]
    fn test_key_uniqueness() {
        let generator = ApiKeyGenerator::production();
        let key1 = generator.generate();
        let key2 = generator.generate();

        assert_ne!(key1.key, key2.key);
        assert_ne!(key1.hash, key2.hash);
    }

    #[test]
    fn test_key_length() {
        let generator = ApiKeyGenerator::production();
        let generated = generator.generate();

        // 32 bytes base64-encoded = 43 chars, plus prefix
        assert!(generated.key.len() > 40);
    }

    #[test]
    fn test_verify_key() {
        let generator = ApiKeyGenerator::production();
        let generated = generator.generate();

        assert!(generator.verify_key(&generated.key, &generated.hash));
        assert!(!generator.verify_key("wrong_key", &generated.hash));
    }

    #[test]
    fn test_hash_deterministic() {
        let generator = ApiKeyGenerator::production();
        let key = "pk_live_test123";

        let hash1 = generator.hash_key(key);
        let hash2 = generator.hash_key(key);

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_extract_prefix() {
        // Full key with enough random chars - extracts type prefix + 8 chars
        assert_eq!(
            ApiKeyGenerator::extract_prefix("pk_live_abc12345xyz"),
            Some("pk_live_abc12345")
        );
        assert_eq!(
            ApiKeyGenerator::extract_prefix("pk_test_xyz78901234"),
            Some("pk_test_xyz78901")
        );
        // Single underscore prefix
        assert_eq!(
            ApiKeyGenerator::extract_prefix("custom_key123"),
            Some("custom_")
        );
        // No prefix
        assert_eq!(ApiKeyGenerator::extract_prefix("noprefix"), None);
        // Short key - takes what's available
        assert_eq!(
            ApiKeyGenerator::extract_prefix("pk_test_abc"),
            Some("pk_test_abc")
        );
    }

    #[test]
    fn test_constant_time_compare() {
        assert!(constant_time_compare("hello", "hello"));
        assert!(!constant_time_compare("hello", "world"));
        assert!(!constant_time_compare("hello", "hell"));
    }

    #[test]
    fn test_custom_key_bytes() {
        let generator = ApiKeyGenerator::production().with_key_bytes(64);
        let generated = generator.generate();

        // 64 bytes base64-encoded = 86 chars, plus prefix
        assert!(generated.key.len() > 80);
    }
}
