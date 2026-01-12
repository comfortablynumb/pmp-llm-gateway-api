//! Credential provider implementations

mod aws_secrets_provider;
mod cached_provider;
mod env_provider;
mod factory;
mod repository;
mod service;
mod storage_repository;
mod vault_provider;

pub use aws_secrets_provider::AwsSecretsCredentialProvider;
pub use cached_provider::CachedCredentialProvider;
pub use env_provider::EnvCredentialProvider;
pub use factory::{CredentialProviderFactory, ProviderConfig};
pub use repository::InMemoryStoredCredentialRepository;
pub use service::{
    CreateCredentialRequest, CredentialService, CredentialServiceTrait, UpdateCredentialRequest,
};
pub use storage_repository::StorageStoredCredentialRepository;
pub use vault_provider::VaultCredentialProvider;
