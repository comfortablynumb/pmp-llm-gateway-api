//! JWT token generation and validation with JWKS support

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use rsa::pkcs1::EncodeRsaPrivateKey;
use rsa::{BigUint, RsaPrivateKey};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

use crate::domain::user::User;
use crate::domain::DomainError;
use rsa::pkcs1::EncodeRsaPublicKey;

/// JWT claims structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtClaims {
    /// Subject (user ID)
    pub sub: String,
    /// Username
    pub username: String,
    /// Issued at timestamp (Unix epoch)
    pub iat: i64,
    /// Expiration timestamp (Unix epoch)
    pub exp: i64,
}

impl JwtClaims {
    /// Create new claims for a user
    pub fn new(user: &User, expiration_hours: u64) -> Self {
        let now = Utc::now();
        let exp = now + Duration::hours(expiration_hours as i64);

        Self {
            sub: user.id().as_str().to_string(),
            username: user.username().to_string(),
            iat: now.timestamp(),
            exp: exp.timestamp(),
        }
    }

    /// Check if the token has expired
    pub fn is_expired(&self) -> bool {
        Utc::now().timestamp() >= self.exp
    }

    /// Get user ID from claims
    pub fn user_id(&self) -> &str {
        &self.sub
    }
}

/// Configuration for JWT service
#[derive(Debug, Clone)]
pub struct JwtConfig {
    /// Secret key for signing tokens (used when JWKS is not available)
    pub secret: String,
    /// Token expiration time in hours
    pub expiration_hours: u64,
}

impl JwtConfig {
    /// Create new JWT configuration
    pub fn new(secret: impl Into<String>, expiration_hours: u64) -> Self {
        Self {
            secret: secret.into(),
            expiration_hours,
        }
    }
}

impl Default for JwtConfig {
    fn default() -> Self {
        Self {
            secret: "change-me-in-production".to_string(),
            expiration_hours: 24,
        }
    }
}

/// Trait for JWT operations
pub trait JwtGenerator: Send + Sync + Debug {
    /// Generate a JWT token for a user
    fn generate(&self, user: &User) -> Result<String, DomainError>;

    /// Validate a JWT token and return the claims
    fn validate(&self, token: &str) -> Result<JwtClaims, DomainError>;

    /// Get the token expiration time in hours
    fn expiration_hours(&self) -> u64;
}

/// JWT service implementation using simple secret
#[derive(Clone)]
pub struct JwtService {
    config: JwtConfig,
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
}

impl Debug for JwtService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JwtService")
            .field("config", &self.config)
            .field("encoding_key", &"[hidden]")
            .field("decoding_key", &"[hidden]")
            .finish()
    }
}

impl JwtService {
    /// Create a new JWT service with the given configuration
    pub fn new(config: JwtConfig) -> Self {
        let encoding_key = EncodingKey::from_secret(config.secret.as_bytes());
        let decoding_key = DecodingKey::from_secret(config.secret.as_bytes());

        Self {
            config,
            encoding_key,
            decoding_key,
        }
    }

    /// Create a JWT service with default configuration
    pub fn with_default_config() -> Self {
        Self::new(JwtConfig::default())
    }
}

impl JwtGenerator for JwtService {
    fn generate(&self, user: &User) -> Result<String, DomainError> {
        let claims = JwtClaims::new(user, self.config.expiration_hours);

        encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|e| DomainError::validation(format!("Failed to generate JWT: {}", e)))
    }

    fn validate(&self, token: &str) -> Result<JwtClaims, DomainError> {
        let validation = Validation::default();

        let token_data = decode::<JwtClaims>(token, &self.decoding_key, &validation)
            .map_err(|e| DomainError::validation(format!("Invalid JWT: {}", e)))?;

        Ok(token_data.claims)
    }

    fn expiration_hours(&self) -> u64 {
        self.config.expiration_hours
    }
}

