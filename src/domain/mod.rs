//! Domain layer - Core business logic and entities

pub mod api_key;
pub mod cache;
pub mod chain;
pub mod crag;
pub mod credentials;
pub mod error;
pub mod ingestion;
pub mod knowledge_base;
pub mod llm;
pub mod model;
pub mod operation;
pub mod prompt;
pub mod storage;
pub mod traits;
pub mod workflow;

pub use chain::{
    ChainExecutor, ChainExecutorConfig, ChainId, ChainRepository, ChainResult, ChainStep,
    FallbackBehavior, ModelChain, RetryConfig, StepResult,
};
pub use credentials::{Credential, CredentialProvider, CredentialType};
pub use error::DomainError;
pub use llm::{
    ContentPart, FinishReason, LlmProvider, LlmRequest, LlmRequestBuilder, LlmResponse, LlmStream,
    Message, MessageRole, StreamChunk, Usage,
};
pub use model::{
    validate_model_config, validate_model_id, InMemoryModelRepository, Model, ModelConfig, ModelId,
    ModelRepository, ModelValidationError,
};
pub use operation::{
    validate_operation_id, Operation, OperationError, OperationId, OperationRepository,
    OperationStatus, OperationType,
};
pub use prompt::{
    InMemoryPromptRepository, Prompt, PromptId, PromptRepository, PromptTemplate, PromptVariable,
    PromptVersion, TemplateError,
};
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
    CragScoringStep, KnowledgeBaseSearchStep, OnErrorAction, StepExecutionResult, VariableRef,
    Workflow, WorkflowContext, WorkflowError, WorkflowExecutor, WorkflowId, WorkflowRepository,
    WorkflowResult, WorkflowStep, WorkflowStepType,
};
