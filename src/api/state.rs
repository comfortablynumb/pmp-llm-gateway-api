//! Application state for shared services

use std::sync::Arc;

use serde_json::Value;

use crate::domain::api_key::{ApiKeyPermissions, ApiKeyRepository};
use crate::domain::config::{ConfigCategory, ConfigEntry, ConfigValue, ExecutionLog, ExecutionLogQuery, ExecutionStats};
use crate::domain::credentials::StoredCredentialRepository;
use crate::domain::experiment::{
    AssignmentResult, Experiment, ExperimentQuery, ExperimentRecordRepository, ExperimentRepository,
    ExperimentResult, ExperimentStatus,
};
use crate::domain::llm::LlmProvider;
use crate::domain::operation::OperationRepository;
use crate::domain::user::{User, UserRepository, UserStatus};
use crate::domain::storage::Storage;
use crate::domain::usage::{
    Budget, BudgetId, BudgetRepository, ModelPricing, UsageAggregate, UsageQuery, UsageRecord,
    UsageRecordId, UsageRepository, UsageSummary,
};
use crate::domain::{
    ApiKey, DomainError, Executor, KnowledgeBase, Model, Operation, OperationType, Prompt,
    StoredCredential, Workflow, WorkflowResult,
};
use crate::infrastructure::api_key::ApiKeyService;
use crate::infrastructure::auth::{JwtClaims, JwtGenerator, JwksJwtService, JwtService};
use crate::infrastructure::credentials::{
    CreateCredentialRequest, CredentialService, UpdateCredentialRequest,
};
use crate::infrastructure::services::{
    ConfigService, CreateExperimentRequest, CreateKnowledgeBaseRequest, CreateModelRequest,
    CreatePromptRequest, CreateTestCaseRequest, CreateWorkflowRequest, CreateVariantRequest,
    ExecuteTestCaseResponse, ExecutionLogService, ExperimentService, IngestDocumentRequest,
    IngestDocumentV2Request, IngestionService, KnowledgeBaseService, ModelService, OperationService,
    PromptService, RecordExperimentParams, RecordExecutionParams, StoredDocument, TestCaseService,
    UpdateExperimentRequest, UpdateKnowledgeBaseRequest, UpdateModelRequest, UpdatePromptRequest,
    UpdateTestCaseRequest, UpdateWorkflowRequest, WorkflowService,
};
use crate::domain::knowledge_base::{DocumentChunk, DocumentSummary, KnowledgeBaseDocument};
use crate::domain::test_case::{
    TestCase, TestCaseQuery, TestCaseRepository, TestCaseResult, TestCaseResultQuery,
    TestCaseResultRepository,
};
use crate::infrastructure::plugin::ProviderRouter;
use crate::infrastructure::usage::{
    BudgetCheckResult, BudgetService, BudgetServiceTrait, RecordUsageParams, UsageTrackingService,
    UsageTrackingServiceTrait,
};
use crate::infrastructure::team::{CreateTeamRequest, TeamService, UpdateTeamRequest};
use crate::infrastructure::user::{
    CreateUserRequest, PasswordHasher, UpdatePasswordRequest, UserService,
};
use crate::infrastructure::webhook::{WebhookService, WebhookServiceTrait};
use crate::domain::team::{Team, TeamQuery, TeamRepository};
use crate::domain::webhook::{
    Webhook, WebhookDelivery, WebhookDeliveryRepository, WebhookRepository,
};
use crate::domain::ExternalApi;
use crate::infrastructure::external_api::{
    CreateExternalApiRequest, ExternalApiService, UpdateExternalApiRequest,
};

/// Application state containing shared services using dynamic dispatch
#[derive(Clone)]
pub struct AppState {
    pub model_service: Arc<dyn ModelServiceTrait>,
    pub prompt_service: Arc<dyn PromptServiceTrait>,
    pub api_key_service: Arc<dyn ApiKeyServiceTrait>,
    pub workflow_service: Arc<dyn WorkflowServiceTrait>,
    pub operation_service: Arc<dyn OperationServiceTrait>,
    pub user_service: Arc<dyn UserServiceTrait>,
    pub team_service: Arc<dyn TeamServiceTrait>,
    pub jwt_service: Arc<dyn JwtServiceTrait>,
    pub credential_service: Arc<dyn CredentialServiceTrait>,
    pub external_api_service: Arc<dyn ExternalApiServiceTrait>,
    pub knowledge_base_service: Arc<dyn KnowledgeBaseServiceTrait>,
    pub ingestion_service: Arc<dyn IngestionServiceTrait>,
    pub usage_service: Arc<dyn UsageServiceTrait>,
    pub budget_service: Arc<dyn BudgetServiceStateTrait>,
    pub experiment_service: Arc<dyn ExperimentServiceTrait>,
    pub test_case_service: Arc<dyn TestCaseServiceTrait>,
    pub config_service: Arc<dyn ConfigServiceTrait>,
    pub execution_log_service: Arc<dyn ExecutionLogServiceTrait>,
    pub webhook_service: Arc<dyn WebhookServiceStateTrait>,
    pub llm_provider: Arc<dyn LlmProvider>,
    pub provider_router: Arc<ProviderRouter>,
}

/// Trait for model service operations
#[async_trait::async_trait]
pub trait ModelServiceTrait: Send + Sync {
    async fn get(&self, id: &str) -> Result<Option<Model>, DomainError>;
    async fn list(&self) -> Result<Vec<Model>, DomainError>;
    async fn create(&self, request: CreateModelRequest) -> Result<Model, DomainError>;
    async fn update(&self, id: &str, request: UpdateModelRequest) -> Result<Model, DomainError>;
    async fn delete(&self, id: &str) -> Result<bool, DomainError>;
}

/// Trait for prompt service operations
#[async_trait::async_trait]
pub trait PromptServiceTrait: Send + Sync {
    async fn get(&self, id: &str) -> Result<Option<Prompt>, DomainError>;
    async fn list(&self) -> Result<Vec<Prompt>, DomainError>;
    async fn create(&self, request: CreatePromptRequest) -> Result<Prompt, DomainError>;
    async fn update(&self, id: &str, request: UpdatePromptRequest) -> Result<Prompt, DomainError>;
    async fn delete(&self, id: &str) -> Result<bool, DomainError>;
    async fn render(
        &self,
        id: &str,
        variables: &std::collections::HashMap<String, String>,
    ) -> Result<String, DomainError>;

    async fn revert(&self, id: &str, version: u32) -> Result<Prompt, DomainError>;
}

/// Trait for workflow service operations
#[async_trait::async_trait]
pub trait WorkflowServiceTrait: Send + Sync {
    async fn get(&self, id: &str) -> Result<Option<Workflow>, DomainError>;
    async fn list(&self) -> Result<Vec<Workflow>, DomainError>;
    async fn create(&self, request: CreateWorkflowRequest) -> Result<Workflow, DomainError>;
    async fn update(&self, id: &str, request: UpdateWorkflowRequest) -> Result<Workflow, DomainError>;
    async fn delete(&self, id: &str) -> Result<bool, DomainError>;
    async fn execute(&self, id: &str, input: Value) -> Result<WorkflowResult, DomainError>;
}

