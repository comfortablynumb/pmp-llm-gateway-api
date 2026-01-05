//! Cache key generation strategies

use std::collections::BTreeMap;
use std::fmt::Debug;
use std::hash::{Hash, Hasher};

use serde::Serialize;

/// Trait for types that can be used as cache keys
pub trait CacheKey: Send + Sync + Debug + Clone {
    /// Returns the string representation of the key
    fn as_str(&self) -> &str;

    /// Returns the key as bytes for hashing
    fn as_bytes(&self) -> &[u8] {
        self.as_str().as_bytes()
    }
}

impl CacheKey for String {
    fn as_str(&self) -> &str {
        self
    }
}

impl CacheKey for &str {
    fn as_str(&self) -> &str {
        self
    }
}

/// Trait for generating cache keys from input data
pub trait CacheKeyGenerator: Send + Sync + Debug {
    /// Generates a cache key from the given components
    fn generate(&self, params: &CacheKeyParams) -> String;

    /// Generates a key with a namespace prefix
    fn generate_with_namespace(&self, namespace: &str, params: &CacheKeyParams) -> String {
        format!("{}:{}", namespace, self.generate(params))
    }
}

/// Parameters for cache key generation
#[derive(Debug, Clone, Default)]
pub struct CacheKeyParams {
    /// Primary identifier (e.g., model ID, prompt ID)
    pub primary: String,
    /// Secondary components (sorted for consistency)
    pub components: BTreeMap<String, String>,
}

impl CacheKeyParams {
    /// Creates new cache key parameters with a primary identifier
    pub fn new(primary: impl Into<String>) -> Self {
        Self {
            primary: primary.into(),
            components: BTreeMap::new(),
        }
    }

    /// Adds a component to the key parameters
    pub fn with_component(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.components.insert(key.into(), value.into());
        self
    }

    /// Creates parameters from a serializable value
    pub fn from_serializable<T: Serialize>(value: &T) -> Result<Self, serde_json::Error> {
        let json = serde_json::to_string(value)?;
        Ok(Self::new(json))
    }
}

/// Default cache key generator using hash-based keys
#[derive(Debug, Clone, Default)]
pub struct DefaultKeyGenerator {
    /// Whether to use short hash keys (8 chars) or full keys
    use_short_hash: bool,
}

impl DefaultKeyGenerator {
    /// Creates a new default key generator
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a generator that produces short hash keys
    pub fn with_short_hash(mut self) -> Self {
        self.use_short_hash = true;
        self
    }

    fn hash_string(input: &str) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        input.hash(&mut hasher);
        hasher.finish()
    }
}

impl CacheKeyGenerator for DefaultKeyGenerator {
    fn generate(&self, params: &CacheKeyParams) -> String {
        let mut parts = vec![params.primary.clone()];

        for (k, v) in &params.components {
            parts.push(format!("{}={}", k, v));
        }

        let combined = parts.join(":");

        if self.use_short_hash {
            let hash = Self::hash_string(&combined);
            format!("{:016x}", hash)
        } else {
            combined
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_key_params_new() {
        let params = CacheKeyParams::new("test-key");
        assert_eq!(params.primary, "test-key");
        assert!(params.components.is_empty());
    }

    #[test]
    fn test_cache_key_params_with_components() {
        let params = CacheKeyParams::new("model-1")
            .with_component("temperature", "0.7")
            .with_component("max_tokens", "100");

        assert_eq!(params.primary, "model-1");
        assert_eq!(params.components.len(), 2);
        assert_eq!(params.components.get("temperature"), Some(&"0.7".to_string()));
    }

    #[test]
    fn test_default_key_generator() {
        let generator = DefaultKeyGenerator::new();
        let params = CacheKeyParams::new("test")
            .with_component("a", "1")
            .with_component("b", "2");

        let key = generator.generate(&params);
        assert_eq!(key, "test:a=1:b=2");
    }

    #[test]
    fn test_default_key_generator_short_hash() {
        let generator = DefaultKeyGenerator::new().with_short_hash();
        let params = CacheKeyParams::new("test")
            .with_component("a", "1");

        let key = generator.generate(&params);
        assert_eq!(key.len(), 16); // 16 hex chars
    }

    #[test]
    fn test_generate_with_namespace() {
        let generator = DefaultKeyGenerator::new();
        let params = CacheKeyParams::new("user-123");

        let key = generator.generate_with_namespace("llm:responses", &params);
        assert_eq!(key, "llm:responses:user-123");
    }

    #[test]
    fn test_components_are_sorted() {
        let generator = DefaultKeyGenerator::new();

        // Add components in random order
        let params = CacheKeyParams::new("test")
            .with_component("zebra", "z")
            .with_component("apple", "a")
            .with_component("mango", "m");

        let key = generator.generate(&params);
        // Should be sorted alphabetically
        assert_eq!(key, "test:apple=a:mango=m:zebra=z");
    }

    #[test]
    fn test_from_serializable() {
        #[derive(Serialize)]
        struct TestData {
            name: String,
            value: i32,
        }

        let data = TestData {
            name: "test".to_string(),
            value: 42,
        };

        let params = CacheKeyParams::from_serializable(&data).unwrap();
        assert!(params.primary.contains("test"));
        assert!(params.primary.contains("42"));
    }
}
