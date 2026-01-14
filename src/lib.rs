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
    api_key::ApiKeyPermissions,
    config::ExecutionLog,
    credentials::StoredCredential,
    knowledge_base::KnowledgeBase,
    team::Team,
    workflow::Workflow,
    Model, Prompt,
};
use infrastructure::{
    api_key::{ApiKeyGenerator, ApiKeyService, InMemoryApiKeyRepository, StorageApiKeyRepository},
    auth::{JwtConfig, JwksJwtService, JwtService},
    config::{InMemoryConfigRepository, PostgresConfigRepository, StorageExecutionLogRepository},
    credentials::{CredentialService, InMemoryStoredCredentialRepository, StorageStoredCredentialRepository},
    experiment::{
        InMemoryExperimentRecordRepository, InMemoryExperimentRepository,
        StorageExperimentRecordRepository, StorageExperimentRepository,
    },
    external_api,
    knowledge_base::{
        KnowledgeBaseProviderRegistry, KnowledgeBaseProviderRegistryTrait,
        LazyKnowledgeBaseProviderRegistry, LazyRegistryConfig,
    },
    llm::LlmProviderFactory,
    operation::{InMemoryOperationRepository, StorageOperationRepository},
    plugin::{register_builtin_plugins, PluginRegistry, ProviderRouter, RoutingProviderResolver},
    services::{
        ConfigService, ExecutionLogService, ExperimentService, IngestionService,
        KnowledgeBaseService, ModelService, OperationService, PromptService, TestCaseService,
        TestCaseServiceDeps, WorkflowService,
    },
    storage::{InMemoryStorage, StorageFactory},
    team::{StorageTeamRepository, TeamService},
    test_case::{
        InMemoryTestCaseRepository, InMemoryTestCaseResultRepository,
        StorageTestCaseRepository, StorageTestCaseResultRepository,
    },
    usage::{
        BudgetService, InMemoryBudgetRepository, InMemoryUsageRepository,
        StorageBudgetRepository, StorageUsageRepository, UsageTrackingService,
    },
    user::{Argon2Hasher, CreateUserRequest, PostgresUserRepository, UserService},
    webhook::{
        InMemoryWebhookDeliveryRepository, InMemoryWebhookRepository,
        StorageWebhookDeliveryRepository, StorageWebhookRepository, WebhookService,
    },
    workflow::WorkflowExecutorImpl,
};
use rand::Rng;
use tracing::info;

/// Create the application state with all services initialized
pub async fn create_app_state() -> anyhow::Result<AppState> {
    create_app_state_with_config(&AppConfig::default()).await
}