/// JWK key structure for manual parsing (includes private key components)
#[derive(Debug, Clone, Deserialize)]
struct JwkKey {
    /// Key type (RSA, EC, oct)
    kty: String,
    /// Key ID
    kid: Option<String>,
    /// Algorithm
    alg: Option<String>,
    /// RSA modulus (base64url)
    n: Option<String>,
    /// RSA public exponent (base64url)
    e: Option<String>,
    /// RSA private exponent (base64url)
    d: Option<String>,
    /// RSA first prime factor (base64url)
    p: Option<String>,
    /// RSA second prime factor (base64url)
    q: Option<String>,
    /// RSA first factor CRT exponent (base64url)
    dp: Option<String>,
    /// RSA second factor CRT exponent (base64url)
    dq: Option<String>,
    /// RSA first CRT coefficient (base64url)
    qi: Option<String>,
    /// Symmetric key value (base64url) for oct keys
    k: Option<String>,
}

/// JWKS structure for manual parsing
#[derive(Debug, Clone, Deserialize)]
struct Jwks {
    keys: Vec<JwkKey>,
}

/// JWKS-based JWT service for token generation and validation with RSA support
pub struct JwksJwtService {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    algorithm: Algorithm,
    key_id: String,
    expiration_hours: u64,
}

impl Debug for JwksJwtService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JwksJwtService")
            .field("algorithm", &self.algorithm)
            .field("key_id", &self.key_id)
            .field("expiration_hours", &self.expiration_hours)
            .finish()
    }
}

impl Clone for JwksJwtService {
    fn clone(&self) -> Self {
        Self {
            encoding_key: self.encoding_key.clone(),
            decoding_key: self.decoding_key.clone(),
            algorithm: self.algorithm,
            key_id: self.key_id.clone(),
            expiration_hours: self.expiration_hours,
        }
    }
}

impl JwksJwtService {
    /// Create a new JWKS-based JWT service from a JWKS JSON string
    pub fn from_jwks_json(jwks_json: &str, expiration_hours: u64) -> Result<Self, DomainError> {
        let jwks: Jwks = serde_json::from_str(jwks_json)
            .map_err(|e| DomainError::validation(format!("Failed to parse JWKS: {}", e)))?;

        if jwks.keys.is_empty() {
            return Err(DomainError::validation("JWKS contains no keys"));
        }

        // Find first key with a kid
        let key = jwks
            .keys
            .iter()
            .find(|k| k.kid.is_some())
            .or_else(|| jwks.keys.first())
            .ok_or_else(|| DomainError::validation("No suitable key found in JWKS"))?;

        let key_id = key.kid.clone().unwrap_or_else(|| "default".to_string());

        match key.kty.as_str() {
            "RSA" => Self::from_rsa_jwk(key, key_id, expiration_hours),
            "oct" => Self::from_symmetric_jwk(key, key_id, expiration_hours),
            other => Err(DomainError::validation(format!(
                "Unsupported key type: {}. Use RSA or oct.",
                other
            ))),
        }
    }

