use async_trait::async_trait;
use std::fmt::Debug;

use super::{Credential, CredentialType};
use crate::domain::DomainError;

/// Trait for credential providers (ENV, AWS Secrets, Vault, etc.)
#[async_trait]
pub trait CredentialProvider: Send + Sync + Debug {
    /// Get a credential by its type
    async fn get_credential(&self, credential_type: &CredentialType) -> Result<Credential, DomainError>;

    /// Check if this provider supports the given credential type
    async fn supports(&self, credential_type: &CredentialType) -> bool;

    /// Refresh a credential (for rotation support)
    async fn refresh(&self, credential_type: &CredentialType) -> Result<Credential, DomainError> {
        self.get_credential(credential_type).await
    }

    /// Get provider name for logging/debugging
    fn provider_name(&self) -> &'static str;
}

#[cfg(test)]
pub mod mock {
    use super::*;
    use std::collections::HashMap;
    use std::sync::RwLock;

    #[derive(Debug)]
    pub struct MockCredentialProvider {
        credentials: RwLock<HashMap<CredentialType, Credential>>,
        name: &'static str,
    }

    impl MockCredentialProvider {
        pub fn new(name: &'static str) -> Self {
            Self {
                credentials: RwLock::new(HashMap::new()),
                name,
            }
        }

        pub fn with_credential(self, cred: Credential) -> Self {
            self.credentials
                .write()
                .unwrap()
                .insert(cred.credential_type().clone(), cred);
            self
        }
    }

    #[async_trait]
    impl CredentialProvider for MockCredentialProvider {
        async fn get_credential(
            &self,
            credential_type: &CredentialType,
        ) -> Result<Credential, DomainError> {
            self.credentials
                .read()
                .unwrap()
                .get(credential_type)
                .cloned()
                .ok_or_else(|| {
                    DomainError::credential(format!(
                        "Credential not found for type: {}",
                        credential_type
                    ))
                })
        }

        async fn supports(&self, credential_type: &CredentialType) -> bool {
            self.credentials.read().unwrap().contains_key(credential_type)
        }

        fn provider_name(&self) -> &'static str {
            self.name
        }
    }
}