/// Create the application state with custom configuration
pub async fn create_app_state_with_config(config: &AppConfig) -> anyhow::Result<AppState> {
    use domain::api_key::ApiKey;
    use domain::experiment::{Experiment, ExperimentRecord};
    use domain::operation::Operation;
    use domain::test_case::{TestCase, TestCaseResult};
    use domain::usage::{Budget, UsageRecord};
    use domain::webhook::{Webhook, WebhookDelivery};
    use infrastructure::storage::StorageType;

    let llm_provider = create_llm_provider()?;

    // Determine storage backend from config
    let storage_backend = StorageType::from_str(&config.storage.backend)
        .unwrap_or(StorageType::InMemory);
    let use_postgres = storage_backend == StorageType::Postgres;

    info!("Storage backend: {:?}", storage_backend);

    // Warn if using in-memory storage in production - this should only happen in tests
    if !use_postgres {
        tracing::warn!(
            "Using in-memory storage. This should only be used for testing. \
             Set APP__STORAGE__BACKEND=postgres for production."
        );
    }

    // PostgreSQL connection - required for user persistence and optionally for all storage
    let database_url = std::env::var("DATABASE_URL")
        .map_err(|_| anyhow::anyhow!("DATABASE_URL environment variable is required"))?;

    info!("Connecting to PostgreSQL...");
    let pg_pool = sqlx::PgPool::connect(&database_url)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to connect to PostgreSQL: {}", e))?;
    info!("PostgreSQL connection established");

    use domain::storage::Storage as StorageTrait;

    // Create services based on storage backend selection
    // Each service wraps its storage internally and implements a common trait
    let (model_service, prompt_service): (
        Arc<dyn api::state::ModelServiceTrait>,
        Arc<dyn api::state::PromptServiceTrait>,
    ) = if use_postgres {
        info!("Using PostgreSQL storage for entities");
        let model_storage =
            StorageFactory::create_postgres_with_pool::<Model>(pg_pool.clone(), "models");
        let prompt_storage =
            StorageFactory::create_postgres_with_pool::<Prompt>(pg_pool.clone(), "prompts");
        (
            Arc::new(ModelService::new(model_storage)),
            Arc::new(PromptService::new(prompt_storage)),
        )
    } else {
        info!("Using in-memory storage for entities");
        let model_storage = Arc::new(InMemoryStorage::<Model>::new());
        let prompt_storage = Arc::new(InMemoryStorage::<Prompt>::new());
        (
            Arc::new(ModelService::new(model_storage)),
            Arc::new(PromptService::new(prompt_storage)),
        )
    };

    // Create workflow and team storage - needed for other services
    let (workflow_storage, team_storage, knowledge_base_storage): (
        Arc<dyn StorageTrait<Workflow>>,
        Arc<dyn StorageTrait<Team>>,
        Arc<dyn StorageTrait<KnowledgeBase>>,
    ) = if use_postgres {
        (
            StorageFactory::create_postgres_with_pool::<Workflow>(pg_pool.clone(), "workflows"),
            StorageFactory::create_postgres_with_pool::<Team>(pg_pool.clone(), "teams"),
            StorageFactory::create_postgres_with_pool::<KnowledgeBase>(
                pg_pool.clone(),
                "knowledge_bases",
            ),
        )
    } else {
        (
            Arc::new(InMemoryStorage::<Workflow>::new()),
            Arc::new(InMemoryStorage::<Team>::new()),
            Arc::new(InMemoryStorage::<KnowledgeBase>::new()),
        )
    };

    // Model storage for lazy registry (needs concrete type for generic bounds)
    let model_storage_for_kb: Arc<dyn StorageTrait<Model>> = if use_postgres {
        StorageFactory::create_postgres_with_pool::<Model>(pg_pool.clone(), "models")
    } else {
        Arc::new(InMemoryStorage::<Model>::new())
    };

    // Prompt storage for workflow executor (needs concrete type)
    let prompt_storage_for_workflow: Arc<dyn StorageTrait<Prompt>> = if use_postgres {
        StorageFactory::create_postgres_with_pool::<Prompt>(pg_pool.clone(), "prompts")
    } else {
        Arc::new(InMemoryStorage::<Prompt>::new())
    };

    // Credential service - needed for provider resolution
    // We need both infrastructure and api::state trait versions
    let (credential_service_infra, credential_service): (
        Arc<dyn infrastructure::credentials::CredentialServiceTrait>,
        Arc<dyn api::state::CredentialServiceTrait>,
    ) = if use_postgres {
        let storage = StorageFactory::create_postgres_with_pool::<StoredCredential>(
            pg_pool.clone(),
            "credentials",
        );
        let service = Arc::new(CredentialService::new(Arc::new(
            StorageStoredCredentialRepository::new(storage),
        )));
        (service.clone(), service)
    } else {
        let service = Arc::new(CredentialService::new(Arc::new(
            InMemoryStoredCredentialRepository::new(),
        )));
        (service.clone(), service)
    };

    // Plugin system - register built-in providers (must be before workflow executor)
    let plugin_registry = PluginRegistry::new();
    let provider_router = Arc::new(ProviderRouter::new());

    // Provider resolver for workflow execution - routes to appropriate provider per model
    let provider_resolver = Arc::new(RoutingProviderResolver::new(
        model_service.clone(),
        credential_service.clone(),
        provider_router.clone(),
        llm_provider.clone(),
    ));

    // API Key service
    let api_key_service: Arc<dyn api::state::ApiKeyServiceTrait> = if use_postgres {
        let storage =
            StorageFactory::create_postgres_with_pool::<ApiKey>(pg_pool.clone(), "api_keys");
        Arc::new(
            ApiKeyService::new(Arc::new(StorageApiKeyRepository::new(storage)))
                .with_generator(ApiKeyGenerator::new("pk_test_")),
        )
    } else {
        Arc::new(
            ApiKeyService::new(Arc::new(InMemoryApiKeyRepository::new()))
                .with_generator(ApiKeyGenerator::new("pk_test_")),
        )
    };

    if let Ok(admin_key) = std::env::var("ADMIN_API_KEY") {
        create_admin_api_key(api_key_service.as_ref(), &admin_key).await?;
    }

    // External API service - needed for workflow executor
    // We need both infrastructure and api::state trait versions
    let (external_api_service_infra, external_api_service): (
        Arc<dyn infrastructure::external_api::ExternalApiServiceTrait>,
        Arc<dyn api::state::ExternalApiServiceTrait>,
    ) = if use_postgres {
        let storage = StorageFactory::create_postgres_with_pool::<domain::ExternalApi>(
            pg_pool.clone(),
            "external_apis",
        );
        let service = Arc::new(external_api::ExternalApiService::new(storage));
        (service.clone(), service)
    } else {
        let service = Arc::new(external_api::ExternalApiService::new(Arc::new(
            InMemoryStorage::<domain::ExternalApi>::new(),
        )));
        (service.clone(), service)
    };

    // Knowledge base provider registry - needed for workflow KB search steps
    let inner_registry = Arc::new(KnowledgeBaseProviderRegistry::new());
    let lazy_config = LazyRegistryConfig::new().with_pg_pool(pg_pool.clone());

    let kb_provider_registry: Arc<dyn KnowledgeBaseProviderRegistryTrait> = Arc::new(
        LazyKnowledgeBaseProviderRegistry::new(
            inner_registry,
            knowledge_base_storage.clone(),
            model_storage_for_kb.clone(),
            credential_service_infra.clone(),
            lazy_config,
        ),
    );

    let workflow_executor: Arc<dyn domain::WorkflowExecutor> = Arc::new(WorkflowExecutorImpl::new(
        provider_resolver,
        prompt_storage_for_workflow.clone(),
        credential_service_infra.clone(),
        external_api_service_infra.clone(),
        kb_provider_registry.clone(),
    ));
    let workflow_service = Arc::new(WorkflowService::new(workflow_storage.clone(), workflow_executor));

    // Operation service
    let operation_service: Arc<dyn api::state::OperationServiceTrait> = if use_postgres {
        let storage =
            StorageFactory::create_postgres_with_pool::<Operation>(pg_pool.clone(), "operations");
        Arc::new(OperationService::new(Arc::new(
            StorageOperationRepository::new(storage),
        )))
    } else {
        Arc::new(OperationService::new(Arc::new(
            InMemoryOperationRepository::new(),
        )))
    };

    // Team service - must be initialized before users and API keys
    let team_repository = Arc::new(StorageTeamRepository::new(team_storage));
    let team_service = Arc::new(TeamService::new(team_repository));

    // Ensure administrators team exists before creating users/API keys
    team_service.ensure_administrators_team().await?;

    // User authentication services - PostgreSQL required for persistence
    let user_repository = Arc::new(PostgresUserRepository::new(pg_pool.clone()));
    let password_hasher = Arc::new(Argon2Hasher::new());
    let user_service: Arc<dyn api::state::UserServiceTrait> =
        Arc::new(UserService::new(user_repository, password_hasher));

    // Create initial admin user if no users exist
    create_initial_admin_user(user_service.as_ref()).await?;

    // Knowledge base service
    let knowledge_base_service = Arc::new(KnowledgeBaseService::new(knowledge_base_storage.clone()));

    // Create embedding config for dynamic provider creation
    let embedding_config = infrastructure::services::EmbeddingConfig::new(
        knowledge_base_storage.clone(),
        model_storage_for_kb.clone(),
        credential_service_infra.clone(),
    );

    // Document ingestion service - uses the kb_provider_registry created earlier
    let ingestion_service = Arc::new(IngestionService::with_embedding_config(
        kb_provider_registry.clone(),
        embedding_config,
    ));

    // Usage tracking and budget services
    let usage_service: Arc<dyn api::state::UsageServiceTrait> = if use_postgres {
        let storage =
            StorageFactory::create_postgres_with_pool::<UsageRecord>(pg_pool.clone(), "usage_records");
        Arc::new(UsageTrackingService::new(Arc::new(
            StorageUsageRepository::new(storage),
        )))
    } else {
        Arc::new(UsageTrackingService::new(Arc::new(
            InMemoryUsageRepository::default(),
        )))
    };

    let budget_service: Arc<dyn api::state::BudgetServiceStateTrait> = if use_postgres {
        let storage =
            StorageFactory::create_postgres_with_pool::<Budget>(pg_pool.clone(), "budgets");
        Arc::new(BudgetService::new(Arc::new(StorageBudgetRepository::new(
            storage,
        ))))
    } else {
        Arc::new(BudgetService::new(Arc::new(InMemoryBudgetRepository::new())))
    };

    // Experiment (A/B testing) service
    let experiment_service: Arc<dyn api::state::ExperimentServiceTrait> = if use_postgres {
        let exp_storage =
            StorageFactory::create_postgres_with_pool::<Experiment>(pg_pool.clone(), "experiments");
        let record_storage = StorageFactory::create_postgres_with_pool::<ExperimentRecord>(
            pg_pool.clone(),
            "experiment_records",
        );
        Arc::new(ExperimentService::new(
            Arc::new(StorageExperimentRepository::new(exp_storage)),
            Arc::new(StorageExperimentRecordRepository::new(record_storage)),
        ))
    } else {
        Arc::new(ExperimentService::new(
            Arc::new(InMemoryExperimentRepository::new()),
            Arc::new(InMemoryExperimentRecordRepository::new()),
        ))
    };

    // Test case service
    let test_case_deps = TestCaseServiceDeps {
        model_service: model_service.clone(),
        prompt_service: prompt_service.clone(),
        workflow_service: workflow_service.clone(),
        credential_service: credential_service.clone(),
        provider_router: provider_router.clone(),
    };

    let test_case_service: Arc<dyn api::state::TestCaseServiceTrait> = if use_postgres {
        let tc_storage =
            StorageFactory::create_postgres_with_pool::<TestCase>(pg_pool.clone(), "test_cases");
        let result_storage = StorageFactory::create_postgres_with_pool::<TestCaseResult>(
            pg_pool.clone(),
            "test_case_results",
        );
        Arc::new(TestCaseService::new(
            Arc::new(StorageTestCaseRepository::new(tc_storage)),
            Arc::new(StorageTestCaseResultRepository::new(result_storage)),
            test_case_deps,
        ))
    } else {
        Arc::new(TestCaseService::new(
            Arc::new(InMemoryTestCaseRepository::new()),
            Arc::new(InMemoryTestCaseResultRepository::new()),
            test_case_deps,
        ))
    };

    if let Err(errors) = register_builtin_plugins(&plugin_registry, &provider_router).await {
        for error in &errors {
            tracing::error!("Failed to register plugin: {}", error);
        }
        if !errors.is_empty() {
            tracing::warn!("Some plugins failed to register, continuing with available plugins");
        }
    }

    // JWT service - prefer JWKS, fallback to secret
    let jwt_expiration = u64::from(config.auth.jwt_expiration_hours);
    let jwt_service: Arc<dyn api::state::JwtServiceTrait> =
        if let Ok(jwks_json) = std::env::var("USERS_JWKS") {
            tracing::info!("Using JWKS for JWT token generation and validation");
            match JwksJwtService::from_jwks_json(&jwks_json, jwt_expiration) {
                Ok(service) => Arc::new(service),
                Err(e) => {
                    tracing::error!("Failed to parse USERS_JWKS: {}. Falling back to secret.", e);
                    create_jwt_service_from_secret(&config, jwt_expiration)
                }
            }
        } else {
            create_jwt_service_from_secret(&config, jwt_expiration)
        };

    // Configuration service
    let config_repository: Arc<dyn domain::ConfigRepository> = if use_postgres {
        Arc::new(PostgresConfigRepository::new(pg_pool.clone()))
    } else {
        Arc::new(InMemoryConfigRepository::with_defaults())
    };
    let config_service = Arc::new(ConfigService::new(config_repository.clone()));

    // Execution log service
    let execution_log_storage: Arc<dyn StorageTrait<ExecutionLog>> = if use_postgres {
        StorageFactory::create_postgres_with_pool::<ExecutionLog>(pg_pool.clone(), "execution_logs")
    } else {
        Arc::new(InMemoryStorage::<ExecutionLog>::new())
    };
    let execution_log_repository = Arc::new(StorageExecutionLogRepository::new(execution_log_storage));
    let execution_log_service = Arc::new(ExecutionLogService::new(
        execution_log_repository,
        config_repository,
    ));

    // Webhook service
    let webhook_service: Arc<dyn api::state::WebhookServiceStateTrait> = if use_postgres {
        let wh_storage =
            StorageFactory::create_postgres_with_pool::<Webhook>(pg_pool.clone(), "webhooks");
        let delivery_storage = StorageFactory::create_postgres_with_pool::<WebhookDelivery>(
            pg_pool.clone(),
            "webhook_deliveries",
        );
        Arc::new(WebhookService::new(
            Arc::new(StorageWebhookRepository::new(wh_storage)),
            Arc::new(StorageWebhookDeliveryRepository::new(delivery_storage)),
        ))
    } else {
        Arc::new(WebhookService::new(
            Arc::new(InMemoryWebhookRepository::new()),
            Arc::new(InMemoryWebhookDeliveryRepository::new()),
        ))
    };

    Ok(AppState::new(
        model_service,
        prompt_service,
        api_key_service,
        workflow_service,
        operation_service,
        user_service,
        team_service,
        jwt_service,
        credential_service,
        external_api_service,
        knowledge_base_service,
        ingestion_service,
        usage_service,
        budget_service,
        experiment_service,
        test_case_service,
        config_service,
        execution_log_service,
        webhook_service,
        llm_provider,
        provider_router,
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
    info!("Using Anthropic provider");
    Ok(LlmProviderFactory::create_anthropic(api_key))
}

fn create_azure_provider() -> anyhow::Result<Arc<dyn domain::LlmProvider>> {
    let api_key = std::env::var("AZURE_API_KEY").unwrap_or_else(|_| "placeholder".to_string());
    let endpoint = std::env::var("AZURE_ENDPOINT")
        .unwrap_or_else(|_| "https://placeholder.openai.azure.com".to_string());

    info!("Using Azure OpenAI provider");
    Ok(LlmProviderFactory::create_azure_openai(endpoint, api_key))
}

/// Create an admin API key with full permissions
async fn create_admin_api_key(
    api_key_service: &dyn api::state::ApiKeyServiceTrait,
    key_value: &str,
) -> anyhow::Result<()> {
    use domain::team::TeamId;

    // Create the API key with the provided value
    let permissions = ApiKeyPermissions::full_access();
    let api_key = api_key_service
        .create_with_known_secret(
            "admin",
            "Admin Key",
            key_value,
            TeamId::administrators().as_str(),
            permissions,
        )
        .await?;

    info!("Admin API key created with ID: {}", api_key.id());
    Ok(())
}

/// Generate a random JWT secret
fn generate_random_secret() -> String {
    use rand::distributions::Alphanumeric;

    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(64)
        .map(char::from)
        .collect()
}

/// Create JWT service from secret (config, env var, or random)
fn create_jwt_service_from_secret(
    config: &config::AppConfig,
    jwt_expiration: u64,
) -> Arc<dyn api::state::JwtServiceTrait> {
    let jwt_secret = config
        .auth
        .jwt_secret
        .clone()
        .or_else(|| std::env::var("JWT_SECRET").ok())
        .unwrap_or_else(|| {
            tracing::warn!(
                "No USERS_JWKS or JWT_SECRET configured. Generating random secret. \
                Sessions will NOT persist across restarts. \
                Set USERS_JWKS environment variable for persistent sessions."
            );
            generate_random_secret()
        });

    Arc::new(JwtService::new(JwtConfig::new(jwt_secret, jwt_expiration)))
}

/// Generate a random password for the initial admin user
fn generate_random_password() -> String {
    use rand::distributions::Alphanumeric;

    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(16)
        .map(char::from)
        .collect()
}

/// Create an initial admin user if no users exist
async fn create_initial_admin_user(
    user_service: &dyn api::state::UserServiceTrait,
) -> anyhow::Result<()> {
    use domain::team::{TeamId, TeamRole};

    // Check if any users exist
    if user_service.count(None).await? > 0 {
        return Ok(());
    }

    // Use ADMIN_DEFAULT_PASSWORD env var if set, otherwise generate random password
    let (password, is_default) = match std::env::var("ADMIN_DEFAULT_PASSWORD") {
        Ok(p) if !p.is_empty() => (p, true),
        _ => (generate_random_password(), false),
    };

    let request = CreateUserRequest {
        id: "admin".to_string(),
        username: "admin".to_string(),
        password: password.clone(),
        team_id: TeamId::administrators(),
        team_role: TeamRole::Owner,
    };

    user_service.create(request).await?;

    info!("===========================================");
    info!("Initial admin user created!");
    info!("Username: admin");

    if is_default {
        info!("Password: (set via ADMIN_DEFAULT_PASSWORD)");
    } else {
        info!("Password: {}", password);
    }

    info!("Please change this password after first login.");
    info!("===========================================");

    Ok(())
}

// ============================================================================
// Default Entities - TEST ONLY
// ============================================================================
// These functions provide default entities for testing purposes only.
// They are NOT used in production code. In production, all entities
// must be created via the admin API or seeded through migrations.
// ============================================================================

#[cfg(test)]
use domain::{
    credentials::CredentialId,
    knowledge_base::{EmbeddingConfig, KnowledgeBaseId, KnowledgeBaseType},
    workflow::{
        ChatCompletionStep, Condition, ConditionalAction, ConditionalStep, ConditionOperator,
        CragScoringStep, KnowledgeBaseSearchStep, WorkflowId, WorkflowStep, WorkflowStepType,
    },
    CredentialType, ModelId, PromptId,
};

#[cfg(test)]
pub(crate) fn default_models() -> Vec<Model> {
    vec![
        Model::new(
            ModelId::new("gpt-4").unwrap(),
            "GPT-4",
            CredentialType::OpenAi,
            "gpt-4",
            "openai-default",
        ),
        Model::new(
            ModelId::new("gpt-4-turbo").unwrap(),
            "GPT-4 Turbo",
            CredentialType::OpenAi,
            "gpt-4-turbo",
            "openai-default",
        ),
        Model::new(
            ModelId::new("gpt-35-turbo").unwrap(),
            "GPT-3.5 Turbo",
            CredentialType::OpenAi,
            "gpt-3.5-turbo",
            "openai-default",
        ),
        Model::new(
            ModelId::new("claude-3-opus").unwrap(),
            "Claude 3 Opus",
            CredentialType::Anthropic,
            "claude-3-opus-20240229",
            "anthropic-default",
        ),
        Model::new(
            ModelId::new("claude-3-sonnet").unwrap(),
            "Claude 3 Sonnet",
            CredentialType::Anthropic,
            "claude-3-sonnet-20240229",
            "anthropic-default",
        ),
    ]
}

#[cfg(test)]
pub(crate) fn default_prompts() -> Vec<Prompt> {
    vec![
        Prompt::new(
            PromptId::new("system-assistant").unwrap(),
            "System Assistant",
            "You are a helpful assistant.",
        ),
        Prompt::new(
            PromptId::new("code-reviewer").unwrap(),
            "Code Reviewer",
            "You are an expert code reviewer. Analyze the code and provide constructive feedback.",
        ),
        Prompt::new(
            PromptId::new("summarizer").unwrap(),
            "Summarizer",
            "Summarize the following text concisely while preserving key information.",
        ),
        Prompt::new(
            PromptId::new("translator").unwrap(),
            "Translator",
            "Translate the following text to ${var:target_language:English}.",
        ),
        // CRAG prompts
        Prompt::new(
            PromptId::new("crag-relevance-scorer").unwrap(),
            "CRAG Relevance Scorer",
            "You are a document relevance scorer. Your task is to evaluate how relevant each document is to the given query.\n\n\
             Scoring guide:\n\
             - 0.00: Not relevant at all\n\
             - 0.25: Some minor parts may be relevant\n\
             - 0.50: Moderately relevant\n\
             - 0.75: Mostly relevant\n\
             - 1.00: Completely relevant\n\n\
             Use any value between 0.00 and 1.00 with two decimal places.\n\n\
             IMPORTANT: Respond with a JSON object containing a \"scores\" array. \
             Each item in the array must have an \"id\" (the document ID) and a \"score\" (the relevance score).\n\n\
             Example response format:\n\
             {\"scores\": [{\"id\": \"doc-1\", \"score\": 0.85}, {\"id\": \"doc-2\", \"score\": 0.42}]}\n\n\
             Query: ${var:query}\n\nDocuments:\n${var:document}",
        ),
        Prompt::new(
            PromptId::new("crag-knowledge-refiner").unwrap(),
            "CRAG Knowledge Refiner",
            "Extract and refine the most relevant information from the following documents \
             to answer the query.\n\nQuery: ${var:query}\n\nDocuments:\n${var:documents}",
        ),
        Prompt::new(
            PromptId::new("crag-web-search-generator").unwrap(),
            "CRAG Web Search Query Generator",
            "Generate a concise web search query to find information about: ${var:query}",
        ),
        Prompt::new(
            PromptId::new("crag-final-answer").unwrap(),
            "CRAG Final Answer Generator",
            "Using the following knowledge, provide a comprehensive answer to the query.\n\n\
             Query: ${var:query}\n\nKnowledge:\n${var:knowledge}",
        ),
        // Workflow prompts
        Prompt::new(
            PromptId::new("rag-system").unwrap(),
            "RAG System Prompt",
            "You are a helpful assistant. Use the following context to answer questions.\n\n\
             Context:\n${var:context}\n\nIf the context doesn't contain relevant information, \
             say so and provide what help you can.",
        ),
        Prompt::new(
            PromptId::new("document-analyzer").unwrap(),
            "Document Analyzer",
            "Analyze the following document and extract key information:\n${var:document}",
        ),
        Prompt::new(
            PromptId::new("sentiment-analyzer").unwrap(),
            "Sentiment Analyzer",
            "Analyze the sentiment of the following text. Respond with: positive, negative, or neutral.\n\nText: ${var:text}",
        ),
        Prompt::new(
            PromptId::new("entity-extractor").unwrap(),
            "Entity Extractor",
            "Extract named entities (people, organizations, locations, dates) from the following text. \
             Return as JSON.\n\nText: ${var:text}",
        ),
        Prompt::new(
            PromptId::new("qa-generator").unwrap(),
            "Q&A Generator",
            "Generate questions and answers based on the following content:\n${var:content}",
        ),
        Prompt::new(
            PromptId::new("classification-prompt").unwrap(),
            "Classification Prompt",
            "Classify the following text into one of these categories: ${var:categories}\n\nText: ${var:text}\n\nRespond with only the category name.",
        ),
        Prompt::new(
            PromptId::new("chain-of-thought").unwrap(),
            "Chain of Thought",
            "Think through this problem step by step:\n${var:problem}\n\nShow your reasoning before giving the final answer.",
        ),
        Prompt::new(
            PromptId::new("few-shot-template").unwrap(),
            "Few-Shot Template",
            "Here are some examples:\n${var:examples}\n\nNow, following the same pattern:\n${var:input}",
        ),
        Prompt::new(
            PromptId::new("json-output").unwrap(),
            "JSON Output",
            "Based on the input, generate a JSON response with the following schema:\n${var:schema}\n\nInput: ${var:input}\n\nRespond with valid JSON only.",
        ),
    ]
}

#[cfg(test)]
pub(crate) fn default_credentials() -> Vec<StoredCredential> {
    vec![
        StoredCredential::new(
            CredentialId::new("openai-default").unwrap(),
            "OpenAI Default",
            CredentialType::OpenAi,
            std::env::var("OPENAI_API_KEY").unwrap_or_else(|_| "sk-placeholder".to_string()),
        ),
        StoredCredential::new(
            CredentialId::new("anthropic-default").unwrap(),
            "Anthropic Default",
            CredentialType::Anthropic,
            std::env::var("ANTHROPIC_API_KEY").unwrap_or_else(|_| "sk-placeholder".to_string()),
        ),
        StoredCredential::new(
            CredentialId::new("pgvector-default").unwrap(),
            "PgVector Default",
            CredentialType::Pgvector,
            std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgres://localhost/pmp_llm_gateway".to_string()),
        ),
    ]
}

#[cfg(test)]
pub(crate) fn default_knowledge_bases() -> Vec<KnowledgeBase> {
    use std::collections::HashMap;

    let mut connection_config = HashMap::new();
    connection_config.insert(
        "credential_id".to_string(),
        "pgvector-default".to_string(),
    );

    vec![KnowledgeBase::new(
        KnowledgeBaseId::new("default-kb").unwrap(),
        "Default Knowledge Base",
        KnowledgeBaseType::Pgvector,
        EmbeddingConfig::new("text-embedding-ada-002", 1536),
    )
    .with_connection_config(connection_config)]
}

#[cfg(test)]
pub(crate) fn default_workflows() -> Vec<Workflow> {
    use domain::workflow::ScoringStrategy;

    // Common input schemas
    let query_schema = serde_json::json!({
        "type": "object",
        "properties": {
            "query": {
                "type": "string",
                "description": "The search query or question"
            }
        },
        "required": ["query"]
    });

    let text_schema = serde_json::json!({
        "type": "object",
        "properties": {
            "text": {
                "type": "string",
                "description": "The input text to process"
            }
        },
        "required": ["text"]
    });

    let document_schema = serde_json::json!({
        "type": "object",
        "properties": {
            "document": {
                "type": "string",
                "description": "The document content to analyze"
            }
        },
        "required": ["document"]
    });

    let translate_schema = serde_json::json!({
        "type": "object",
        "properties": {
            "text": {
                "type": "string",
                "description": "The text to translate"
            },
            "target_language": {
                "type": "string",
                "description": "Target language for translation (e.g., 'Spanish', 'French')",
                "default": "English"
            }
        },
        "required": ["text"]
    });

    vec![
        // Basic RAG workflow
        Workflow::new(WorkflowId::new("basic-rag").unwrap(), "Basic RAG")
            .with_description("Simple retrieval-augmented generation workflow")
            .with_input_schema(query_schema.clone())
            .with_step(WorkflowStep::new(
                "search",
                WorkflowStepType::KnowledgeBaseSearch(
                    KnowledgeBaseSearchStep::new("default-kb", "${input.query}")
                        .with_top_k(5),
                ),
            ))
            .with_step(WorkflowStep::new(
                "generate",
                WorkflowStepType::ChatCompletion(
                    ChatCompletionStep::new("gpt-4", "rag-system")
                        .with_prompt_variable("query", "${input.query}")
                        .with_prompt_variable("context", "${step:search:documents}")
                        .with_temperature(0.7)
                        .with_max_tokens(1000),
                ),
            )),
        // CRAG workflow with scoring
        Workflow::new(WorkflowId::new("crag-pipeline").unwrap(), "CRAG Pipeline")
            .with_description("Corrective RAG with document scoring and refinement")
            .with_input_schema(query_schema.clone())
            .with_step(WorkflowStep::new(
                "search",
                WorkflowStepType::KnowledgeBaseSearch(
                    KnowledgeBaseSearchStep::new("default-kb", "${input.query}")
                        .with_top_k(10),
                ),
            ))
            .with_step(WorkflowStep::new(
                "score",
                WorkflowStepType::CragScoring(
                    CragScoringStep::new("gpt-4", "crag-relevance-scorer")
                        .with_documents_source("${step:search:documents}")
                        .with_threshold(0.7)
                        .with_strategy(ScoringStrategy::Hybrid)
                        .with_prompt_variables({
                            let mut vars = std::collections::HashMap::new();
                            // document: XML representation for the prompt
                            vars.insert("document".to_string(), "${step:search:documents_xml}".to_string());
                            vars.insert("query".to_string(), "${input.query}".to_string());
                            vars
                        }),
                ),
            ))
            .with_step(WorkflowStep::new(
                "answer",
                WorkflowStepType::ChatCompletion(
                    ChatCompletionStep::new("gpt-4", "crag-final-answer")
                        .with_prompt_variable("query", "${input.query}")
                        .with_prompt_variable("documents", "${step:score:relevant_documents}")
                        .with_temperature(0.7)
                        .with_max_tokens(1500),
                ),
            )),
        // Conditional workflow example
        Workflow::new(WorkflowId::new("conditional-router").unwrap(), "Conditional Router")
            .with_description("Routes to different models based on query complexity")
            .with_input_schema(query_schema)
            .with_step(WorkflowStep::new(
                "classify",
                WorkflowStepType::ChatCompletion(
                    ChatCompletionStep::new("gpt-3.5-turbo", "classification-prompt")
                        .with_prompt_variable("query", "${input.query}")
                        .with_temperature(0.1)
                        .with_max_tokens(50),
                ),
            ))
            .with_step(WorkflowStep::new(
                "route",
                WorkflowStepType::Conditional(
                    ConditionalStep::new(vec![Condition::new(
                        "${classify.response}",
                        ConditionOperator::Contains,
                        ConditionalAction::GoToStep("complex_answer".to_string()),
                    )
                    .with_value(serde_json::Value::String("complex".to_string()))])
                    .with_default_action(ConditionalAction::GoToStep("simple_answer".to_string())),
                ),
            ))
            .with_step(WorkflowStep::new(
                "simple_answer",
                WorkflowStepType::ChatCompletion(
                    ChatCompletionStep::new("gpt-3.5-turbo", "system-assistant")
                        .with_prompt_variable("query", "${input.query}")
                        .with_temperature(0.7)
                        .with_max_tokens(500),
                ),
            ))
            .with_step(WorkflowStep::new(
                "complex_answer",
                WorkflowStepType::ChatCompletion(
                    ChatCompletionStep::new("gpt-4", "chain-of-thought")
                        .with_prompt_variable("query", "${input.query}")
                        .with_temperature(0.7)
                        .with_max_tokens(2000),
                ),
            )),
        // Sentiment analysis workflow
        Workflow::new(WorkflowId::new("sentiment-analysis").unwrap(), "Sentiment Analysis")
            .with_description("Analyzes sentiment of input text")
            .with_input_schema(text_schema.clone())
            .with_step(WorkflowStep::new(
                "analyze",
                WorkflowStepType::ChatCompletion(
                    ChatCompletionStep::new("gpt-3.5-turbo", "sentiment-analyzer")
                        .with_prompt_variable("text", "${input.text}")
                        .with_temperature(0.1)
                        .with_max_tokens(50),
                ),
            )),
        // Entity extraction workflow
        Workflow::new(WorkflowId::new("entity-extraction").unwrap(), "Entity Extraction")
            .with_description("Extracts named entities from text")
            .with_input_schema(text_schema)
            .with_step(WorkflowStep::new(
                "extract",
                WorkflowStepType::ChatCompletion(
                    ChatCompletionStep::new("gpt-4", "entity-extractor")
                        .with_prompt_variable("text", "${input.text}")
                        .with_temperature(0.1)
                        .with_max_tokens(1000),
                ),
            )),
        // Multi-step document analysis
        Workflow::new(WorkflowId::new("document-analysis").unwrap(), "Document Analysis")
            .with_description("Multi-step document analysis with summary and Q&A generation")
            .with_input_schema(document_schema)
            .with_step(WorkflowStep::new(
                "summarize",
                WorkflowStepType::ChatCompletion(
                    ChatCompletionStep::new("gpt-4", "summarizer")
                        .with_prompt_variable("document", "${input.document}")
                        .with_temperature(0.5)
                        .with_max_tokens(500),
                ),
            ))
            .with_step(WorkflowStep::new(
                "extract_entities",
                WorkflowStepType::ChatCompletion(
                    ChatCompletionStep::new("gpt-4", "entity-extractor")
                        .with_prompt_variable("text", "${input.document}")
                        .with_temperature(0.1)
                        .with_max_tokens(1000),
                ),
            ))
            .with_step(WorkflowStep::new(
                "generate_qa",
                WorkflowStepType::ChatCompletion(
                    ChatCompletionStep::new("gpt-4", "qa-generator")
                        .with_prompt_variable("summary", "${step:summarize:response}")
                        .with_temperature(0.7)
                        .with_max_tokens(1500),
                ),
            )),
        // Translation workflow
        Workflow::new(WorkflowId::new("translate").unwrap(), "Translate")
            .with_description("Translates text to target language")
            .with_input_schema(translate_schema)
            .with_step(WorkflowStep::new(
                "translate",
                WorkflowStepType::ChatCompletion(
                    ChatCompletionStep::new("gpt-4", "translator")
                        .with_prompt_variable("text", "${input.text}")
                        .with_prompt_variable("target_language", "${input.target_language}")
                        .with_temperature(0.3)
                        .with_max_tokens(2000),
                ),
            )),
    ]
}