/// Trait for API key service operations
#[async_trait::async_trait]
pub trait ApiKeyServiceTrait: Send + Sync {
    async fn validate(&self, key: &str) -> Result<Option<ApiKey>, DomainError>;
    async fn get(&self, id: &str) -> Result<Option<ApiKey>, DomainError>;
    async fn list(&self) -> Result<Vec<ApiKey>, DomainError>;
    async fn create(
        &self,
        name: &str,
        team_id: &str,
        permissions: ApiKeyPermissions,
    ) -> Result<(ApiKey, String), DomainError>;
    /// Create an API key with a known secret value (for ADMIN_API_KEY)
    async fn create_with_known_secret(
        &self,
        id: &str,
        name: &str,
        secret: &str,
        team_id: &str,
        permissions: ApiKeyPermissions,
    ) -> Result<ApiKey, DomainError>;
    async fn update_permissions(
        &self,
        id: &str,
        permissions: ApiKeyPermissions,
    ) -> Result<(), DomainError>;
    async fn delete(&self, id: &str) -> Result<(), DomainError>;
    async fn suspend(&self, id: &str) -> Result<(), DomainError>;
    async fn activate(&self, id: &str) -> Result<(), DomainError>;
    async fn revoke(&self, id: &str) -> Result<(), DomainError>;
}

/// Trait for operation service (async operations)
#[async_trait::async_trait]
pub trait OperationServiceTrait: Send + Sync {
    /// Create a new pending operation
    async fn create_pending(
        &self,
        op_type: OperationType,
        input: Value,
        metadata: Value,
    ) -> Result<Operation, DomainError>;
    /// Get an operation by ID
    async fn get(&self, id: &str) -> Result<Option<Operation>, DomainError>;
    /// Get multiple operations by IDs
    async fn get_batch(&self, ids: &[String]) -> Result<Vec<Operation>, DomainError>;
    /// Mark an operation as running
    async fn mark_running(&self, id: &str) -> Result<Operation, DomainError>;
    /// Mark an operation as completed with result
    async fn mark_completed(&self, id: &str, result: Value) -> Result<Operation, DomainError>;
    /// Mark an operation as failed with error message
    async fn mark_failed(&self, id: &str, error: String) -> Result<Operation, DomainError>;
    /// Cancel an operation
    async fn cancel(&self, id: &str) -> Result<Operation, DomainError>;
    /// Clean up old completed operations
    async fn cleanup_old(&self) -> Result<u64, DomainError>;
}

/// Trait for user service operations
#[async_trait::async_trait]
pub trait UserServiceTrait: Send + Sync {
    /// Authenticate a user with username and password
    async fn authenticate(&self, username: &str, password: &str) -> Result<Option<User>, DomainError>;
    /// Get a user by ID
    async fn get(&self, id: &str) -> Result<Option<User>, DomainError>;
    /// Get a user by username
    async fn get_by_username(&self, username: &str) -> Result<Option<User>, DomainError>;
    /// Create a new user
    async fn create(&self, request: CreateUserRequest) -> Result<User, DomainError>;
    /// List all users
    async fn list(&self, status: Option<UserStatus>) -> Result<Vec<User>, DomainError>;
    /// Count users
    async fn count(&self, status: Option<UserStatus>) -> Result<usize, DomainError>;
    /// Update a user's password
    async fn update_password(&self, id: &str, request: UpdatePasswordRequest) -> Result<User, DomainError>;
    /// Suspend a user
    async fn suspend(&self, id: &str) -> Result<User, DomainError>;
    /// Activate a user
    async fn activate(&self, id: &str) -> Result<User, DomainError>;
    /// Delete a user
    async fn delete(&self, id: &str) -> Result<bool, DomainError>;
}

/// Trait for team service operations
#[async_trait::async_trait]
pub trait TeamServiceTrait: Send + Sync {
    /// Get a team by ID
    async fn get(&self, id: &str) -> Result<Option<Team>, DomainError>;
    /// List all teams
    async fn list(&self, query: Option<TeamQuery>) -> Result<Vec<Team>, DomainError>;
    /// Count teams
    async fn count(&self, query: Option<TeamQuery>) -> Result<usize, DomainError>;
    /// Create a new team
    async fn create(&self, request: CreateTeamRequest) -> Result<Team, DomainError>;
    /// Update a team
    async fn update(&self, id: &str, request: UpdateTeamRequest) -> Result<Team, DomainError>;
    /// Suspend a team
    async fn suspend(&self, id: &str) -> Result<Team, DomainError>;
    /// Activate a team
    async fn activate(&self, id: &str) -> Result<Team, DomainError>;
    /// Delete a team
    async fn delete(&self, id: &str) -> Result<bool, DomainError>;
    /// Check if a team exists
    async fn exists(&self, id: &str) -> Result<bool, DomainError>;
}

/// Trait for JWT service operations
pub trait JwtServiceTrait: Send + Sync {
    /// Generate a JWT token for a user
    fn generate(&self, user: &User) -> Result<String, DomainError>;
    /// Validate a JWT token and return the claims
    fn validate(&self, token: &str) -> Result<JwtClaims, DomainError>;
    /// Get the token expiration time in hours
    fn expiration_hours(&self) -> u64;
}

/// Trait for knowledge base service operations
#[async_trait::async_trait]
pub trait KnowledgeBaseServiceTrait: Send + Sync {
    /// Get a knowledge base by ID
    async fn get(&self, id: &str) -> Result<Option<KnowledgeBase>, DomainError>;
    /// List all knowledge bases
    async fn list(&self) -> Result<Vec<KnowledgeBase>, DomainError>;
    /// Create a new knowledge base
    async fn create(&self, request: CreateKnowledgeBaseRequest)
        -> Result<KnowledgeBase, DomainError>;
    /// Update a knowledge base
    async fn update(
        &self,
        id: &str,
        request: UpdateKnowledgeBaseRequest,
    ) -> Result<KnowledgeBase, DomainError>;
    /// Delete a knowledge base
    async fn delete(&self, id: &str) -> Result<bool, DomainError>;
    /// Check if a knowledge base exists
    async fn exists(&self, id: &str) -> Result<bool, DomainError>;
}

/// Trait for document ingestion service operations
#[async_trait::async_trait]
pub trait IngestionServiceTrait: Send + Sync {
    /// Ingest a document into a knowledge base
    async fn ingest(
        &self,
        kb_id: &str,
        request: IngestDocumentRequest,
    ) -> Result<crate::domain::ingestion::IngestionResult, DomainError>;
    /// List all sources in a knowledge base
    async fn list_sources(
        &self,
        kb_id: &str,
    ) -> Result<Vec<crate::domain::knowledge_base::SourceInfo>, DomainError>;
    /// Get documents by source ID
    async fn get_documents_by_source(
        &self,
        kb_id: &str,
        source: &str,
    ) -> Result<Vec<StoredDocument>, DomainError>;
    /// Get document count for a knowledge base
    async fn document_count(&self, kb_id: &str) -> Result<usize, DomainError>;
    /// Delete documents by source ID
    async fn delete_by_source(&self, kb_id: &str, source: &str) -> Result<usize, DomainError>;
    /// Ensure the storage schema exists (create tables/indexes)
    async fn ensure_schema(&self, kb_id: &str) -> Result<(), DomainError>;

