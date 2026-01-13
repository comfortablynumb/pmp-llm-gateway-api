//! Domain layer - Core business logic and entities

pub mod api_key;
pub mod cache;
pub mod chain;
pub mod config;
pub mod crag;
pub mod credentials;
pub mod embedding;
pub mod error;
pub mod experiment;
pub mod external_api;
pub mod ingestion;
pub mod knowledge_base;
pub mod llm;
pub mod model;
pub mod operation;
pub mod plugin;
pub mod prompt;
pub mod semantic_cache;
pub mod storage;
pub mod team;
pub mod test_case;
pub mod traits;
pub mod usage;
pub mod user;
pub mod webhook;
pub mod workflow;

pub use chain::{
    ChainExecutor, ChainExecutorConfig, ChainId, ChainRepository, ChainResult, ChainStep,
    FallbackBehavior, ModelChain, RetryConfig, StepResult,
};
pub use config::{
    AppConfiguration, ConfigCategory, ConfigEntry, ConfigKey, ConfigMetadata, ConfigRepository,
    ConfigValidationError, ConfigValue, ExecutionLog, ExecutionLogId, ExecutionLogQuery,
    ExecutionLogRepository, ExecutionLogValidationError, ExecutionStats, ExecutionStatus,
    ExecutionType, Executor, TokenUsage as ExecutionTokenUsage, WorkflowStepLog,
};
pub use credentials::{
    Credential, CredentialId, CredentialProvider, CredentialType, StoredCredential,
    StoredCredentialRepository,
};
pub use error::DomainError;
pub use llm::{
    ContentPart, FinishReason, LlmJsonSchema, LlmProvider, LlmRequest, LlmRequestBuilder,
    LlmResponse, LlmResponseFormat, LlmStream, Message, MessageRole, ProviderResolver,
    StaticProviderResolver, StreamChunk, Usage,
};
pub use model::{
    validate_model_config, validate_model_id, Model, ModelConfig, ModelId, ModelValidationError,
};
pub use operation::{
    validate_operation_id, Operation, OperationError, OperationId, OperationRepository,
    OperationStatus, OperationType,
};
pub use prompt::{Prompt, PromptId, PromptTemplate, PromptVariable, PromptVersion, TemplateError};
pub use cache::{Cache, CacheExt, CacheKey, CacheKeyGenerator, CacheKeyParams, DefaultKeyGenerator};
pub use knowledge_base::{
    AddDocumentsResult, DeleteDocumentsResult, Document, EmbeddingConfig, FilterBuilder,
    FilterCondition, FilterConnector, FilterOperator, FilterValue, KnowledgeBase,
    KnowledgeBaseConfig, KnowledgeBaseId, KnowledgeBaseProvider, KnowledgeBaseType,
    KnowledgeBaseValidationError, MetadataFilter, SearchParams, SearchQuery, SearchResult,
};
pub use storage::{Storage, StorageEntity, StorageKey};
pub use ingestion::{
    BatchIngestionResult, Chunk, ChunkingConfig, ChunkingStrategy, ChunkingType, ChunkMetadata,
    DocumentMetadata, DocumentParser, IngestionConfig, IngestionError, IngestionResult,
    ParsedDocument, ParserContent, ParserInput, ParserType,
};
pub use crag::{
    CragConfig, CragFilter, CragResult, CragSummary, DocumentScorer, RelevanceClassification,
    ScoredDocument, ScoringInput, ScoringStrategy,
};
pub use api_key::{
    ApiKey, ApiKeyId, ApiKeyPermissions, ApiKeyRepository, ApiKeyStatus, ApiKeyValidationError,
    RateLimitConfig, ResourcePermission,
};
pub use workflow::{
    ChatCompletionStep, Condition, ConditionalAction, ConditionalStep, ConditionOperator,
    CragScoringStep, HttpMethod, HttpRequestStep, KnowledgeBaseSearchStep, OnErrorAction,
    StepExecutionResult, VariableRef, Workflow, WorkflowContext, WorkflowError, WorkflowExecutor,
    WorkflowId, WorkflowRepository, WorkflowResult, WorkflowStep, WorkflowStepType,
    WorkflowTokenUsage,
};
pub use user::{
    validate_password, validate_user_id, validate_username, User, UserId, UserRepository,
    UserStatus, UserValidationError,
};
pub use embedding::{
    cosine_similarity, Embedding, EmbeddingInput, EmbeddingProvider, EmbeddingRequest,
    EmbeddingResponse, EmbeddingUsage,
};
pub use external_api::{ExternalApi, ExternalApiId};
pub use semantic_cache::{
    CachedEntry, SemanticCache, SemanticCacheConfig, SemanticCacheStats, SemanticSearchParams,
    SemanticSearchResult,
};
pub use usage::{
    Budget, BudgetAlert, BudgetId, BudgetPeriod, BudgetRepository, BudgetStatus,
    BudgetValidationError, ModelPricing, PricingTier, UsageAggregate, UsageQuery, UsageRecord,
    UsageRecordId, UsageRepository, UsageSummary, UsageType,
};
pub use experiment::{
    AssignmentResult, ConfigOverrides, Experiment, ExperimentId, ExperimentQuery,
    ExperimentRecord, ExperimentRecordId, ExperimentRecordQuery, ExperimentRecordRepository,
    ExperimentRepository, ExperimentResult, ExperimentStatus, ExperimentValidationError,
    LatencyStats, StatisticalSignificance, TrafficAllocation, Variant, VariantConfig, VariantId,
    VariantMetrics,
};
pub use plugin::{
    CredentialProviderConfig, CredentialProviderPlugin, CredentialSourceType,
    EmbeddingProviderPlugin, ExtensionType, KnowledgeBaseProviderConfig,
    KnowledgeBaseProviderPlugin, LlmProviderConfig, LlmProviderPlugin, Plugin, PluginContext,
    PluginError, PluginMetadata, PluginState,
};
pub use test_case::{
    AssertionCriteria, AssertionEvaluator, AssertionOperator, AssertionResult, ModelPromptInput,
    TestCase, TestCaseId, TestCaseInput, TestCaseQuery, TestCaseRepository, TestCaseResult,
    TestCaseResultId, TestCaseResultQuery, TestCaseResultRepository, TestCaseType,
    TestCaseValidationError, TokenUsage, WorkflowInput as TestCaseWorkflowInput,
};
pub use team::{
    validate_team_id, validate_team_name, Team, TeamId, TeamQuery, TeamRepository, TeamRole,
    TeamStatus, TeamValidationError,
};
pub use webhook::{
    DeliveryStatus, Webhook, WebhookDelivery, WebhookDeliveryId, WebhookDeliveryRepository,
    WebhookEvent, WebhookEventType, WebhookId, WebhookRepository, WebhookStatus,
};
