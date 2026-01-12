//! Plugin Registry
//!
//! Central registry for managing plugins and their lifecycle.

use crate::domain::credentials::CredentialType;
use crate::domain::plugin::{
    ExtensionType, LlmProviderPlugin, Plugin, PluginContext, PluginError, PluginMetadata,
    PluginState,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Entry in the plugin registry
#[derive(Debug)]
struct PluginEntry {
    /// The plugin instance
    plugin: Arc<dyn Plugin>,

    /// Current state of the plugin
    state: PluginState,
}

/// Central registry for managing plugins
#[derive(Debug)]
pub struct PluginRegistry {
    /// Plugins indexed by their ID
    plugins: RwLock<HashMap<String, PluginEntry>>,

    /// Index of credential types to plugin IDs for fast lookup
    credential_type_index: RwLock<HashMap<String, Vec<String>>>,
}

impl PluginRegistry {
    /// Create a new empty plugin registry
    pub fn new() -> Self {
        Self {
            plugins: RwLock::new(HashMap::new()),
            credential_type_index: RwLock::new(HashMap::new()),
        }
    }

    /// Register a plugin with the registry
    pub async fn register(&self, plugin: Arc<dyn Plugin>) -> Result<(), PluginError> {
        let metadata = plugin.metadata();
        let plugin_id = metadata.id.clone();

        let mut plugins = self.plugins.write().await;

        if plugins.contains_key(&plugin_id) {
            return Err(PluginError::already_registered(&plugin_id));
        }

        info!(
            plugin_id = %plugin_id,
            plugin_name = %metadata.name,
            plugin_version = %metadata.version,
            "Registering plugin"
        );

        plugins.insert(
            plugin_id.clone(),
            PluginEntry {
                plugin: plugin.clone(),
                state: PluginState::Registered,
            },
        );

        // Update credential type index for LLM provider plugins
        if plugin.extension_types().contains(&ExtensionType::LlmProvider) {
            // Note: We can't directly check for LlmProviderPlugin trait without downcasting
            // This will be handled by the ProviderRouter which knows the concrete types
            debug!(
                plugin_id = %plugin_id,
                "Plugin provides LLM provider extension"
            );
        }

        Ok(())
    }

    /// Register an LLM provider plugin with credential type indexing
    pub async fn register_llm_provider(
        &self,
        plugin: Arc<dyn LlmProviderPlugin>,
    ) -> Result<(), PluginError> {
        let metadata = plugin.metadata();
        let plugin_id = metadata.id.clone();

        // Register the base plugin
        self.register(plugin.clone() as Arc<dyn Plugin>).await?;

        // Index by credential types
        let mut index = self.credential_type_index.write().await;
        for cred_type in plugin.supported_credential_types() {
            let key = credential_type_to_string(&cred_type);
            index.entry(key).or_default().push(plugin_id.clone());
        }

        Ok(())
    }

    /// Initialize a specific plugin
    pub async fn initialize_plugin(
        &self,
        plugin_id: &str,
        context: PluginContext,
    ) -> Result<(), PluginError> {
        let mut plugins = self.plugins.write().await;

        let entry = plugins
            .get_mut(plugin_id)
            .ok_or_else(|| PluginError::not_found(plugin_id))?;

        if !entry.state.can_initialize() {
            return Err(PluginError::initialization_failed(
                plugin_id,
                format!("Cannot initialize plugin in state {:?}", entry.state),
            ));
        }

        entry.state = PluginState::Initializing;

        match entry.plugin.initialize(context).await {
            Ok(()) => {
                entry.state = PluginState::Ready;
                info!(plugin_id = %plugin_id, "Plugin initialized successfully");
                Ok(())
            }
            Err(e) => {
                entry.state = PluginState::Error;
                error!(plugin_id = %plugin_id, error = %e, "Plugin initialization failed");
                Err(e)
            }
        }
    }

    /// Initialize all registered plugins
    pub async fn initialize_all(&self, context: PluginContext) -> Vec<PluginError> {
        let plugin_ids: Vec<String> = {
            let plugins = self.plugins.read().await;
            plugins.keys().cloned().collect()
        };

        let mut errors = Vec::new();

        for plugin_id in plugin_ids {
            if let Err(e) = self.initialize_plugin(&plugin_id, context.clone()).await {
                errors.push(e);
            }
        }

        errors
    }

    /// Shutdown a specific plugin
    pub async fn shutdown_plugin(&self, plugin_id: &str) -> Result<(), PluginError> {
        let mut plugins = self.plugins.write().await;

        let entry = plugins
            .get_mut(plugin_id)
            .ok_or_else(|| PluginError::not_found(plugin_id))?;

        if !entry.state.can_shutdown() {
            warn!(
                plugin_id = %plugin_id,
                state = ?entry.state,
                "Plugin cannot be shut down in current state"
            );
            return Ok(());
        }

        entry.state = PluginState::ShuttingDown;

        match entry.plugin.shutdown().await {
            Ok(()) => {
                entry.state = PluginState::Stopped;
                info!(plugin_id = %plugin_id, "Plugin shut down successfully");
                Ok(())
            }
            Err(e) => {
                entry.state = PluginState::Error;
                error!(plugin_id = %plugin_id, error = %e, "Plugin shutdown failed");
                Err(e)
            }
        }
    }

    /// Shutdown all plugins
    pub async fn shutdown_all(&self) -> Vec<PluginError> {
        let plugin_ids: Vec<String> = {
            let plugins = self.plugins.read().await;
            plugins.keys().cloned().collect()
        };

        let mut errors = Vec::new();

        for plugin_id in plugin_ids {
            if let Err(e) = self.shutdown_plugin(&plugin_id).await {
                errors.push(e);
            }
        }

        errors
    }

    /// Get a plugin by ID
    pub async fn get_plugin(&self, plugin_id: &str) -> Option<Arc<dyn Plugin>> {
        let plugins = self.plugins.read().await;
        plugins.get(plugin_id).map(|entry| entry.plugin.clone())
    }

    /// Get the state of a plugin
    pub async fn get_plugin_state(&self, plugin_id: &str) -> Option<PluginState> {
        let plugins = self.plugins.read().await;
        plugins.get(plugin_id).map(|entry| entry.state)
    }

    /// Get all registered plugin metadata
    pub async fn list_plugins(&self) -> Vec<PluginMetadata> {
        let plugins = self.plugins.read().await;
        plugins
            .values()
            .map(|entry| entry.plugin.metadata().clone())
            .collect()
    }

    /// Get all ready plugins
    pub async fn list_ready_plugins(&self) -> Vec<PluginMetadata> {
        let plugins = self.plugins.read().await;
        plugins
            .values()
            .filter(|entry| entry.state.is_ready())
            .map(|entry| entry.plugin.metadata().clone())
            .collect()
    }

    /// Get plugin IDs that support a specific credential type
    pub async fn get_plugins_for_credential_type(
        &self,
        credential_type: &CredentialType,
    ) -> Vec<String> {
        let index = self.credential_type_index.read().await;
        let key = credential_type_to_string(credential_type);
        index.get(&key).cloned().unwrap_or_default()
    }

    /// Check if any plugin supports a credential type
    pub async fn has_plugin_for_credential_type(&self, credential_type: &CredentialType) -> bool {
        !self
            .get_plugins_for_credential_type(credential_type)
            .await
            .is_empty()
    }

    /// Run health checks on all ready plugins
    pub async fn health_check_all(&self) -> HashMap<String, Result<bool, PluginError>> {
        let plugins = self.plugins.read().await;
        let mut results = HashMap::new();

        for (plugin_id, entry) in plugins.iter() {
            if entry.state.is_ready() {
                let result = entry.plugin.health_check().await;
                results.insert(plugin_id.clone(), result);
            }
        }

        results
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert CredentialType to a string key for indexing
fn credential_type_to_string(cred_type: &CredentialType) -> String {
    match cred_type {
        CredentialType::OpenAi => "openai".to_string(),
        CredentialType::Anthropic => "anthropic".to_string(),
        CredentialType::AzureOpenAi => "azure_openai".to_string(),
        CredentialType::AwsBedrock => "aws_bedrock".to_string(),
        CredentialType::Pgvector => "pgvector".to_string(),
        CredentialType::AwsKnowledgeBase => "aws_knowledge_base".to_string(),
        CredentialType::Pinecone => "pinecone".to_string(),
        CredentialType::HttpApiKey => "http_api_key".to_string(),
        CredentialType::Custom(name) => format!("custom_{}", name),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::plugin::PluginMetadata;

    // Mock plugin for testing
    #[derive(Debug)]
    struct MockPlugin {
        metadata: PluginMetadata,
        state: std::sync::atomic::AtomicU8,
    }

    impl MockPlugin {
        fn new(id: &str) -> Self {
            Self {
                metadata: PluginMetadata::new(id, format!("Mock Plugin {}", id), "1.0.0"),
                state: std::sync::atomic::AtomicU8::new(0),
            }
        }
    }

    #[async_trait::async_trait]
    impl Plugin for MockPlugin {
        fn metadata(&self) -> &PluginMetadata {
            &self.metadata
        }

        fn extension_types(&self) -> Vec<ExtensionType> {
            vec![ExtensionType::LlmProvider]
        }

        async fn initialize(&self, _context: PluginContext) -> Result<(), PluginError> {
            self.state
                .store(1, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        }

        async fn health_check(&self) -> Result<bool, PluginError> {
            Ok(self.state.load(std::sync::atomic::Ordering::SeqCst) == 1)
        }

        async fn shutdown(&self) -> Result<(), PluginError> {
            self.state
                .store(2, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        }

        fn state(&self) -> PluginState {
            match self.state.load(std::sync::atomic::Ordering::SeqCst) {
                0 => PluginState::Registered,
                1 => PluginState::Ready,
                2 => PluginState::Stopped,
                _ => PluginState::Error,
            }
        }
    }

    #[tokio::test]
    async fn test_register_plugin() {
        let registry = PluginRegistry::new();
        let plugin = Arc::new(MockPlugin::new("test-plugin"));

        let result = registry.register(plugin).await;
        assert!(result.is_ok());

        let plugins = registry.list_plugins().await;
        assert_eq!(plugins.len(), 1);
        assert_eq!(plugins[0].id, "test-plugin");
    }

    #[tokio::test]
    async fn test_duplicate_registration() {
        let registry = PluginRegistry::new();
        let plugin1 = Arc::new(MockPlugin::new("test-plugin"));
        let plugin2 = Arc::new(MockPlugin::new("test-plugin"));

        registry.register(plugin1).await.unwrap();
        let result = registry.register(plugin2).await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            PluginError::AlreadyRegistered { .. }
        ));
    }

    #[tokio::test]
    async fn test_initialize_plugin() {
        let registry = PluginRegistry::new();
        let plugin = Arc::new(MockPlugin::new("test-plugin"));

        registry.register(plugin).await.unwrap();
        let result = registry
            .initialize_plugin("test-plugin", PluginContext::new())
            .await;

        assert!(result.is_ok());

        let state = registry.get_plugin_state("test-plugin").await;
        assert_eq!(state, Some(PluginState::Ready));
    }

    #[tokio::test]
    async fn test_shutdown_plugin() {
        let registry = PluginRegistry::new();
        let plugin = Arc::new(MockPlugin::new("test-plugin"));

        registry.register(plugin).await.unwrap();
        registry
            .initialize_plugin("test-plugin", PluginContext::new())
            .await
            .unwrap();

        let result = registry.shutdown_plugin("test-plugin").await;
        assert!(result.is_ok());

        let state = registry.get_plugin_state("test-plugin").await;
        assert_eq!(state, Some(PluginState::Stopped));
    }

    #[tokio::test]
    async fn test_list_ready_plugins() {
        let registry = PluginRegistry::new();
        let plugin1 = Arc::new(MockPlugin::new("plugin-1"));
        let plugin2 = Arc::new(MockPlugin::new("plugin-2"));

        registry.register(plugin1).await.unwrap();
        registry.register(plugin2).await.unwrap();

        // Only initialize plugin-1
        registry
            .initialize_plugin("plugin-1", PluginContext::new())
            .await
            .unwrap();

        let ready = registry.list_ready_plugins().await;
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].id, "plugin-1");
    }

    #[tokio::test]
    async fn test_credential_type_to_string() {
        assert_eq!(credential_type_to_string(&CredentialType::OpenAi), "openai");
        assert_eq!(
            credential_type_to_string(&CredentialType::Anthropic),
            "anthropic"
        );
        assert_eq!(
            credential_type_to_string(&CredentialType::AzureOpenAi),
            "azure_openai"
        );
        assert_eq!(
            credential_type_to_string(&CredentialType::Custom("my-provider".to_string())),
            "custom_my-provider"
        );
    }
}
