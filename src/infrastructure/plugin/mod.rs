//! Plugin Infrastructure
//!
//! This module provides the infrastructure layer for the plugin system, including:
//! - Plugin registry for managing plugin lifecycle
//! - Provider router for routing requests to appropriate plugins
//! - Routing provider resolver for workflow execution
//! - Configuration via TOML files
//! - Built-in plugins for standard LLM providers

pub mod builtin;
pub mod config;
pub mod registry;
pub mod router;
pub mod routing_resolver;

pub use builtin::{
    register_builtin_plugins, register_builtin_plugins_with_config, AnthropicPlugin,
    AzureOpenAiPlugin, BedrockPlugin, OpenAiPlugin,
};
pub use config::{BuiltinProvider, PluginConfig, PluginConfigError, PluginSettings, ProviderConfigs};
pub use registry::PluginRegistry;
pub use router::{CacheStats, ProviderRouter};
pub use routing_resolver::RoutingProviderResolver;