    // New schema methods (document/chunk separation)
    /// Ingest a document using the new schema
    async fn ingest_document(
        &self,
        kb_id: &str,
        request: IngestDocumentV2Request,
    ) -> Result<KnowledgeBaseDocument, DomainError>;
    /// List all documents in a knowledge base (new schema)
    async fn list_documents_v2(&self, kb_id: &str) -> Result<Vec<DocumentSummary>, DomainError>;
    /// Get a document by ID (new schema)
    async fn get_document_v2(
        &self,
        kb_id: &str,
        document_id: uuid::Uuid,
    ) -> Result<Option<KnowledgeBaseDocument>, DomainError>;
    /// Get chunks for a document (new schema)
    async fn get_document_chunks(
        &self,
        kb_id: &str,
        document_id: uuid::Uuid,
    ) -> Result<Vec<DocumentChunk>, DomainError>;
    /// Delete a document by ID (new schema)
    async fn delete_document_v2(
        &self,
        kb_id: &str,
        document_id: uuid::Uuid,
    ) -> Result<bool, DomainError>;
    /// Disable a document (new schema)
    async fn disable_document(
        &self,
        kb_id: &str,
        document_id: uuid::Uuid,
    ) -> Result<bool, DomainError>;
    /// Enable a document (new schema)
    async fn enable_document(
        &self,
        kb_id: &str,
        document_id: uuid::Uuid,
    ) -> Result<bool, DomainError>;
}

/// Trait for credential service operations
#[async_trait::async_trait]
pub trait CredentialServiceTrait: Send + Sync {
    /// Get a credential by ID
    async fn get(&self, id: &str) -> Result<Option<StoredCredential>, DomainError>;
    /// List all credentials
    async fn list(&self) -> Result<Vec<StoredCredential>, DomainError>;
    /// Create a new credential
    async fn create(
        &self,
        request: CreateCredentialRequest,
    ) -> Result<StoredCredential, DomainError>;
    /// Update a credential
    async fn update(
        &self,
        id: &str,
        request: UpdateCredentialRequest,
    ) -> Result<StoredCredential, DomainError>;
    /// Delete a credential
    async fn delete(&self, id: &str) -> Result<(), DomainError>;
    /// Check if a credential exists
    async fn exists(&self, id: &str) -> Result<bool, DomainError>;
}

/// Trait for external API service operations
#[async_trait::async_trait]
pub trait ExternalApiServiceTrait: Send + Sync {
    /// Get an external API by ID
    async fn get(&self, id: &str) -> Result<Option<ExternalApi>, DomainError>;
    /// List all external APIs
    async fn list(&self) -> Result<Vec<ExternalApi>, DomainError>;
    /// Create a new external API
    async fn create(&self, request: CreateExternalApiRequest) -> Result<ExternalApi, DomainError>;
    /// Update an external API
    async fn update(
        &self,
        id: &str,
        request: UpdateExternalApiRequest,
    ) -> Result<ExternalApi, DomainError>;
    /// Delete an external API
    async fn delete(&self, id: &str) -> Result<(), DomainError>;
    /// Check if an external API exists
    async fn exists(&self, id: &str) -> Result<bool, DomainError>;
}

/// Trait for usage service operations
#[async_trait::async_trait]
pub trait UsageServiceTrait: Send + Sync {
    /// Record a usage event
    async fn record(&self, params: RecordUsageParams) -> Result<UsageRecord, DomainError>;
    /// Get a usage record by ID
    async fn get(&self, id: &UsageRecordId) -> Result<Option<UsageRecord>, DomainError>;
    /// Query usage records
    async fn query(&self, query: &UsageQuery) -> Result<Vec<UsageRecord>, DomainError>;
    /// Count usage records matching query
    async fn count(&self, query: &UsageQuery) -> Result<usize, DomainError>;
    /// Get aggregated usage
    async fn aggregate(&self, query: &UsageQuery) -> Result<UsageAggregate, DomainError>;
    /// Get usage summary with daily breakdown
    async fn summary(&self, query: &UsageQuery) -> Result<UsageSummary, DomainError>;
    /// Delete records older than timestamp
    async fn delete_before(&self, timestamp: u64) -> Result<usize, DomainError>;
    /// Delete all records for an API key
    async fn delete_by_api_key(&self, api_key_id: &str) -> Result<usize, DomainError>;
    /// Get pricing for a model
    fn get_pricing(&self, model_id: &str) -> Option<ModelPricing>;
    /// Calculate cost for tokens
    fn calculate_cost(&self, model_id: &str, input_tokens: u32, output_tokens: u32) -> i64;
}

/// Trait for budget service operations (state version to avoid name collision)
#[async_trait::async_trait]
pub trait BudgetServiceStateTrait: Send + Sync {
    /// Create a new budget
    async fn create(&self, budget: Budget) -> Result<Budget, DomainError>;
    /// Get a budget by ID
    async fn get(&self, id: &BudgetId) -> Result<Option<Budget>, DomainError>;
    /// Update a budget
    async fn update(&self, budget: Budget) -> Result<Budget, DomainError>;
    /// Delete a budget
    async fn delete(&self, id: &BudgetId) -> Result<bool, DomainError>;
    /// List all budgets
    async fn list(&self) -> Result<Vec<Budget>, DomainError>;
    /// List budgets by team
    async fn list_by_team(&self, team_id: &str) -> Result<Vec<Budget>, DomainError>;
    /// Check if a request is allowed based on budgets (no team context)
    async fn check_budget(
        &self,
        api_key_id: &str,
        model_id: Option<&str>,
        estimated_cost_micros: i64,
    ) -> Result<BudgetCheckResult, DomainError>;
    /// Check if a request is allowed based on budgets (with team context)
    async fn check_budget_with_team(
        &self,
        api_key_id: &str,
        team_id: Option<&str>,
        model_id: Option<&str>,
        estimated_cost_micros: i64,
    ) -> Result<BudgetCheckResult, DomainError>;
}

/// Trait for experiment service operations (A/B testing)
#[async_trait::async_trait]
pub trait ExperimentServiceTrait: Send + Sync {
    /// Get an experiment by ID
    async fn get(&self, id: &str) -> Result<Option<Experiment>, DomainError>;
    /// List experiments with optional query filter
    async fn list(&self, query: Option<ExperimentQuery>) -> Result<Vec<Experiment>, DomainError>;
    /// Create a new experiment
    async fn create(&self, request: CreateExperimentRequest) -> Result<Experiment, DomainError>;
    /// Update an experiment
    async fn update(
        &self,
        id: &str,
        request: UpdateExperimentRequest,
    ) -> Result<Experiment, DomainError>;
    /// Delete an experiment
    async fn delete(&self, id: &str) -> Result<bool, DomainError>;
    /// Add a variant to an experiment
    async fn add_variant(
        &self,
        experiment_id: &str,
        request: CreateVariantRequest,
    ) -> Result<Experiment, DomainError>;
    /// Remove a variant from an experiment
    async fn remove_variant(
        &self,
        experiment_id: &str,
        variant_id: &str,
    ) -> Result<Experiment, DomainError>;
    /// Start an experiment (Draft -> Active)
    async fn start(&self, id: &str) -> Result<Experiment, DomainError>;
    /// Pause an experiment (Active -> Paused)
    async fn pause(&self, id: &str) -> Result<Experiment, DomainError>;
    /// Resume an experiment (Paused -> Active)
    async fn resume(&self, id: &str) -> Result<Experiment, DomainError>;
    /// Complete an experiment (Active/Paused -> Completed)
    async fn complete(&self, id: &str) -> Result<Experiment, DomainError>;
    /// Assign a variant for a given model and API key
    async fn assign_variant(
        &self,
        model_id: &str,
        api_key_id: &str,
    ) -> Result<Option<AssignmentResult>, DomainError>;
    /// Record an experiment request
    async fn record(&self, params: RecordExperimentParams) -> Result<(), DomainError>;
    /// Get experiment results with statistical analysis
    async fn get_results(&self, id: &str) -> Result<ExperimentResult, DomainError>;
    /// Find experiments by status
    async fn find_by_status(
        &self,
        status: ExperimentStatus,
    ) -> Result<Vec<Experiment>, DomainError>;
}

