//! PMP LLM Gateway API
//!
//! A unified interface for multiple LLM providers with support for:
//! - Multiple credential sources (ENV, AWS Secrets, Vault)
//! - Model configuration and chaining
//! - Knowledge bases with CRAG support
//! - Caching and storage strategies

pub mod api;
pub mod cli;
pub mod config;
pub mod domain;
pub mod infrastructure;

pub use config::AppConfig;

use std::sync::Arc;

use api::state::AppState;
use domain::{
    api_key::ApiKeyPermissions, CredentialType, InMemoryModelRepository,
    InMemoryPromptRepository, Model, ModelId, Prompt, PromptId,
};
use infrastructure::{
    api_key::{ApiKeyGenerator, ApiKeyService, InMemoryApiKeyRepository},
    llm::LlmProviderFactory,
    operation::InMemoryOperationRepository,
    services::{ModelService, OperationService, PromptService, WorkflowService},
    workflow::{InMemoryWorkflowRepository, WorkflowExecutorImpl},
};
use tracing::info;

/// Create the application state with all services initialized
pub async fn create_app_state() -> anyhow::Result<AppState> {
    let llm_provider = create_llm_provider()?;

    let model_repository = Arc::new(InMemoryModelRepository::new().with_models(default_models()));
    let prompt_repository =
        Arc::new(InMemoryPromptRepository::new().with_prompts(default_prompts()));

    let model_service = Arc::new(ModelService::new(model_repository));
    let prompt_service = Arc::new(PromptService::new(prompt_repository));

    let api_key_repository = Arc::new(InMemoryApiKeyRepository::new());
    let api_key_service = Arc::new(
        ApiKeyService::new(api_key_repository.clone())
            .with_generator(ApiKeyGenerator::new("pk_test_")),
    );

    if let Ok(admin_key) = std::env::var("ADMIN_API_KEY") {
        create_admin_api_key(&api_key_service, &admin_key).await?;
    }

    let workflow_repository = Arc::new(InMemoryWorkflowRepository::new());
    let workflow_executor = Arc::new(WorkflowExecutorImpl::new(llm_provider.clone()));
    let workflow_service = Arc::new(WorkflowService::new(workflow_repository, workflow_executor));

    let operation_repository = Arc::new(InMemoryOperationRepository::new());
    let operation_service = Arc::new(OperationService::new(operation_repository));

    Ok(AppState::new(
        model_service,
        prompt_service,
        api_key_service,
        workflow_service,
        operation_service,
        llm_provider,
    ))
}

fn create_llm_provider() -> anyhow::Result<Arc<dyn domain::LlmProvider>> {
    let provider_type = std::env::var("LLM_PROVIDER").unwrap_or_else(|_| "openai".to_string());

    match provider_type.to_lowercase().as_str() {
        "openai" => create_openai_provider(),
        "anthropic" => create_anthropic_provider(),
        "azure" | "azure_openai" => create_azure_provider(),
        _ => {
            info!(
                "Unknown provider '{}', defaulting to OpenAI",
                provider_type
            );
            create_openai_provider()
        }
    }
}

fn create_openai_provider() -> anyhow::Result<Arc<dyn domain::LlmProvider>> {
    let api_key =
        std::env::var("OPENAI_API_KEY").unwrap_or_else(|_| "sk-placeholder".to_string());
    let base_url = std::env::var("OPENAI_BASE_URL").ok();

    if let Some(url) = base_url {
        info!("Using OpenAI provider with custom base URL: {}", url);
        Ok(LlmProviderFactory::create_openai_with_base_url(api_key, url))
    } else {
        info!("Using OpenAI provider with default base URL");
        Ok(LlmProviderFactory::create_openai(api_key))
    }
}

fn create_anthropic_provider() -> anyhow::Result<Arc<dyn domain::LlmProvider>> {
    let api_key =
        std::env::var("ANTHROPIC_API_KEY").unwrap_or_else(|_| "sk-placeholder".to_string());
    let base_url = std::env::var("ANTHROPIC_BASE_URL").ok();

    if let Some(url) = base_url {
        info!("Using Anthropic provider with custom base URL: {}", url);
        Ok(LlmProviderFactory::create_anthropic_with_base_url(
            api_key, url,
        ))
    } else {
        info!("Using Anthropic provider with default base URL");
        Ok(LlmProviderFactory::create_anthropic(api_key))
    }
}

fn create_azure_provider() -> anyhow::Result<Arc<dyn domain::LlmProvider>> {
    let api_key =
        std::env::var("AZURE_OPENAI_API_KEY").unwrap_or_else(|_| "placeholder".to_string());
    let endpoint = std::env::var("AZURE_OPENAI_ENDPOINT")
        .unwrap_or_else(|_| "https://placeholder.openai.azure.com".to_string());

    info!("Using Azure OpenAI provider with endpoint: {}", endpoint);
    Ok(LlmProviderFactory::create_azure_openai(endpoint, api_key))
}

async fn create_admin_api_key(
    service: &ApiKeyService<InMemoryApiKeyRepository>,
    key_secret: &str,
) -> anyhow::Result<()> {
    use domain::api_key::{ApiKeyId, ResourcePermission};

    let permissions = ApiKeyPermissions {
        admin: true,
        models: ResourcePermission::All,
        knowledge_bases: ResourcePermission::All,
        prompts: ResourcePermission::All,
        chains: ResourcePermission::All,
    };

    let key_id = ApiKeyId::new("admin-key")?;

    let result = if !key_secret.is_empty() {
        service
            .create_with_secret(key_id, "Admin API Key", key_secret, permissions, None)
            .await?
    } else {
        service
            .create(key_id, "Admin API Key", permissions, None)
            .await?
    };

    info!(
        "Admin API key created with ID: {}",
        result.api_key.id().as_str()
    );
    info!("API key secret: {}", result.secret);

    Ok(())
}

fn default_models() -> Vec<Model> {
    vec![
        Model::new(
            ModelId::new("gpt-4").unwrap(),
            "GPT-4",
            CredentialType::OpenAi,
            "gpt-4",
        ),
        Model::new(
            ModelId::new("gpt-4o").unwrap(),
            "GPT-4o",
            CredentialType::OpenAi,
            "gpt-4o",
        ),
        Model::new(
            ModelId::new("gpt-3-5-turbo").unwrap(),
            "GPT-3.5 Turbo",
            CredentialType::OpenAi,
            "gpt-3.5-turbo",
        ),
        Model::new(
            ModelId::new("claude-3-5-sonnet").unwrap(),
            "Claude 3.5 Sonnet",
            CredentialType::Anthropic,
            "claude-3-5-sonnet-20241022",
        ),
        Model::new(
            ModelId::new("claude-3-opus").unwrap(),
            "Claude 3 Opus",
            CredentialType::Anthropic,
            "claude-3-opus-20240229",
        ),
    ]
}

fn default_prompts() -> Vec<Prompt> {
    vec![
        Prompt::new(
            PromptId::new("helpful-assistant").unwrap(),
            "Helpful Assistant".to_string(),
            "You are a helpful assistant that provides accurate and concise answers.".to_string(),
        ),
        Prompt::new(
            PromptId::new("code-reviewer").unwrap(),
            "Code Reviewer".to_string(),
            "You are an expert code reviewer. Review the following code for bugs, security issues, and improvements:\n\n${var:code}".to_string(),
        ),
    ]
}