    /// Create service from RSA JWK
    fn from_rsa_jwk(
        key: &JwkKey,
        key_id: String,
        expiration_hours: u64,
    ) -> Result<Self, DomainError> {
        // Determine algorithm
        let algorithm = match key.alg.as_deref() {
            Some("RS256") | None => Algorithm::RS256,
            Some("RS384") => Algorithm::RS384,
            Some("RS512") => Algorithm::RS512,
            Some(alg) => {
                return Err(DomainError::validation(format!(
                    "Unsupported RSA algorithm: {}",
                    alg
                )))
            }
        };

        // Extract required components
        let n = key
            .n
            .as_ref()
            .ok_or_else(|| DomainError::validation("RSA key missing 'n' (modulus)"))?;
        let e = key
            .e
            .as_ref()
            .ok_or_else(|| DomainError::validation("RSA key missing 'e' (public exponent)"))?;
        let d = key
            .d
            .as_ref()
            .ok_or_else(|| DomainError::validation("RSA key missing 'd' (private exponent)"))?;

        // Build RSA private key
        let private_key = build_rsa_private_key(n, e, d, &key.p, &key.q, &key.dp, &key.dq, &key.qi)?;

        // Convert private key to PKCS#1 PEM format for encoding (signing)
        let private_pem = private_key
            .to_pkcs1_pem(rsa::pkcs1::LineEnding::LF)
            .map_err(|e| DomainError::validation(format!("Failed to encode RSA private key: {}", e)))?;

        // Extract public key and convert to PEM for decoding (verification)
        let public_key = private_key.to_public_key();
        let public_pem = public_key
            .to_pkcs1_pem(rsa::pkcs1::LineEnding::LF)
            .map_err(|e| DomainError::validation(format!("Failed to encode RSA public key: {}", e)))?;

        let encoding_key = EncodingKey::from_rsa_pem(private_pem.as_bytes())
            .map_err(|e| DomainError::validation(format!("Failed to create encoding key: {}", e)))?;

        let decoding_key = DecodingKey::from_rsa_pem(public_pem.as_bytes())
            .map_err(|e| DomainError::validation(format!("Failed to create decoding key: {}", e)))?;

        Ok(Self {
            encoding_key,
            decoding_key,
            algorithm,
            key_id,
            expiration_hours,
        })
    }

    /// Create service from symmetric (oct) JWK
    fn from_symmetric_jwk(
        key: &JwkKey,
        key_id: String,
        expiration_hours: u64,
    ) -> Result<Self, DomainError> {
        let k = key
            .k
            .as_ref()
            .ok_or_else(|| DomainError::validation("Symmetric key missing 'k' value"))?;

        let algorithm = match key.alg.as_deref() {
            Some("HS256") | None => Algorithm::HS256,
            Some("HS384") => Algorithm::HS384,
            Some("HS512") => Algorithm::HS512,
            Some(alg) => {
                return Err(DomainError::validation(format!(
                    "Unsupported symmetric algorithm: {}",
                    alg
                )))
            }
        };

        let secret_bytes = URL_SAFE_NO_PAD
            .decode(k)
            .map_err(|e| DomainError::validation(format!("Invalid base64url in JWK 'k': {}", e)))?;

        let encoding_key = EncodingKey::from_secret(&secret_bytes);
        let decoding_key = DecodingKey::from_secret(&secret_bytes);

        Ok(Self {
            encoding_key,
            decoding_key,
            algorithm,
            key_id,
            expiration_hours,
        })
    }
}

/// Build RSA private key from JWK components
fn build_rsa_private_key(
    n: &str,
    e: &str,
    d: &str,
    p: &Option<String>,
    q: &Option<String>,
    dp: &Option<String>,
    dq: &Option<String>,
    qi: &Option<String>,
) -> Result<RsaPrivateKey, DomainError> {
    let n_bytes = decode_base64url(n, "n")?;
    let e_bytes = decode_base64url(e, "e")?;
    let d_bytes = decode_base64url(d, "d")?;

    let n_uint = BigUint::from_bytes_be(&n_bytes);
    let e_uint = BigUint::from_bytes_be(&e_bytes);
    let d_uint = BigUint::from_bytes_be(&d_bytes);

    // If we have the prime factors, use them for a complete key
    if let (Some(p_str), Some(q_str)) = (p, q) {
        let p_bytes = decode_base64url(p_str, "p")?;
        let q_bytes = decode_base64url(q_str, "q")?;
        let p_uint = BigUint::from_bytes_be(&p_bytes);
        let q_uint = BigUint::from_bytes_be(&q_bytes);

        // Build primes vector
        let primes = vec![p_uint, q_uint];

        // Try to construct with CRT components if available
        if let (Some(dp_str), Some(dq_str), Some(qi_str)) = (dp, dq, qi) {
            let _dp_bytes = decode_base64url(dp_str, "dp")?;
            let _dq_bytes = decode_base64url(dq_str, "dq")?;
            let _qi_bytes = decode_base64url(qi_str, "qi")?;

            // The rsa crate will compute CRT params from primes
        }

        RsaPrivateKey::from_components(n_uint, e_uint, d_uint, primes)
            .map_err(|e| DomainError::validation(format!("Invalid RSA key components: {}", e)))
    } else {
        // Without primes, try to construct from n, e, d only
        // This is less efficient but still works
        RsaPrivateKey::from_components(n_uint, e_uint, d_uint, vec![])
            .map_err(|e| DomainError::validation(format!("Invalid RSA key (missing primes): {}", e)))
    }
}