/// Trait for test case service operations
#[async_trait::async_trait]
pub trait TestCaseServiceTrait: Send + Sync {
    /// Get a test case by ID
    async fn get(&self, id: &str) -> Result<Option<TestCase>, DomainError>;
    /// List test cases matching query
    async fn list(&self, query: &TestCaseQuery) -> Result<Vec<TestCase>, DomainError>;
    /// Count test cases matching query
    async fn count(&self, query: &TestCaseQuery) -> Result<usize, DomainError>;
    /// Create a new test case
    async fn create(&self, request: CreateTestCaseRequest) -> Result<TestCase, DomainError>;
    /// Update a test case
    async fn update(
        &self,
        id: &str,
        request: UpdateTestCaseRequest,
    ) -> Result<TestCase, DomainError>;
    /// Delete a test case
    async fn delete(&self, id: &str) -> Result<bool, DomainError>;
    /// Execute a test case
    async fn execute(&self, id: &str) -> Result<ExecuteTestCaseResponse, DomainError>;
    /// Get results for a test case
    async fn get_results(
        &self,
        id: &str,
        query: &TestCaseResultQuery,
    ) -> Result<Vec<TestCaseResult>, DomainError>;
    /// Get the latest result for a test case
    async fn get_latest_result(&self, id: &str) -> Result<Option<TestCaseResult>, DomainError>;
}

/// Trait for configuration service operations
#[async_trait::async_trait]
pub trait ConfigServiceTrait: Send + Sync {
    /// Get all configuration entries
    async fn list(&self) -> Result<Vec<ConfigEntry>, DomainError>;
    /// Get configuration entries by category
    async fn list_by_category(&self, category: ConfigCategory)
        -> Result<Vec<ConfigEntry>, DomainError>;
    /// Get a specific configuration entry
    async fn get_entry(&self, key: &str) -> Result<Option<ConfigEntry>, DomainError>;
    /// Get a configuration value
    async fn get_value(&self, key: &str) -> Result<Option<ConfigValue>, DomainError>;
    /// Set a configuration value
    async fn set(&self, key: &str, value: ConfigValue) -> Result<(), DomainError>;
    /// Reset configuration to defaults
    async fn reset(&self) -> Result<(), DomainError>;
}

/// Trait for execution log service operations
#[async_trait::async_trait]
pub trait ExecutionLogServiceTrait: Send + Sync {
    /// Record an execution
    async fn record(
        &self,
        params: RecordExecutionParams,
    ) -> Result<Option<ExecutionLog>, DomainError>;
    /// Get an execution log by ID
    async fn get(&self, id: &str) -> Result<Option<ExecutionLog>, DomainError>;
    /// List execution logs with filtering
    async fn list(&self, query: &ExecutionLogQuery) -> Result<Vec<ExecutionLog>, DomainError>;
    /// Count execution logs matching query
    async fn count(&self, query: &ExecutionLogQuery) -> Result<usize, DomainError>;
    /// Delete an execution log
    async fn delete(&self, id: &str) -> Result<bool, DomainError>;
    /// Delete logs older than configured retention period
    async fn cleanup_old_logs(&self) -> Result<usize, DomainError>;
    /// Delete logs older than specified days
    async fn delete_older_than(&self, days: i64) -> Result<usize, DomainError>;
    /// Get execution statistics
    async fn stats(&self, query: &ExecutionLogQuery) -> Result<ExecutionStats, DomainError>;
    /// Update an existing execution log
    async fn update(&self, log: &ExecutionLog) -> Result<(), DomainError>;
    /// Record a pending ingestion operation
    async fn record_pending_ingestion(
        &self,
        kb_id: &str,
        source_name: &str,
        executor: Executor,
        input: serde_json::Value,
    ) -> Result<ExecutionLog, DomainError>;
}

/// Trait for webhook service operations (state version to avoid name collision)
#[async_trait::async_trait]
pub trait WebhookServiceStateTrait: Send + Sync {
    /// Create a new webhook
    async fn create(&self, webhook: Webhook) -> Result<Webhook, DomainError>;
    /// Update a webhook
    async fn update(&self, id: &str, webhook: Webhook) -> Result<Webhook, DomainError>;
    /// Delete a webhook
    async fn delete(&self, id: &str) -> Result<(), DomainError>;
    /// Get a webhook by ID
    async fn get(&self, id: &str) -> Result<Webhook, DomainError>;
    /// List all webhooks
    async fn list(&self) -> Result<Vec<Webhook>, DomainError>;
    /// Get deliveries for a webhook
    async fn get_deliveries(
        &self,
        webhook_id: &str,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<WebhookDelivery>, DomainError>;
    /// Reset a webhook's failure count
    async fn reset_webhook(&self, id: &str) -> Result<Webhook, DomainError>;
    /// Clean up old deliveries
    async fn cleanup_deliveries(&self, retention_days: u32) -> Result<u64, DomainError>;
}

// Implement traits for the actual services

#[async_trait::async_trait]
impl<S: Storage<Model> + 'static> ModelServiceTrait for ModelService<S> {
    async fn get(&self, id: &str) -> Result<Option<Model>, DomainError> {
        ModelService::get(self, id).await
    }

    async fn list(&self) -> Result<Vec<Model>, DomainError> {
        ModelService::list(self).await
    }

    async fn create(&self, request: CreateModelRequest) -> Result<Model, DomainError> {
        ModelService::create(self, request).await
    }

    async fn update(&self, id: &str, request: UpdateModelRequest) -> Result<Model, DomainError> {
        ModelService::update(self, id, request).await
    }

    async fn delete(&self, id: &str) -> Result<bool, DomainError> {
        ModelService::delete(self, id).await
    }
}

#[async_trait::async_trait]
impl<S: Storage<Prompt> + 'static> PromptServiceTrait for PromptService<S> {
    async fn get(&self, id: &str) -> Result<Option<Prompt>, DomainError> {
        PromptService::get(self, id).await
    }

    async fn list(&self) -> Result<Vec<Prompt>, DomainError> {
        PromptService::list(self).await
    }

    async fn create(&self, request: CreatePromptRequest) -> Result<Prompt, DomainError> {
        PromptService::create(self, request).await
    }

    async fn update(&self, id: &str, request: UpdatePromptRequest) -> Result<Prompt, DomainError> {
        PromptService::update(self, id, request).await
    }

    async fn delete(&self, id: &str) -> Result<bool, DomainError> {
        PromptService::delete(self, id).await
    }

    async fn render(
        &self,
        id: &str,
        variables: &std::collections::HashMap<String, String>,
    ) -> Result<String, DomainError> {
        PromptService::render_by_id(self, id, variables.clone()).await
    }

    async fn revert(&self, id: &str, version: u32) -> Result<Prompt, DomainError> {
        PromptService::revert(self, id, version).await
    }
}

