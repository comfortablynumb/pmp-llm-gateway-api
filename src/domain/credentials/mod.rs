//! Credential management domain

mod credential;
mod provider;
mod stored;

pub use credential::{Credential, CredentialType};
pub use provider::CredentialProvider;
pub use stored::{CredentialId, StoredCredential, StoredCredentialRepository};

#[cfg(test)]
pub use provider::mock;
