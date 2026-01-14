//! Routing provider resolver implementation
//!
//! Resolves model IDs to LLM providers using the plugin router,
//! with fallback to a default provider.

use async_trait::async_trait;
use std::sync::Arc;
use tracing::{debug, warn};

use crate::api::state::{CredentialServiceTrait, ModelServiceTrait};
use crate::domain::llm::{LlmProvider, ProviderResolver, ResolvedModel};
use crate::domain::DomainError;
use crate::infrastructure::plugin::ProviderRouter;

/// A provider resolver that uses the plugin router to resolve models
/// to their appropriate providers based on credential configuration.
pub struct RoutingProviderResolver {
    /// Model service for looking up model configurations
    model_service: Arc<dyn ModelServiceTrait>,

    /// Credential service for looking up credentials
    credential_service: Arc<dyn CredentialServiceTrait>,

    /// Provider router for creating provider instances
    provider_router: Arc<ProviderRouter>,

    /// Fallback provider when resolution fails
    fallback_provider: Arc<dyn LlmProvider>,
}

impl std::fmt::Debug for RoutingProviderResolver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RoutingProviderResolver")
            .field("provider_router", &"ProviderRouter { ... }")
            .field(
                "fallback_provider",
                &self.fallback_provider.provider_name(),
            )
            .finish()
    }
}

impl RoutingProviderResolver {
    /// Create a new routing provider resolver.
    pub fn new(
        model_service: Arc<dyn ModelServiceTrait>,
        credential_service: Arc<dyn CredentialServiceTrait>,
        provider_router: Arc<ProviderRouter>,
        fallback_provider: Arc<dyn LlmProvider>,
    ) -> Self {
        Self {
            model_service,
            credential_service,
            provider_router,
            fallback_provider,
        }
    }
}

#[async_trait]
impl ProviderResolver for RoutingProviderResolver {
    async fn resolve(&self, model_id: &str) -> Result<Arc<dyn LlmProvider>, DomainError> {
        // Try to get the model configuration
        let model = match self.model_service.get(model_id).await {
            Ok(Some(model)) => model,
            Ok(None) => {
                debug!(model_id = %model_id, "Model not found, using fallback provider");
                return Ok(self.fallback_provider.clone());
            }
            Err(e) => {
                warn!(model_id = %model_id, error = %e, "Failed to get model, using fallback provider");
                return Ok(self.fallback_provider.clone());
            }
        };

        // Get the credential for this model
        let credential_id = model.credential_id();
        let stored_credential = match self.credential_service.get(credential_id).await {
            Ok(Some(cred)) => cred,
            Ok(None) => {
                warn!(
                    model_id = %model_id,
                    credential_id = %credential_id,
                    "Credential not found, using fallback provider"
                );
                return Ok(self.fallback_provider.clone());
            }
            Err(e) => {
                warn!(
                    model_id = %model_id,
                    credential_id = %credential_id,
                    error = %e,
                    "Failed to get credential, using fallback provider"
                );
                return Ok(self.fallback_provider.clone());
            }
        };

        // Check if credential is enabled
        if !stored_credential.is_enabled() {
            warn!(
                model_id = %model_id,
                credential_id = %credential_id,
                "Credential is disabled, using fallback provider"
            );
            return Ok(self.fallback_provider.clone());
        }

        // Convert to domain Credential and use router
        let credential = stored_credential.to_credential();

        match self.provider_router.get_provider(&model, &credential).await {
            Ok(provider) => {
                debug!(
                    model_id = %model_id,
                    credential_id = %credential_id,
                    "Using provider from router"
                );
                Ok(provider)
            }
            Err(e) => {
                warn!(
                    model_id = %model_id,
                    error = %e,
                    "Failed to get provider from router, using fallback"
                );
                Ok(self.fallback_provider.clone())
            }
        }
    }