#[async_trait::async_trait]
impl<R: ApiKeyRepository + 'static> ApiKeyServiceTrait for ApiKeyService<R> {
    async fn validate(&self, key: &str) -> Result<Option<ApiKey>, DomainError> {
        ApiKeyService::validate(self, key).await
    }

    async fn get(&self, id: &str) -> Result<Option<ApiKey>, DomainError> {
        let key_id = crate::domain::api_key::ApiKeyId::new(id)
            .map_err(|e| DomainError::validation(e.to_string()))?;
        ApiKeyService::get(self, &key_id).await
    }

    async fn list(&self) -> Result<Vec<ApiKey>, DomainError> {
        ApiKeyService::list(self, None).await
    }

    async fn create(
        &self,
        name: &str,
        team_id: &str,
        permissions: ApiKeyPermissions,
    ) -> Result<(ApiKey, String), DomainError> {
        // Generate a new API key ID using UUID
        let uuid = uuid::Uuid::new_v4().to_string();
        let id = crate::domain::api_key::ApiKeyId::new(&uuid)
            .map_err(|e| DomainError::validation(e.to_string()))?;
        let team_id = crate::domain::team::TeamId::new(team_id)
            .map_err(|e| DomainError::validation(e.to_string()))?;
        let result = ApiKeyService::create(self, id, name, team_id, permissions, None).await?;
        Ok((result.api_key, result.secret))
    }

    async fn create_with_known_secret(
        &self,
        id: &str,
        name: &str,
        secret: &str,
        team_id: &str,
        permissions: ApiKeyPermissions,
    ) -> Result<ApiKey, DomainError> {
        let key_id = crate::domain::api_key::ApiKeyId::new(id)
            .map_err(|e| DomainError::validation(e.to_string()))?;
        let team_id = crate::domain::team::TeamId::new(team_id)
            .map_err(|e| DomainError::validation(e.to_string()))?;
        let result =
            ApiKeyService::create_with_secret(self, key_id, name, secret, team_id, permissions, None)
                .await?;
        Ok(result.api_key)
    }

    async fn update_permissions(
        &self,
        id: &str,
        permissions: ApiKeyPermissions,
    ) -> Result<(), DomainError> {
        let key_id = crate::domain::api_key::ApiKeyId::new(id)
            .map_err(|e| DomainError::validation(e.to_string()))?;
        ApiKeyService::update_permissions(self, &key_id, permissions).await?;
        Ok(())
    }

    async fn delete(&self, id: &str) -> Result<(), DomainError> {
        let key_id = crate::domain::api_key::ApiKeyId::new(id)
            .map_err(|e| DomainError::validation(e.to_string()))?;
        ApiKeyService::delete(self, &key_id).await?;
        Ok(())
    }

    async fn suspend(&self, id: &str) -> Result<(), DomainError> {
        let key_id = crate::domain::api_key::ApiKeyId::new(id)
            .map_err(|e| DomainError::validation(e.to_string()))?;
        ApiKeyService::suspend(self, &key_id).await?;
        Ok(())
    }

    async fn activate(&self, id: &str) -> Result<(), DomainError> {
        let key_id = crate::domain::api_key::ApiKeyId::new(id)
            .map_err(|e| DomainError::validation(e.to_string()))?;
        ApiKeyService::activate(self, &key_id).await?;
        Ok(())
    }

    async fn revoke(&self, id: &str) -> Result<(), DomainError> {
        let key_id = crate::domain::api_key::ApiKeyId::new(id)
            .map_err(|e| DomainError::validation(e.to_string()))?;
        ApiKeyService::revoke(self, &key_id).await?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl WorkflowServiceTrait for WorkflowService {
    async fn get(&self, id: &str) -> Result<Option<Workflow>, DomainError> {
        WorkflowService::get(self, id).await
    }

    async fn list(&self) -> Result<Vec<Workflow>, DomainError> {
        WorkflowService::list(self).await
    }

    async fn create(&self, request: CreateWorkflowRequest) -> Result<Workflow, DomainError> {
        WorkflowService::create(self, request).await
    }

    async fn update(&self, id: &str, request: UpdateWorkflowRequest) -> Result<Workflow, DomainError> {
        WorkflowService::update(self, id, request).await
    }

    async fn delete(&self, id: &str) -> Result<bool, DomainError> {
        WorkflowService::delete(self, id).await
    }

    async fn execute(&self, id: &str, input: Value) -> Result<WorkflowResult, DomainError> {
        WorkflowService::execute(self, id, input).await
    }
}

#[async_trait::async_trait]
impl<R: OperationRepository + 'static> OperationServiceTrait for OperationService<R> {
    async fn create_pending(
        &self,
        op_type: OperationType,
        input: Value,
        metadata: Value,
    ) -> Result<Operation, DomainError> {
        OperationService::create_pending(self, op_type, input, metadata).await
    }

    async fn get(&self, id: &str) -> Result<Option<Operation>, DomainError> {
        OperationService::get(self, id).await
    }

    async fn get_batch(&self, ids: &[String]) -> Result<Vec<Operation>, DomainError> {
        OperationService::get_batch(self, ids).await
    }

    async fn mark_running(&self, id: &str) -> Result<Operation, DomainError> {
        OperationService::mark_running(self, id).await
    }

    async fn mark_completed(&self, id: &str, result: Value) -> Result<Operation, DomainError> {
        OperationService::mark_completed(self, id, result).await
    }

    async fn mark_failed(&self, id: &str, error: String) -> Result<Operation, DomainError> {
        OperationService::mark_failed(self, id, error).await
    }

    async fn cancel(&self, id: &str) -> Result<Operation, DomainError> {
        OperationService::cancel(self, id).await
    }

    async fn cleanup_old(&self) -> Result<u64, DomainError> {
        OperationService::cleanup_old(self).await
    }
}

#[async_trait::async_trait]
impl<R: UserRepository + 'static, H: PasswordHasher + 'static> UserServiceTrait
    for UserService<R, H>
{
    async fn authenticate(
        &self,
        username: &str,
        password: &str,
    ) -> Result<Option<User>, DomainError> {
        UserService::authenticate(self, username, password).await
    }

    async fn get(&self, id: &str) -> Result<Option<User>, DomainError> {
        UserService::get(self, id).await
    }

    async fn get_by_username(&self, username: &str) -> Result<Option<User>, DomainError> {
        UserService::get_by_username(self, username).await
    }

    async fn create(&self, request: CreateUserRequest) -> Result<User, DomainError> {
        UserService::create(self, request).await
    }

    async fn list(&self, status: Option<UserStatus>) -> Result<Vec<User>, DomainError> {
        UserService::list(self, status).await
    }

    async fn count(&self, status: Option<UserStatus>) -> Result<usize, DomainError> {
        UserService::count(self, status).await
    }

    async fn update_password(
        &self,
        id: &str,
        request: UpdatePasswordRequest,
    ) -> Result<User, DomainError> {
        UserService::update_password(self, id, request).await
    }

    async fn suspend(&self, id: &str) -> Result<User, DomainError> {
        UserService::suspend(self, id).await
    }

    async fn activate(&self, id: &str) -> Result<User, DomainError> {
        UserService::activate(self, id).await
    }

    async fn delete(&self, id: &str) -> Result<bool, DomainError> {
        UserService::delete(self, id).await
    }
}

