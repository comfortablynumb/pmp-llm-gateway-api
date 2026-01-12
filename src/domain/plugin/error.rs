//! Plugin error types

use thiserror::Error;

/// Plugin-specific errors
#[derive(Debug, Error)]
pub enum PluginError {
    #[error("Plugin not found: {plugin_id}")]
    NotFound { plugin_id: String },

    #[error("Plugin initialization failed for '{plugin_id}': {message}")]
    InitializationFailed { plugin_id: String, message: String },

    #[error("Plugin already registered: {plugin_id}")]
    AlreadyRegistered { plugin_id: String },

    #[error("Plugin not initialized: {plugin_id}")]
    NotInitialized { plugin_id: String },

    #[error("Plugin health check failed for '{plugin_id}': {message}")]
    HealthCheckFailed { plugin_id: String, message: String },

    #[error("Plugin shutdown failed for '{plugin_id}': {message}")]
    ShutdownFailed { plugin_id: String, message: String },

    #[error("Provider creation failed for '{plugin_id}': {message}")]
    ProviderCreationFailed { plugin_id: String, message: String },

    #[error("Unsupported credential type '{credential_type}' for plugin '{plugin_id}'")]
    UnsupportedCredentialType {
        plugin_id: String,
        credential_type: String,
    },

    #[error("Configuration error for '{plugin_id}': {message}")]
    Configuration { plugin_id: String, message: String },

    #[error("No plugin found for credential type: {credential_type}")]
    NoPluginForCredentialType { credential_type: String },

    #[error("Internal plugin error: {message}")]
    Internal { message: String },
}

impl PluginError {
    pub fn not_found(plugin_id: impl Into<String>) -> Self {
        Self::NotFound {
            plugin_id: plugin_id.into(),
        }
    }

    pub fn initialization_failed(
        plugin_id: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::InitializationFailed {
            plugin_id: plugin_id.into(),
            message: message.into(),
        }
    }

    pub fn already_registered(plugin_id: impl Into<String>) -> Self {
        Self::AlreadyRegistered {
            plugin_id: plugin_id.into(),
        }
    }

    pub fn not_initialized(plugin_id: impl Into<String>) -> Self {
        Self::NotInitialized {
            plugin_id: plugin_id.into(),
        }
    }

    pub fn health_check_failed(
        plugin_id: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::HealthCheckFailed {
            plugin_id: plugin_id.into(),
            message: message.into(),
        }
    }

    pub fn shutdown_failed(plugin_id: impl Into<String>, message: impl Into<String>) -> Self {
        Self::ShutdownFailed {
            plugin_id: plugin_id.into(),
            message: message.into(),
        }
    }

    pub fn provider_creation_failed(
        plugin_id: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::ProviderCreationFailed {
            plugin_id: plugin_id.into(),
            message: message.into(),
        }
    }

    pub fn unsupported_credential_type(
        plugin_id: impl Into<String>,
        credential_type: impl Into<String>,
    ) -> Self {
        Self::UnsupportedCredentialType {
            plugin_id: plugin_id.into(),
            credential_type: credential_type.into(),
        }
    }

    pub fn configuration(plugin_id: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Configuration {
            plugin_id: plugin_id.into(),
            message: message.into(),
        }
    }

    pub fn no_plugin_for_credential_type(credential_type: impl Into<String>) -> Self {
        Self::NoPluginForCredentialType {
            credential_type: credential_type.into(),
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal {
            message: message.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_not_found_error() {
        let error = PluginError::not_found("openai");
        assert_eq!(error.to_string(), "Plugin not found: openai");
    }

    #[test]
    fn test_initialization_failed_error() {
        let error = PluginError::initialization_failed("openai", "Missing API key");
        assert_eq!(
            error.to_string(),
            "Plugin initialization failed for 'openai': Missing API key"
        );
    }

    #[test]
    fn test_already_registered_error() {
        let error = PluginError::already_registered("anthropic");
        assert_eq!(error.to_string(), "Plugin already registered: anthropic");
    }

    #[test]
    fn test_unsupported_credential_type_error() {
        let error = PluginError::unsupported_credential_type("openai", "bedrock");
        assert_eq!(
            error.to_string(),
            "Unsupported credential type 'bedrock' for plugin 'openai'"
        );
    }

    #[test]
    fn test_no_plugin_for_credential_type() {
        let error = PluginError::no_plugin_for_credential_type("unknown");
        assert_eq!(
            error.to_string(),
            "No plugin found for credential type: unknown"
        );
    }
}
