//! Plugin domain module
//!
//! Provides traits and types for the plugin system that enables extensible provider support.
//!
//! ## Overview
//!
//! The plugin system allows extending the gateway with custom providers for:
//! - LLM providers (chat completions)
//! - Embedding providers (vector generation)
//! - Knowledge base providers (RAG)
//! - Credential providers (secrets management)
//!
//! ## Core Traits
//!
//! - `Plugin` - Base trait for all plugins with lifecycle management
//! - `LlmProviderPlugin` - Extension for LLM capabilities
//! - `EmbeddingProviderPlugin` - Extension for embedding capabilities
//! - `KnowledgeBaseProviderPlugin` - Extension for knowledge base capabilities
//! - `CredentialProviderPlugin` - Extension for credential management

mod credential_provider;
mod embedding_provider;
mod entity;
mod error;
mod extensions;
mod knowledge_base;
mod llm_provider;

// Re-export credential provider types
pub use credential_provider::{
    CredentialProviderConfig, CredentialProviderPlugin, CredentialSourceType,
};

// Re-export embedding provider types
pub use embedding_provider::EmbeddingProviderPlugin;

// Re-export entity types
pub use entity::{Plugin, PluginContext, PluginMetadata, PluginState};

// Re-export error types
pub use error::PluginError;

// Re-export extension types
pub use extensions::ExtensionType;

// Re-export knowledge base types
pub use knowledge_base::{KnowledgeBaseProviderConfig, KnowledgeBaseProviderPlugin};

// Re-export LLM provider types
pub use llm_provider::{LlmProviderConfig, LlmProviderPlugin};