/// Decode base64url string to bytes
fn decode_base64url(s: &str, field: &str) -> Result<Vec<u8>, DomainError> {
    URL_SAFE_NO_PAD
        .decode(s)
        .map_err(|e| DomainError::validation(format!("Invalid base64url in '{}': {}", field, e)))
}

impl JwtGenerator for JwksJwtService {
    fn generate(&self, user: &User) -> Result<String, DomainError> {
        let claims = JwtClaims::new(user, self.expiration_hours);

        let mut header = Header::new(self.algorithm);
        header.kid = Some(self.key_id.clone());

        encode(&header, &claims, &self.encoding_key)
            .map_err(|e| DomainError::validation(format!("Failed to generate JWT: {}", e)))
    }

    fn validate(&self, token: &str) -> Result<JwtClaims, DomainError> {
        let mut validation = Validation::new(self.algorithm);
        validation.validate_exp = true;

        let token_data = decode::<JwtClaims>(token, &self.decoding_key, &validation)
            .map_err(|e| DomainError::validation(format!("Invalid JWT: {}", e)))?;

        Ok(token_data.claims)
    }

    fn expiration_hours(&self) -> u64 {
        self.expiration_hours
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::team::{TeamId, TeamRole};
    use crate::domain::user::UserId;

    fn admin_team() -> TeamId {
        TeamId::administrators()
    }

    fn create_test_user() -> User {
        let id = UserId::new("test-user").unwrap();
        User::new(id, "testuser", "hashed_password", admin_team(), TeamRole::Member)
    }

    fn create_service() -> JwtService {
        JwtService::new(JwtConfig::new("test-secret-key-12345", 24))
    }

    #[test]
    fn test_generate_and_validate() {
        let service = create_service();
        let user = create_test_user();

        let token = service.generate(&user).unwrap();
        assert!(!token.is_empty());

        let claims = service.validate(&token).unwrap();
        assert_eq!(claims.sub, "test-user");
        assert_eq!(claims.username, "testuser");
        assert!(!claims.is_expired());
    }

    #[test]
    fn test_invalid_token() {
        let service = create_service();

        let result = service.validate("invalid-token");
        assert!(result.is_err());
    }

    #[test]
    fn test_wrong_secret() {
        let service1 = JwtService::new(JwtConfig::new("secret-1", 24));
        let service2 = JwtService::new(JwtConfig::new("secret-2", 24));

        let user = create_test_user();
        let token = service1.generate(&user).unwrap();

        // Token generated with different secret should fail validation
        let result = service2.validate(&token);
        assert!(result.is_err());
    }

    #[test]
    fn test_expired_token() {
        // Create an expired token by manually crafting claims in the past
        use jsonwebtoken::{encode, Header};

        let service = JwtService::new(JwtConfig::new("test-secret", 24));
        let user = create_test_user();

        // Create claims with expiration 1 hour in the past
        let past_time = chrono::Utc::now() - chrono::Duration::hours(1);
        let claims = JwtClaims {
            sub: user.id().as_str().to_string(),
            username: user.username().to_string(),
            iat: (past_time - chrono::Duration::hours(2)).timestamp(),
            exp: past_time.timestamp(), // Already expired
        };

        // Generate token with expired claims
        let token = encode(
            &Header::default(),
            &claims,
            &jsonwebtoken::EncodingKey::from_secret(b"test-secret"),
        )
        .unwrap();

        // Token should fail validation due to expiration
        let result = service.validate(&token);
        assert!(result.is_err());
    }

    #[test]
    fn test_claims_expiration() {
        let user = create_test_user();
        let claims = JwtClaims::new(&user, 24);

        assert!(!claims.is_expired());
        assert_eq!(claims.user_id(), "test-user");
    }

    #[test]
    fn test_expiration_hours() {
        let service = JwtService::new(JwtConfig::new("secret", 48));
        assert_eq!(service.expiration_hours(), 48);
    }

    #[test]
    fn test_default_config() {
        let service = JwtService::with_default_config();
        assert_eq!(service.expiration_hours(), 24);
    }

    #[test]
    fn test_jwks_invalid_json() {
        let result = JwksJwtService::from_jwks_json("not valid json", 24);
        assert!(result.is_err());
    }

    #[test]
    fn test_jwks_empty_keys() {
        let result = JwksJwtService::from_jwks_json(r#"{"keys": []}"#, 24);
        assert!(result.is_err());
    }

    #[test]
    fn test_jwks_hs256() {
        // Test with HS256 (HMAC) symmetric key
        let secret = "super-secret-key-for-testing-purposes-12345678";
        let k_value = URL_SAFE_NO_PAD.encode(secret);

        let jwks_json = format!(
            r#"{{
            "keys": [
                {{
                    "kty": "oct",
                    "kid": "test-key-1",
                    "alg": "HS256",
                    "k": "{}"
                }}
            ]
        }}"#,
            k_value
        );

        let service = JwksJwtService::from_jwks_json(&jwks_json, 24).unwrap();
        let user = create_test_user();

        let token = service.generate(&user).unwrap();
        assert!(!token.is_empty());

        let claims = service.validate(&token).unwrap();
        assert_eq!(claims.sub, "test-user");
        assert_eq!(claims.username, "testuser");
    }

