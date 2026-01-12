//! Built-in Plugins
//!
//! This module contains the built-in provider plugins that ship with the gateway.

mod anthropic;
mod azure;
mod bedrock;
mod openai;

pub use anthropic::AnthropicPlugin;
pub use azure::AzureOpenAiPlugin;
pub use bedrock::BedrockPlugin;
pub use openai::OpenAiPlugin;

use crate::domain::plugin::{LlmProviderPlugin, PluginContext, PluginError};
use crate::infrastructure::plugin::config::{BuiltinProvider, PluginConfig};
use crate::infrastructure::plugin::registry::PluginRegistry;
use crate::infrastructure::plugin::router::ProviderRouter;
use std::sync::Arc;
use tracing::{debug, info};

/// Register all built-in LLM provider plugins using default configuration
/// (all providers enabled).
pub async fn register_builtin_plugins(
    registry: &PluginRegistry,
    router: &ProviderRouter,
) -> Result<(), Vec<PluginError>> {
    register_builtin_plugins_with_config(registry, router, &PluginConfig::default()).await
}

/// Register built-in LLM provider plugins based on configuration.
///
/// Only plugins that are enabled in the configuration will be registered.
pub async fn register_builtin_plugins_with_config(
    registry: &PluginRegistry,
    router: &ProviderRouter,
    config: &PluginConfig,
) -> Result<(), Vec<PluginError>> {
    if !config.settings.enabled {
        info!("Plugin system is disabled by configuration");
        return Ok(());
    }

    let enabled_providers = config.enabled_providers();
    let provider_count = enabled_providers.len();
    info!(count = provider_count, "Registering enabled built-in plugins");

    let mut errors = Vec::new();

    for provider in enabled_providers {
        let plugin: Arc<dyn LlmProviderPlugin> = match provider {
            BuiltinProvider::OpenAi => Arc::new(OpenAiPlugin::new()),
            BuiltinProvider::Anthropic => Arc::new(AnthropicPlugin::new()),
            BuiltinProvider::AzureOpenAi => Arc::new(AzureOpenAiPlugin::new()),
            BuiltinProvider::Bedrock => Arc::new(BedrockPlugin::new()),
        };

        let plugin_name = provider.name();
        debug!(plugin = plugin_name, "Registering plugin");

        // Register with registry
        if let Err(e) = registry.register_llm_provider(plugin.clone()).await {
            errors.push(e);
            continue;
        }

        // Register with router
        router.register_llm_plugin(plugin.clone()).await;

        // Initialize the plugin
        let plugin_id = plugin.metadata().id.clone();

        if let Err(e) = registry
            .initialize_plugin(&plugin_id, PluginContext::new())
            .await
        {
            errors.push(e);
        } else {
            debug!(plugin = plugin_name, "Plugin registered and initialized");
        }
    }

    if errors.is_empty() {
        info!(
            count = provider_count,
            "All enabled plugins registered successfully"
        );
        Ok(())
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_register_all_builtin_plugins() {
        let registry = PluginRegistry::new();
        let router = ProviderRouter::new();

        let result = register_builtin_plugins(&registry, &router).await;
        assert!(result.is_ok());

        // Verify all plugins are registered
        let plugins = registry.list_plugins().await;
        assert_eq!(plugins.len(), 4);

        let plugin_ids: Vec<_> = plugins.iter().map(|p| p.id.as_str()).collect();
        assert!(plugin_ids.contains(&"openai"));
        assert!(plugin_ids.contains(&"anthropic"));
        assert!(plugin_ids.contains(&"azure_openai"));
        assert!(plugin_ids.contains(&"aws_bedrock"));
    }

    #[tokio::test]
    async fn test_builtin_plugins_are_ready() {
        let registry = PluginRegistry::new();
        let router = ProviderRouter::new();

        register_builtin_plugins(&registry, &router).await.unwrap();

        let ready_plugins = registry.list_ready_plugins().await;
        assert_eq!(ready_plugins.len(), 4);
    }

    #[tokio::test]
    async fn test_router_supports_credential_types() {
        let registry = PluginRegistry::new();
        let router = ProviderRouter::new();

        register_builtin_plugins(&registry, &router).await.unwrap();

        let credential_types = router.list_supported_credential_types().await;
        assert!(credential_types.contains(&"openai".to_string()));
        assert!(credential_types.contains(&"anthropic".to_string()));
        assert!(credential_types.contains(&"azure_openai".to_string()));
        assert!(credential_types.contains(&"aws_bedrock".to_string()));
    }

    #[tokio::test]
    async fn test_register_with_config_subset() {
        let registry = PluginRegistry::new();
        let router = ProviderRouter::new();

        // Only enable OpenAI and Anthropic
        let config = PluginConfig::from_str(
            r#"
[providers.azure_openai]
enabled = false

[providers.bedrock]
enabled = false
"#,
        )
        .unwrap();

        let result = register_builtin_plugins_with_config(&registry, &router, &config).await;
        assert!(result.is_ok());

        let plugins = registry.list_plugins().await;
        assert_eq!(plugins.len(), 2);

        let plugin_ids: Vec<_> = plugins.iter().map(|p| p.id.as_str()).collect();
        assert!(plugin_ids.contains(&"openai"));
        assert!(plugin_ids.contains(&"anthropic"));
        assert!(!plugin_ids.contains(&"azure_openai"));
        assert!(!plugin_ids.contains(&"aws_bedrock"));
    }

    #[tokio::test]
    async fn test_register_with_disabled_plugin_system() {
        let registry = PluginRegistry::new();
        let router = ProviderRouter::new();

        let config = PluginConfig::from_str(
            r#"
[settings]
enabled = false
"#,
        )
        .unwrap();

        let result = register_builtin_plugins_with_config(&registry, &router, &config).await;
        assert!(result.is_ok());

        // No plugins should be registered
        let plugins = registry.list_plugins().await;
        assert_eq!(plugins.len(), 0);
    }

    #[tokio::test]
    async fn test_register_single_provider() {
        let registry = PluginRegistry::new();
        let router = ProviderRouter::new();

        let config = PluginConfig::from_str(
            r#"
[providers.openai]
enabled = true

[providers.anthropic]
enabled = false

[providers.azure_openai]
enabled = false

[providers.bedrock]
enabled = false
"#,
        )
        .unwrap();

        let result = register_builtin_plugins_with_config(&registry, &router, &config).await;
        assert!(result.is_ok());

        let plugins = registry.list_plugins().await;
        assert_eq!(plugins.len(), 1);
        assert_eq!(plugins[0].id, "openai");
    }
}