impl JwtServiceTrait for JwtService {
    fn generate(&self, user: &User) -> Result<String, DomainError> {
        JwtGenerator::generate(self, user)
    }

    fn validate(&self, token: &str) -> Result<JwtClaims, DomainError> {
        JwtGenerator::validate(self, token)
    }

    fn expiration_hours(&self) -> u64 {
        JwtGenerator::expiration_hours(self)
    }
}

impl JwtServiceTrait for JwksJwtService {
    fn generate(&self, user: &User) -> Result<String, DomainError> {
        JwtGenerator::generate(self, user)
    }

    fn validate(&self, token: &str) -> Result<JwtClaims, DomainError> {
        JwtGenerator::validate(self, token)
    }

    fn expiration_hours(&self) -> u64 {
        JwtGenerator::expiration_hours(self)
    }
}

#[async_trait::async_trait]
impl<R: TeamRepository + 'static> TeamServiceTrait for TeamService<R> {
    async fn get(&self, id: &str) -> Result<Option<Team>, DomainError> {
        TeamService::get(self, id).await
    }

    async fn list(&self, query: Option<TeamQuery>) -> Result<Vec<Team>, DomainError> {
        TeamService::list(self, query).await
    }

    async fn count(&self, query: Option<TeamQuery>) -> Result<usize, DomainError> {
        TeamService::count(self, query).await
    }

    async fn create(&self, request: CreateTeamRequest) -> Result<Team, DomainError> {
        TeamService::create(self, request).await
    }

    async fn update(&self, id: &str, request: UpdateTeamRequest) -> Result<Team, DomainError> {
        TeamService::update(self, id, request).await
    }

    async fn suspend(&self, id: &str) -> Result<Team, DomainError> {
        TeamService::suspend(self, id).await
    }

    async fn activate(&self, id: &str) -> Result<Team, DomainError> {
        TeamService::activate(self, id).await
    }

    async fn delete(&self, id: &str) -> Result<bool, DomainError> {
        TeamService::delete(self, id).await
    }

    async fn exists(&self, id: &str) -> Result<bool, DomainError> {
        TeamService::exists(self, id).await
    }
}

#[async_trait::async_trait]
impl<R: StoredCredentialRepository + 'static> CredentialServiceTrait for CredentialService<R> {
    async fn get(&self, id: &str) -> Result<Option<StoredCredential>, DomainError> {
        CredentialService::get(self, id).await
    }

    async fn list(&self) -> Result<Vec<StoredCredential>, DomainError> {
        CredentialService::list(self).await
    }

    async fn create(
        &self,
        request: CreateCredentialRequest,
    ) -> Result<StoredCredential, DomainError> {
        CredentialService::create(self, request).await
    }

    async fn update(
        &self,
        id: &str,
        request: UpdateCredentialRequest,
    ) -> Result<StoredCredential, DomainError> {
        CredentialService::update(self, id, request).await
    }

    async fn delete(&self, id: &str) -> Result<(), DomainError> {
        CredentialService::delete(self, id).await
    }

    async fn exists(&self, id: &str) -> Result<bool, DomainError> {
        CredentialService::exists(self, id).await
    }
}

#[async_trait::async_trait]
impl<S: Storage<ExternalApi> + 'static> ExternalApiServiceTrait for ExternalApiService<S> {
    async fn get(&self, id: &str) -> Result<Option<ExternalApi>, DomainError> {
        ExternalApiService::get(self, id).await
    }

    async fn list(&self) -> Result<Vec<ExternalApi>, DomainError> {
        ExternalApiService::list(self).await
    }

    async fn create(&self, request: CreateExternalApiRequest) -> Result<ExternalApi, DomainError> {
        ExternalApiService::create(self, request).await
    }

    async fn update(
        &self,
        id: &str,
        request: UpdateExternalApiRequest,
    ) -> Result<ExternalApi, DomainError> {
        ExternalApiService::update(self, id, request).await
    }

    async fn delete(&self, id: &str) -> Result<(), DomainError> {
        ExternalApiService::delete(self, id).await
    }

    async fn exists(&self, id: &str) -> Result<bool, DomainError> {
        ExternalApiService::exists(self, id).await
    }
}

#[async_trait::async_trait]
impl KnowledgeBaseServiceTrait for KnowledgeBaseService {
    async fn get(&self, id: &str) -> Result<Option<KnowledgeBase>, DomainError> {
        KnowledgeBaseService::get(self, id).await
    }

    async fn list(&self) -> Result<Vec<KnowledgeBase>, DomainError> {
        KnowledgeBaseService::list(self).await
    }

    async fn create(
        &self,
        request: CreateKnowledgeBaseRequest,
    ) -> Result<KnowledgeBase, DomainError> {
        KnowledgeBaseService::create(self, request).await
    }

    async fn update(
        &self,
        id: &str,
        request: UpdateKnowledgeBaseRequest,
    ) -> Result<KnowledgeBase, DomainError> {
        KnowledgeBaseService::update(self, id, request).await
    }

    async fn delete(&self, id: &str) -> Result<bool, DomainError> {
        KnowledgeBaseService::delete(self, id).await
    }

    async fn exists(&self, id: &str) -> Result<bool, DomainError> {
        KnowledgeBaseService::exists(self, id).await
    }
}

#[async_trait::async_trait]
impl IngestionServiceTrait for IngestionService {
    async fn ingest(
        &self,
        kb_id: &str,
        request: IngestDocumentRequest,
    ) -> Result<crate::domain::ingestion::IngestionResult, DomainError> {
        IngestionService::ingest(self, kb_id, request).await
    }

    async fn list_sources(
        &self,
        kb_id: &str,
    ) -> Result<Vec<crate::domain::knowledge_base::SourceInfo>, DomainError> {
        IngestionService::list_sources(self, kb_id).await
    }

    async fn get_documents_by_source(
        &self,
        kb_id: &str,
        source: &str,
    ) -> Result<Vec<StoredDocument>, DomainError> {
        IngestionService::get_documents_by_source(self, kb_id, source).await
    }

    async fn document_count(&self, kb_id: &str) -> Result<usize, DomainError> {
        IngestionService::document_count(self, kb_id).await
    }

    async fn delete_by_source(&self, kb_id: &str, source: &str) -> Result<usize, DomainError> {
        IngestionService::delete_by_source(self, kb_id, source).await
    }

    async fn ensure_schema(&self, kb_id: &str) -> Result<(), DomainError> {
        IngestionService::ensure_schema(self, kb_id).await
    }

    async fn ingest_document(
        &self,
        kb_id: &str,
        request: IngestDocumentV2Request,
    ) -> Result<KnowledgeBaseDocument, DomainError> {
        IngestionService::ingest_document(self, kb_id, request).await
    }

    async fn list_documents_v2(&self, kb_id: &str) -> Result<Vec<DocumentSummary>, DomainError> {
        IngestionService::list_documents_v2(self, kb_id).await
    }

    async fn get_document_v2(
        &self,
        kb_id: &str,
        document_id: uuid::Uuid,
    ) -> Result<Option<KnowledgeBaseDocument>, DomainError> {
        IngestionService::get_document_v2(self, kb_id, document_id).await
    }

    async fn get_document_chunks(
        &self,
        kb_id: &str,
        document_id: uuid::Uuid,
    ) -> Result<Vec<DocumentChunk>, DomainError> {
        IngestionService::get_document_chunks(self, kb_id, document_id).await
    }

