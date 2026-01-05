//! Credential management domain

mod credential;
mod provider;

pub use credential::{Credential, CredentialType};
pub use provider::CredentialProvider;

#[cfg(test)]
pub use provider::mock;
