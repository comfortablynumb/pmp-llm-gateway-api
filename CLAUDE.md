# PMP LLM Gateway API

## Project Type
Rust web API using Axum framework.

## Structure
```
src/
├── main.rs              # Entry point (CLI dispatch)
├── lib.rs               # Library exports + create_app_state()
├── cli/                 # CLI commands
│   ├── mod.rs           # Cli struct, Command enum
│   ├── serve/           # API + UI combined
│   ├── api/             # API only
│   └── ui/              # UI only (+ proxy)
├── api/                 # HTTP layer
│   ├── health.rs        # Health check endpoints
│   ├── middleware/      # Auth middleware (RequireApiKey)
│   ├── router.rs        # Axum router setup
│   ├── state.rs         # AppState with service traits
│   ├── types/           # OpenAI-compatible types (chat, error, models)
│   ├── v1/              # API v1 endpoints (chat, models)
│   └── admin/           # Admin API (models, prompts, api-keys, credentials)
├── config/              # Configuration (AppConfig)
├── domain/              # Business logic
│   ├── error.rs         # DomainError enum
│   ├── api_key/         # API key management (ApiKey, ApiKeyPermissions, RateLimitConfig)
│   ├── cache/           # Cache abstraction (Cache trait, CacheKey)
│   ├── credentials/     # Credential, CredentialType, CredentialProvider trait
│   ├── crag/            # CRAG domain (CragConfig, DocumentScorer, ScoredDocument)
│   ├── ingestion/       # Document ingestion (DocumentParser, ChunkingStrategy traits)
│   ├── knowledge_base/  # Knowledge base domain (KnowledgeBase, SearchResult, FilterBuilder)
│   ├── llm/             # LLM domain models (LlmRequest with prompt refs)
│   ├── model/           # Model configuration (Model, ModelId, ModelConfig)
│   ├── chain/           # Model chains (ChainExecutor, circuit breaker)
│   ├── prompt/          # Prompt management (templating, versioning)
│   ├── storage/         # Storage abstraction (Storage trait, StorageEntity)
│   ├── workflow/        # Multi-step workflows (WorkflowExecutor, variable resolution)
│   └── operation/       # Async operations (Operation, OperationStatus, OperationRepository)
└── infrastructure/      # External implementations
    ├── logging.rs       # Tracing setup
    ├── api_key/         # ApiKeyGenerator, RateLimiter, InMemoryApiKeyRepository, ApiKeyService
    ├── cache/           # InMemoryCache, RedisCache, CacheFactory
    ├── crag/            # ThresholdDocumentScorer, LlmDocumentScorer, CragPipeline
    ├── credentials/     # ENV, AWS Secrets, Vault providers
    ├── ingestion/       # Parsers, Chunkers, IngestionPipeline, factories
    ├── knowledge_base/  # PgvectorKnowledgeBase, AwsKnowledgeBase, factory
    ├── llm/             # LLM providers (OpenAI, Anthropic, Azure, Bedrock)
    ├── services/        # ModelService, PromptService, WorkflowService, OperationService, LlmCacheService
    ├── storage/         # InMemoryStorage, PostgresStorage, migrations
    ├── workflow/        # WorkflowExecutorImpl, InMemoryWorkflowRepository
    └── operation/       # InMemoryOperationRepository
public/                      # Admin UI static files (jQuery + Tailwind CSS)
├── index.html, css/, js/views/
```

## Commands
```bash
cargo run serve               # API + UI combined (default)
cargo run api                 # API server only
cargo run ui                  # UI + proxy to http://localhost:3001
cargo run ui --api-url URL    # UI + proxy to custom API URL
cargo run ui --skip-proxy     # UI only (static files)
cargo test                    # Run tests (645 tests)
cargo build --release         # Release build
bin/up.bat full              # Start all Docker services
bin/down.bat                 # Stop Docker services
bin/test-integration.bat     # Run integration tests
```

## Key Features Implemented
- **LLM Providers**: OpenAI, Anthropic, Azure OpenAI, AWS Bedrock
- **Credentials**: ENV, AWS Secrets Manager, Vault with caching
- **Models**: ID validation, config versioning, CRUD service
- **Chains**: Fallback, retry with exponential backoff, circuit breaker, metrics
- **Prompts**: CRUD, versioning, variable templating `${var:name:default}`, rendering
- **Storage**: Generic Storage trait, InMemoryStorage, PostgresStorage with pooling, migrations
- **Cache**: Generic Cache trait, InMemoryCache (moka), RedisCache, LlmCacheService
- **Knowledge Bases**: Pgvector, AWS Bedrock KB, metadata filtering with FilterBuilder
- **Document Ingestion**: Parsers (TXT, Markdown, HTML, JSON), Chunkers (FixedSize, Sentence, Paragraph, Recursive), IngestionPipeline
- **CRAG**: DocumentScorer trait, LLM/Threshold/Hybrid scoring strategies, CragPipeline with knowledge base integration
- **API Keys**: Secure key generation (SHA256), ResourcePermission (All/Specific/None), sliding window RateLimiter, ApiKeyService
- **OpenAI API**: Chat completions, models endpoints, SSE streaming, prompt references, API key auth middleware
- **Admin API**: Models CRUD, Prompts CRUD, API Keys management (CRUD + suspend/activate/revoke), Workflows CRUD, Credential provider info
- **Workflows**: Multi-step workflows with ChatCompletion, KnowledgeBaseSearch, CragScoring, Conditional steps; variable references `${request:field}`, `${step:name:field}`
- **Async Operations**: Run chat completions and workflows async with `?async=true`; query/cancel operations via `/v1/operations/{id}`
- **Admin UI**: Embedded jQuery + Tailwind CSS SPA at `/ui/`; manages Models, Prompts, API Keys, Workflows; CLI subcommands (serve, api, ui)

## Current Status
Milestone 18 complete. 645 unit tests + 17 hurl integration test files (123 requests). Next: Milestone 19 (Observability).

## Integration Tests
Run with `bin/test-integration.bat` (Windows) or `docker compose --profile test up`.
Tests use pmp-mock-http (ironedge/pmp-mock-http:v0.4.0) to mock LLM provider APIs.