    async fn delete_document_v2(
        &self,
        kb_id: &str,
        document_id: uuid::Uuid,
    ) -> Result<bool, DomainError> {
        IngestionService::delete_document_v2(self, kb_id, document_id).await
    }

    async fn disable_document(
        &self,
        kb_id: &str,
        document_id: uuid::Uuid,
    ) -> Result<bool, DomainError> {
        IngestionService::disable_document(self, kb_id, document_id).await
    }

    async fn enable_document(
        &self,
        kb_id: &str,
        document_id: uuid::Uuid,
    ) -> Result<bool, DomainError> {
        IngestionService::enable_document(self, kb_id, document_id).await
    }
}

#[async_trait::async_trait]
impl<R: UsageRepository + 'static> UsageServiceTrait for UsageTrackingService<R> {
    async fn record(&self, params: RecordUsageParams) -> Result<UsageRecord, DomainError> {
        UsageTrackingServiceTrait::record(self, params).await
    }

    async fn get(&self, id: &UsageRecordId) -> Result<Option<UsageRecord>, DomainError> {
        UsageTrackingServiceTrait::get(self, id).await
    }

    async fn query(&self, query: &UsageQuery) -> Result<Vec<UsageRecord>, DomainError> {
        UsageTrackingServiceTrait::query(self, query).await
    }

    async fn count(&self, query: &UsageQuery) -> Result<usize, DomainError> {
        UsageTrackingServiceTrait::count(self, query).await
    }

    async fn aggregate(&self, query: &UsageQuery) -> Result<UsageAggregate, DomainError> {
        UsageTrackingServiceTrait::aggregate(self, query).await
    }

    async fn summary(&self, query: &UsageQuery) -> Result<UsageSummary, DomainError> {
        UsageTrackingServiceTrait::summary(self, query).await
    }

    async fn delete_before(&self, timestamp: u64) -> Result<usize, DomainError> {
        UsageTrackingServiceTrait::delete_before(self, timestamp).await
    }

    async fn delete_by_api_key(&self, api_key_id: &str) -> Result<usize, DomainError> {
        UsageTrackingServiceTrait::delete_by_api_key(self, api_key_id).await
    }

    fn get_pricing(&self, model_id: &str) -> Option<ModelPricing> {
        UsageTrackingServiceTrait::get_pricing(self, model_id).cloned()
    }

    fn calculate_cost(&self, model_id: &str, input_tokens: u32, output_tokens: u32) -> i64 {
        UsageTrackingServiceTrait::calculate_cost(self, model_id, input_tokens, output_tokens)
    }
}

#[async_trait::async_trait]
impl<R: BudgetRepository + 'static> BudgetServiceStateTrait for BudgetService<R> {
    async fn create(&self, budget: Budget) -> Result<Budget, DomainError> {
        BudgetServiceTrait::create(self, budget).await
    }

    async fn get(&self, id: &BudgetId) -> Result<Option<Budget>, DomainError> {
        BudgetServiceTrait::get(self, id).await
    }

    async fn update(&self, budget: Budget) -> Result<Budget, DomainError> {
        BudgetServiceTrait::update(self, budget).await
    }

    async fn delete(&self, id: &BudgetId) -> Result<bool, DomainError> {
        BudgetServiceTrait::delete(self, id).await
    }

    async fn list(&self) -> Result<Vec<Budget>, DomainError> {
        BudgetServiceTrait::list(self).await
    }

    async fn list_by_team(&self, team_id: &str) -> Result<Vec<Budget>, DomainError> {
        BudgetServiceTrait::list_by_team(self, team_id).await
    }

    async fn check_budget(
        &self,
        api_key_id: &str,
        model_id: Option<&str>,
        estimated_cost_micros: i64,
    ) -> Result<BudgetCheckResult, DomainError> {
        BudgetServiceTrait::check_budget(self, api_key_id, model_id, estimated_cost_micros).await
    }

    async fn check_budget_with_team(
        &self,
        api_key_id: &str,
        team_id: Option<&str>,
        model_id: Option<&str>,
        estimated_cost_micros: i64,
    ) -> Result<BudgetCheckResult, DomainError> {
        BudgetServiceTrait::check_budget_with_team(
            self,
            api_key_id,
            team_id,
            model_id,
            estimated_cost_micros,
        )
        .await
    }
}

#[async_trait::async_trait]
impl<R: ExperimentRepository + 'static, RR: ExperimentRecordRepository + 'static>
    ExperimentServiceTrait for ExperimentService<R, RR>
{
    async fn get(&self, id: &str) -> Result<Option<Experiment>, DomainError> {
        ExperimentService::get(self, id).await
    }

    async fn list(&self, query: Option<ExperimentQuery>) -> Result<Vec<Experiment>, DomainError> {
        let default_query = ExperimentQuery::new();
        let query_ref = query.as_ref().unwrap_or(&default_query);
        ExperimentService::list(self, query_ref).await
    }

    async fn create(&self, request: CreateExperimentRequest) -> Result<Experiment, DomainError> {
        ExperimentService::create(self, request).await
    }

    async fn update(
        &self,
        id: &str,
        request: UpdateExperimentRequest,
    ) -> Result<Experiment, DomainError> {
        ExperimentService::update(self, id, request).await
    }

    async fn delete(&self, id: &str) -> Result<bool, DomainError> {
        ExperimentService::delete(self, id).await
    }

    async fn add_variant(
        &self,
        experiment_id: &str,
        request: CreateVariantRequest,
    ) -> Result<Experiment, DomainError> {
        ExperimentService::add_variant(self, experiment_id, request).await
    }

    async fn remove_variant(
        &self,
        experiment_id: &str,
        variant_id: &str,
    ) -> Result<Experiment, DomainError> {
        ExperimentService::remove_variant(self, experiment_id, variant_id).await
    }

    async fn start(&self, id: &str) -> Result<Experiment, DomainError> {
        ExperimentService::start(self, id).await
    }

    async fn pause(&self, id: &str) -> Result<Experiment, DomainError> {
        ExperimentService::pause(self, id).await
    }

    async fn resume(&self, id: &str) -> Result<Experiment, DomainError> {
        ExperimentService::resume(self, id).await
    }

    async fn complete(&self, id: &str) -> Result<Experiment, DomainError> {
        ExperimentService::complete(self, id).await
    }

    async fn assign_variant(
        &self,
        model_id: &str,
        api_key_id: &str,
    ) -> Result<Option<AssignmentResult>, DomainError> {
        ExperimentService::assign_variant(self, model_id, api_key_id).await
    }

    async fn record(&self, params: RecordExperimentParams) -> Result<(), DomainError> {
        ExperimentService::record(self, params).await
    }

    async fn get_results(&self, id: &str) -> Result<ExperimentResult, DomainError> {
        ExperimentService::get_results(self, id).await
    }

    async fn find_by_status(
        &self,
        status: ExperimentStatus,
    ) -> Result<Vec<Experiment>, DomainError> {
        ExperimentService::find_by_status(self, status).await
    }
}

