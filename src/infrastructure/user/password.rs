//! Password hashing utilities using Argon2

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher as Argon2PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use std::fmt::Debug;

use crate::domain::DomainError;

/// Trait for password hashing operations
pub trait PasswordHasher: Send + Sync + Debug {
    /// Hash a password
    fn hash(&self, password: &str) -> Result<String, DomainError>;

    /// Verify a password against a hash
    fn verify(&self, password: &str, hash: &str) -> bool;
}

/// Argon2-based password hasher
#[derive(Debug, Clone, Default)]
pub struct Argon2Hasher;

impl Argon2Hasher {
    /// Create a new Argon2 hasher
    pub fn new() -> Self {
        Self
    }
}

impl PasswordHasher for Argon2Hasher {
    fn hash(&self, password: &str) -> Result<String, DomainError> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();

        argon2
            .hash_password(password.as_bytes(), &salt)
            .map(|hash| hash.to_string())
            .map_err(|e| DomainError::validation(format!("Failed to hash password: {}", e)))
    }

    fn verify(&self, password: &str, hash: &str) -> bool {
        let parsed_hash = match PasswordHash::new(hash) {
            Ok(h) => h,
            Err(_) => return false,
        };

        Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_and_verify() {
        let hasher = Argon2Hasher::new();
        let password = "my_secure_password";

        let hash = hasher.hash(password).unwrap();

        assert!(hasher.verify(password, &hash));
        assert!(!hasher.verify("wrong_password", &hash));
    }

    #[test]
    fn test_hash_is_unique() {
        let hasher = Argon2Hasher::new();
        let password = "my_secure_password";

        let hash1 = hasher.hash(password).unwrap();
        let hash2 = hasher.hash(password).unwrap();

        // Hashes should be different due to random salt
        assert_ne!(hash1, hash2);

        // But both should verify correctly
        assert!(hasher.verify(password, &hash1));
        assert!(hasher.verify(password, &hash2));
    }

    #[test]
    fn test_verify_invalid_hash() {
        let hasher = Argon2Hasher::new();

        assert!(!hasher.verify("password", "invalid_hash_format"));
        assert!(!hasher.verify("password", ""));
    }

    #[test]
    fn test_empty_password() {
        let hasher = Argon2Hasher::new();
        let password = "";

        let hash = hasher.hash(password).unwrap();
        assert!(hasher.verify(password, &hash));
    }
}