    #[test]
    fn test_jwks_rs256() {
        use rand::rngs::OsRng;
        use rsa::traits::{PrivateKeyParts, PublicKeyParts};

        // Generate a fresh RSA key for testing
        let mut rng = OsRng;
        let private_key = RsaPrivateKey::new(&mut rng, 2048).unwrap();

        // Extract components and encode as base64url
        let n = URL_SAFE_NO_PAD.encode(private_key.n().to_bytes_be());
        let e = URL_SAFE_NO_PAD.encode(private_key.e().to_bytes_be());
        let d = URL_SAFE_NO_PAD.encode(private_key.d().to_bytes_be());

        let primes = private_key.primes();
        let p = URL_SAFE_NO_PAD.encode(primes[0].to_bytes_be());
        let q = URL_SAFE_NO_PAD.encode(primes[1].to_bytes_be());

        // Build JWKS JSON
        let jwks_json = format!(
            r#"{{
            "keys": [
                {{
                    "kty": "RSA",
                    "kid": "test-rsa-key",
                    "alg": "RS256",
                    "n": "{}",
                    "e": "{}",
                    "d": "{}",
                    "p": "{}",
                    "q": "{}"
                }}
            ]
        }}"#,
            n, e, d, p, q
        );

        let service = JwksJwtService::from_jwks_json(&jwks_json, 24).unwrap();
        let user = create_test_user();

        let token = service.generate(&user).unwrap();
        assert!(!token.is_empty());

        // Verify token has kid in header
        let header = jsonwebtoken::decode_header(&token).unwrap();
        assert_eq!(header.kid, Some("test-rsa-key".to_string()));
        assert_eq!(header.alg, Algorithm::RS256);

        let claims = service.validate(&token).unwrap();
        assert_eq!(claims.sub, "test-user");
        assert_eq!(claims.username, "testuser");
    }
}