#[async_trait::async_trait]
impl<R: TestCaseRepository + 'static, RR: TestCaseResultRepository + 'static> TestCaseServiceTrait
    for TestCaseService<R, RR>
{
    async fn get(&self, id: &str) -> Result<Option<TestCase>, DomainError> {
        TestCaseService::get(self, id).await
    }

    async fn list(&self, query: &TestCaseQuery) -> Result<Vec<TestCase>, DomainError> {
        TestCaseService::list(self, query).await
    }

    async fn count(&self, query: &TestCaseQuery) -> Result<usize, DomainError> {
        TestCaseService::count(self, query).await
    }

    async fn create(&self, request: CreateTestCaseRequest) -> Result<TestCase, DomainError> {
        TestCaseService::create(self, request).await
    }

    async fn update(
        &self,
        id: &str,
        request: UpdateTestCaseRequest,
    ) -> Result<TestCase, DomainError> {
        TestCaseService::update(self, id, request).await
    }

    async fn delete(&self, id: &str) -> Result<bool, DomainError> {
        TestCaseService::delete(self, id).await
    }

    async fn execute(&self, id: &str) -> Result<ExecuteTestCaseResponse, DomainError> {
        TestCaseService::execute(self, id).await
    }

    async fn get_results(
        &self,
        id: &str,
        query: &TestCaseResultQuery,
    ) -> Result<Vec<TestCaseResult>, DomainError> {
        TestCaseService::get_results(self, id, query).await
    }

    async fn get_latest_result(&self, id: &str) -> Result<Option<TestCaseResult>, DomainError> {
        TestCaseService::get_latest_result(self, id).await
    }
}

#[async_trait::async_trait]
impl ConfigServiceTrait for ConfigService {
    async fn list(&self) -> Result<Vec<ConfigEntry>, DomainError> {
        ConfigService::list(self).await
    }

    async fn list_by_category(
        &self,
        category: ConfigCategory,
    ) -> Result<Vec<ConfigEntry>, DomainError> {
        ConfigService::list_by_category(self, category).await
    }

    async fn get_entry(&self, key: &str) -> Result<Option<ConfigEntry>, DomainError> {
        ConfigService::get_entry(self, key).await
    }

    async fn get_value(&self, key: &str) -> Result<Option<ConfigValue>, DomainError> {
        ConfigService::get_value(self, key).await
    }

    async fn set(&self, key: &str, value: ConfigValue) -> Result<(), DomainError> {
        ConfigService::set(self, key, value).await
    }

    async fn reset(&self) -> Result<(), DomainError> {
        ConfigService::reset(self).await
    }
}

#[async_trait::async_trait]
impl ExecutionLogServiceTrait for ExecutionLogService {
    async fn record(
        &self,
        params: RecordExecutionParams,
    ) -> Result<Option<ExecutionLog>, DomainError> {
        ExecutionLogService::record(self, params).await
    }

    async fn get(&self, id: &str) -> Result<Option<ExecutionLog>, DomainError> {
        ExecutionLogService::get(self, id).await
    }

    async fn list(&self, query: &ExecutionLogQuery) -> Result<Vec<ExecutionLog>, DomainError> {
        ExecutionLogService::list(self, query).await
    }

    async fn count(&self, query: &ExecutionLogQuery) -> Result<usize, DomainError> {
        ExecutionLogService::count(self, query).await
    }

    async fn delete(&self, id: &str) -> Result<bool, DomainError> {
        ExecutionLogService::delete(self, id).await
    }

    async fn cleanup_old_logs(&self) -> Result<usize, DomainError> {
        ExecutionLogService::cleanup_old_logs(self).await
    }

    async fn delete_older_than(&self, days: i64) -> Result<usize, DomainError> {
        ExecutionLogService::delete_older_than(self, days).await
    }

    async fn stats(&self, query: &ExecutionLogQuery) -> Result<ExecutionStats, DomainError> {
        ExecutionLogService::stats(self, query).await
    }

    async fn update(&self, log: &ExecutionLog) -> Result<(), DomainError> {
        ExecutionLogService::update(self, log).await
    }

    async fn record_pending_ingestion(
        &self,
        kb_id: &str,
        source_name: &str,
        executor: Executor,
        input: serde_json::Value,
    ) -> Result<ExecutionLog, DomainError> {
        ExecutionLogService::record_pending_ingestion(self, kb_id, source_name, executor, input)
            .await
    }
}

#[async_trait::async_trait]
impl<W: WebhookRepository + 'static, D: WebhookDeliveryRepository + 'static>
    WebhookServiceStateTrait for WebhookService<W, D>
{
    async fn create(&self, webhook: Webhook) -> Result<Webhook, DomainError> {
        WebhookServiceTrait::create(self, webhook).await
    }

    async fn update(&self, id: &str, webhook: Webhook) -> Result<Webhook, DomainError> {
        WebhookServiceTrait::update(self, id, webhook).await
    }

    async fn delete(&self, id: &str) -> Result<(), DomainError> {
        WebhookServiceTrait::delete(self, id).await
    }

    async fn get(&self, id: &str) -> Result<Webhook, DomainError> {
        WebhookServiceTrait::get(self, id).await
    }

    async fn list(&self) -> Result<Vec<Webhook>, DomainError> {
        WebhookServiceTrait::list(self).await
    }

    async fn get_deliveries(
        &self,
        webhook_id: &str,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<WebhookDelivery>, DomainError> {
        WebhookServiceTrait::get_deliveries(self, webhook_id, limit, offset).await
    }

    async fn reset_webhook(&self, id: &str) -> Result<Webhook, DomainError> {
        WebhookServiceTrait::reset_webhook(self, id).await
    }

    async fn cleanup_deliveries(&self, retention_days: u32) -> Result<u64, DomainError> {
        WebhookServiceTrait::cleanup_deliveries(self, retention_days).await
    }
}

impl AppState {
    /// Create new application state with provided services
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        model_service: Arc<dyn ModelServiceTrait>,
        prompt_service: Arc<dyn PromptServiceTrait>,
        api_key_service: Arc<dyn ApiKeyServiceTrait>,
        workflow_service: Arc<dyn WorkflowServiceTrait>,
        operation_service: Arc<dyn OperationServiceTrait>,
        user_service: Arc<dyn UserServiceTrait>,
        team_service: Arc<dyn TeamServiceTrait>,
        jwt_service: Arc<dyn JwtServiceTrait>,
        credential_service: Arc<dyn CredentialServiceTrait>,
        external_api_service: Arc<dyn ExternalApiServiceTrait>,
        knowledge_base_service: Arc<dyn KnowledgeBaseServiceTrait>,
        ingestion_service: Arc<dyn IngestionServiceTrait>,
        usage_service: Arc<dyn UsageServiceTrait>,
        budget_service: Arc<dyn BudgetServiceStateTrait>,
        experiment_service: Arc<dyn ExperimentServiceTrait>,
        test_case_service: Arc<dyn TestCaseServiceTrait>,
        config_service: Arc<dyn ConfigServiceTrait>,
        execution_log_service: Arc<dyn ExecutionLogServiceTrait>,
        webhook_service: Arc<dyn WebhookServiceStateTrait>,
        llm_provider: Arc<dyn LlmProvider>,
        provider_router: Arc<ProviderRouter>,
    ) -> Self {
        Self {
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
            webhook_service,
            execution_log_service,
            llm_provider,
            provider_router,
        }
    }

    /// Get the webhook service
    pub fn webhook_service(&self) -> &Arc<dyn WebhookServiceStateTrait> {
        &self.webhook_service
    }
}