    async fn resolve_with_model(&self, model_id: &str) -> Result<ResolvedModel, DomainError> {
        // Try to get the model configuration
        let model = match self.model_service.get(model_id).await {
            Ok(Some(model)) => model,
            Ok(None) => {
                debug!(model_id = %model_id, "Model not found, using fallback provider");
                return Ok(ResolvedModel {
                    provider: self.fallback_provider.clone(),
                    provider_model: model_id.to_string(),
                });
            }
            Err(e) => {
                warn!(model_id = %model_id, error = %e, "Failed to get model, using fallback provider");
                return Ok(ResolvedModel {
                    provider: self.fallback_provider.clone(),
                    provider_model: model_id.to_string(),
                });
            }
        };

        // Get the provider_model from the model entity
        let provider_model = model.provider_model().to_string();

        // Get the credential for this model
        let credential_id = model.credential_id();
        let stored_credential = match self.credential_service.get(credential_id).await {
            Ok(Some(cred)) => cred,
            Ok(None) => {
                warn!(
                    model_id = %model_id,
                    credential_id = %credential_id,
                    "Credential not found, using fallback provider"
                );
                return Ok(ResolvedModel {
                    provider: self.fallback_provider.clone(),
                    provider_model,
                });
            }
            Err(e) => {
                warn!(
                    model_id = %model_id,
                    credential_id = %credential_id,
                    error = %e,
                    "Failed to get credential, using fallback provider"
                );
                return Ok(ResolvedModel {
                    provider: self.fallback_provider.clone(),
                    provider_model,
                });
            }
        };

        // Check if credential is enabled
        if !stored_credential.is_enabled() {
            warn!(
                model_id = %model_id,
                credential_id = %credential_id,
                "Credential is disabled, using fallback provider"
            );
            return Ok(ResolvedModel {
                provider: self.fallback_provider.clone(),
                provider_model,
            });
        }

        // Convert to domain Credential and use router
        let credential = stored_credential.to_credential();

        match self.provider_router.get_provider(&model, &credential).await {
            Ok(provider) => {
                debug!(
                    model_id = %model_id,
                    credential_id = %credential_id,
                    provider_model = %provider_model,
                    "Using provider from router with provider_model"
                );
                Ok(ResolvedModel {
                    provider,
                    provider_model,
                })
            }
            Err(e) => {
                warn!(
                    model_id = %model_id,
                    error = %e,
                    "Failed to get provider from router, using fallback"
                );
                Ok(ResolvedModel {
                    provider: self.fallback_provider.clone(),
                    provider_model,
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::credentials::{CredentialId, CredentialType, StoredCredential};
    use crate::domain::llm::MockLlmProvider;
    use crate::domain::model::{Model, ModelId};
    use crate::infrastructure::credentials::{CreateCredentialRequest, UpdateCredentialRequest};
    use crate::infrastructure::services::{CreateModelRequest, UpdateModelRequest};

    // Mock model service
    #[derive(Debug)]
    struct MockModelService {
        model: Option<Model>,
    }

    impl MockModelService {
        fn with_model(model: Model) -> Self {
            Self { model: Some(model) }
        }

        fn empty() -> Self {
            Self { model: None }
        }
    }

    #[async_trait]
    impl ModelServiceTrait for MockModelService {
        async fn get(&self, _id: &str) -> Result<Option<Model>, DomainError> {
            Ok(self.model.clone())
        }

        async fn list(&self) -> Result<Vec<Model>, DomainError> {
            Ok(self.model.clone().into_iter().collect())
        }

        async fn create(&self, _request: CreateModelRequest) -> Result<Model, DomainError> {
            unimplemented!()
        }

        async fn update(
            &self,
            _id: &str,
            _request: UpdateModelRequest,
        ) -> Result<Model, DomainError> {
            unimplemented!()
        }

        async fn delete(&self, _id: &str) -> Result<bool, DomainError> {
            unimplemented!()
        }
    }

    // Mock credential service
    #[derive(Debug)]
    struct MockCredentialService {
        credential: Option<StoredCredential>,
    }

    impl MockCredentialService {
        fn with_credential(credential: StoredCredential) -> Self {
            Self {
                credential: Some(credential),
            }
        }

        fn empty() -> Self {
            Self { credential: None }
        }
    }

    #[async_trait]
    impl CredentialServiceTrait for MockCredentialService {
        async fn get(&self, _id: &str) -> Result<Option<StoredCredential>, DomainError> {
            Ok(self.credential.clone())
        }

        async fn list(&self) -> Result<Vec<StoredCredential>, DomainError> {
            Ok(self.credential.clone().into_iter().collect())
        }

        async fn create(
            &self,
            _request: CreateCredentialRequest,
        ) -> Result<StoredCredential, DomainError> {
            unimplemented!()
        }

        async fn update(
            &self,
            _id: &str,
            _request: UpdateCredentialRequest,
        ) -> Result<StoredCredential, DomainError> {
            unimplemented!()
        }

        async fn delete(&self, _id: &str) -> Result<(), DomainError> {
            unimplemented!()
        }

        async fn exists(&self, _id: &str) -> Result<bool, DomainError> {
            Ok(self.credential.is_some())
        }
    }

    fn create_test_model(model_id: &str, credential_id: &str) -> Model {
        Model::new(
            ModelId::new(model_id).unwrap(),
            "Test Model".to_string(),
            CredentialType::OpenAi,
            "gpt-4".to_string(),
            credential_id.to_string(),
        )
    }

    fn create_test_credential(id: &str) -> StoredCredential {
        StoredCredential::new(
            CredentialId::new(id).unwrap(),
            "Test Credential".to_string(),
            CredentialType::OpenAi,
            "sk-test-key".to_string(),
        )
    }

    fn create_disabled_credential(id: &str) -> StoredCredential {
        create_test_credential(id).with_enabled(false)
    }

    #[tokio::test]
    async fn test_resolve_returns_fallback_when_model_not_found() {
        let fallback = Arc::new(MockLlmProvider::new("mock"));
        let resolver = RoutingProviderResolver::new(
            Arc::new(MockModelService::empty()),
            Arc::new(MockCredentialService::empty()),
            Arc::new(ProviderRouter::new()),
            fallback.clone(),
        );

        let provider = resolver.resolve("unknown-model").await.unwrap();
        assert_eq!(provider.provider_name(), "mock");
    }

    #[tokio::test]
    async fn test_resolve_returns_fallback_when_credential_not_found() {
        let model = create_test_model("gpt-4-test", "openai-cred");
        let fallback = Arc::new(MockLlmProvider::new("mock"));
        let resolver = RoutingProviderResolver::new(
            Arc::new(MockModelService::with_model(model)),
            Arc::new(MockCredentialService::empty()),
            Arc::new(ProviderRouter::new()),
            fallback.clone(),
        );

        let provider = resolver.resolve("gpt-4-test").await.unwrap();
        assert_eq!(provider.provider_name(), "mock");
    }

    #[tokio::test]
    async fn test_resolve_returns_fallback_when_credential_disabled() {
        let model = create_test_model("gpt-4-test", "openai-cred");
        let credential = create_disabled_credential("openai-cred");

        let fallback = Arc::new(MockLlmProvider::new("mock"));
        let resolver = RoutingProviderResolver::new(
            Arc::new(MockModelService::with_model(model)),
            Arc::new(MockCredentialService::with_credential(credential)),
            Arc::new(ProviderRouter::new()),
            fallback.clone(),
        );

        let provider = resolver.resolve("gpt-4-test").await.unwrap();
        assert_eq!(provider.provider_name(), "mock");
    }

    #[tokio::test]
    async fn test_resolve_returns_fallback_when_no_plugin_registered() {
        let model = create_test_model("gpt-4-test", "openai-cred");
        let credential = create_test_credential("openai-cred");

        let fallback = Arc::new(MockLlmProvider::new("mock"));
        let resolver = RoutingProviderResolver::new(
            Arc::new(MockModelService::with_model(model)),
            Arc::new(MockCredentialService::with_credential(credential)),
            Arc::new(ProviderRouter::new()), // Empty router - no plugins registered
            fallback.clone(),
        );

        // Should fall back because no plugin is registered for OpenAI
        let provider = resolver.resolve("gpt-4-test").await.unwrap();
        assert_eq!(provider.provider_name(), "mock");
    }
}
