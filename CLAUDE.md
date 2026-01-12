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
│   ├── middleware/      # Auth middleware (RequireApiKey, RequireUser, RequireAdmin)
│   ├── auth/            # Authentication endpoints (login, logout, me)
│   ├── router.rs        # Axum router setup
│   ├── state.rs         # AppState with service traits
│   ├── types/           # OpenAI-compatible types (chat, error, models)
│   ├── v1/              # API v1 endpoints (chat, models)
│   └── admin/           # Admin API (models, prompts, api-keys, credentials)
├── config/              # Configuration (AppConfig)
├── domain/              # Business logic
│   ├── error.rs         # DomainError enum
│   ├── team/            # Team entity (Team, TeamId, TeamStatus, TeamRole, TeamRepository trait)
│   ├── user/            # User entity (User, UserId, UserStatus, team_id, team_role, UserRepository trait)
│   ├── api_key/         # API key management (ApiKey, team_id, ApiKeyPermissions, RateLimitConfig)
│   ├── cache/           # Cache abstraction (Cache trait, CacheKey)
│   ├── credentials/     # Credential, CredentialType, CredentialProvider trait
│   ├── external_api/    # ExternalApi entity (base_url, base_headers for HTTP Request steps)
│   ├── crag/            # CRAG domain (CragConfig, DocumentScorer, ScoredDocument)
│   ├── embedding/       # Embedding domain (EmbeddingProvider, EmbeddingRequest/Response)
│   ├── ingestion/       # Document ingestion (DocumentParser, ChunkingStrategy traits)
│   ├── knowledge_base/  # Knowledge base domain (KnowledgeBase, SearchResult, FilterBuilder)
│   ├── llm/             # LLM domain models (LlmRequest, ProviderResolver trait)
│   ├── model/           # Model configuration (Model, ModelId, ModelConfig)
│   ├── chain/           # Model chains (ChainExecutor, circuit breaker)
│   ├── prompt/          # Prompt management (templating, versioning)
│   ├── semantic_cache/  # Semantic caching (SemanticCache, CachedEntry, cosine similarity)
│   ├── storage/         # Storage abstraction (Storage trait, StorageEntity)
│   ├── workflow/        # Multi-step workflows (WorkflowExecutor, variable resolution)
│   ├── usage/           # Cost tracking (UsageRecord, Budget, ModelPricing)
│   ├── experiment/      # A/B testing (Experiment, Variant, TrafficAllocation, ExperimentResult)
│   ├── plugin/          # Plugin system (Plugin trait, ExtensionType, LlmProviderPlugin)
│   ├── operation/       # Async operations (Operation, OperationStatus, OperationRepository)
│   ├── config/          # App configuration & execution logs (AppConfiguration, ExecutionLog)
│   └── webhook/         # Webhook notifications (Webhook, WebhookDelivery, WebhookEventType)
└── infrastructure/      # External implementations
    ├── logging.rs       # Tracing setup
    ├── observability/   # OpenTelemetry tracing, Prometheus metrics
    ├── auth/            # JWT token management (JwtService, JwksJwtService with RSA support, JwtClaims, JwtConfig)
    ├── team/            # TeamService, StorageTeamRepository (uses Storage trait)
    ├── user/            # UserService, PasswordHasher (Argon2), InMemoryUserRepository, PostgresUserRepository
    ├── api_key/         # ApiKeyGenerator, RateLimiter, InMemoryApiKeyRepository, ApiKeyService
    ├── cache/           # InMemoryCache, RedisCache, CacheFactory
    ├── crag/            # ThresholdDocumentScorer, LlmDocumentScorer, CragPipeline
    ├── credentials/     # ENV, AWS Secrets, Vault providers
    ├── external_api/    # ExternalApiService
    ├── embedding/       # OpenAiEmbeddingProvider
    ├── ingestion/       # Parsers, Chunkers, IngestionPipeline, factories
    ├── knowledge_base/  # InMemoryKnowledgeBaseProvider, PgvectorKnowledgeBase, AwsKnowledgeBase, KnowledgeBaseProviderRegistry, factory
    ├── llm/             # LLM providers (OpenAI, Anthropic, Azure, Bedrock)
    ├── semantic_cache/  # InMemorySemanticCache
    ├── services/        # ModelService, PromptService, WorkflowService, OperationService, LlmCacheService, SemanticLlmCacheService, ExperimentService, ConfigService, ExecutionLogService, IngestionService
    ├── storage/         # InMemoryStorage, PostgresStorage, migrations
    ├── usage/           # UsageTrackingService, BudgetService, InMemoryUsageRepository, InMemoryBudgetRepository
    ├── workflow/        # WorkflowExecutorImpl, InMemoryWorkflowRepository
    ├── experiment/      # ConsistentHasher, statistical analysis, InMemoryExperimentRepository
    ├── plugin/          # PluginRegistry, ProviderRouter, RoutingProviderResolver, PluginConfig (TOML), builtin plugins
    ├── operation/       # InMemoryOperationRepository
    ├── config/          # StorageConfigRepository, StorageExecutionLogRepository
    └── webhook/         # WebhookService, InMemoryWebhookRepository, HMAC signatures
public/                      # Admin UI static files (jQuery + Tailwind CSS)
resources/e2e/               # Playwright E2E tests
├── tests/                   # Test specs (auth, navigation, models, prompts, api-keys)
└── playwright.config.ts     # Playwright configuration
k8s/                         # Kubernetes manifests (Kustomize)
├── base/                    # Base manifests (namespace, deployment, service, hpa, servicemonitor)
└── overlays/production/     # Production overlay
```

## Commands
```bash
cargo run serve               # API + UI combined (default)
cargo run api                 # API server only
cargo run ui                  # UI + proxy to http://localhost:3001
cargo run ui --api-url URL    # UI + proxy to custom API URL
cargo run ui --skip-proxy     # UI only (static files)
cargo test                    # Run tests (1604 tests)
cargo build --release         # Release build
bin/up.bat full              # Start all Docker services
bin/up.bat dev               # Start DB + run migrations + seed data
bin/down.bat                 # Stop Docker services
bin/test-integration.bat     # Run integration tests
bin/test-e2e.bat             # Run Playwright E2E tests
```

## Docker Profiles
- `full`: PostgreSQL + Redis + migrations
- `dev`: PostgreSQL + migrations + seed data (development only)
- `postgres`: PostgreSQL + migrations only
- `test`: Full test environment with mocks

## Environment Variables
- `ADMIN_DEFAULT_PASSWORD`: Set initial admin password (default: random, logged to console)
- `DATABASE_URL`: PostgreSQL connection string
- `USERS_JWKS`: JWKS JSON for JWT signing/validation (preferred, persists sessions across restarts)
- `JWT_SECRET`: Fallback secret for JWT signing (used if USERS_JWKS not set)
- `APP__STORAGE__BACKEND`: Storage backend ("postgres" default, "memory" for tests only)

## Key Features Implemented
- **LLM Providers**: OpenAI, Anthropic, Azure OpenAI, AWS Bedrock
- **Credentials**: ENV, AWS Secrets Manager, Vault with caching; StoredCredential entity with CRUD
- **Models**: ID validation, config versioning, credential association, CRUD service
- **Chains**: Fallback, retry with exponential backoff, circuit breaker, metrics
- **Prompts**: CRUD, versioning, variable templating `${var:name:default}`, rendering
- **Storage**: Generic Storage trait, InMemoryStorage, PostgresStorage with pooling, migrations
- **Cache**: Generic Cache trait, InMemoryCache (moka), RedisCache, LlmCacheService
- **Semantic Caching**: EmbeddingProvider trait, OpenAI embeddings, SemanticCache with cosine similarity, SemanticLlmCacheService
- **Knowledge Bases**: Pgvector, AWS Bedrock KB, InMemoryKnowledgeBaseProvider for dev mode; metadata filtering with FilterBuilder; default "default-kb" uses pgvector-default credential for database connection; document ingestion via admin API and UI; KnowledgeBaseProviderRegistry with lazy provider creation; KB connection_config supports credential_id for database credentials
- **Document Ingestion**: Parsers (TXT, Markdown, HTML, JSON), Chunkers (FixedSize, Sentence, Paragraph, Recursive), IngestionPipeline; IngestionService routes to actual KB providers (pgvector stores in PostgreSQL); list/delete documents by source; ensure_schema endpoint to create tables/indexes
- **CRAG**: DocumentScorer trait, LLM/Threshold/Hybrid scoring strategies, CragPipeline with knowledge base integration
- **API Keys**: Secure key generation (SHA256), ResourcePermission (All/Specific/None), sliding window RateLimiter, ApiKeyService
- **OpenAI API**: Chat completions, models endpoints, SSE streaming, prompt references, API key auth middleware
- **Admin API**: Models CRUD, Prompts CRUD, API Keys management (CRUD + suspend/activate/revoke), Workflows CRUD, Credentials CRUD, External APIs CRUD, Knowledge Bases CRUD, Experiments CRUD + lifecycle
- **Workflows**: Multi-step workflows with ChatCompletion (requires model_id, prompt_id, user_message), KnowledgeBaseSearch, CragScoring (requires model_id, prompt_id), Conditional, HttpRequest (requires external_api_id, optional credential_id); 7 built-in templates; 17 built-in prompts
- **External APIs**: Centralized configuration for HTTP request base URLs and headers; used by HttpRequest workflow steps; separates API configuration from authentication credentials
- **Async Operations**: Run chat completions and workflows async with `?async=true`; query/cancel operations via `/v1/operations/{id}`
- **Admin UI**: Embedded jQuery + Tailwind CSS SPA at `/ui/`; uses `/api/v1/*` endpoints; grouped sidebar (Resources, Access, Integrations, Testing, Operations); manages Models, Prompts, API Keys, Workflows, Credentials, External APIs, Knowledge Bases, Experiments, Budgets, Webhooks; CLI subcommands (serve, api, ui)
- **User Authentication**: Username/password login with JWT tokens for Admin UI; auto-creates admin user on first run; dual auth (API keys for services, JWT for UI); DATABASE_URL required for user persistence; USERS_JWKS (RSA/RS256) or JWT_SECRET env var for session persistence across restarts
- **Credential Testing**: Test LLM provider connections via `/admin/credentials/:id/test` endpoint; UI with Test button on credentials list
- **Workflow Mock Testing**: Test workflow execution with mocked step outputs via `/admin/workflows/:id/test`; UI for configuring input and step mocks
- **Observability**: OpenTelemetry tracing (OTLP export), Prometheus metrics (`/metrics`), structured JSON logging, graceful shutdown
- **Production Ready**: Kubernetes manifests (Kustomize), HPA, health probes with dependency checks, ServiceMonitor
- **Cost Tracking & Budgets**: UsageRecord with micro-dollar precision, ModelPricing with volume tiers, Budget with alerts/limits, usage analytics; BudgetScope (AllApiKeys, SpecificApiKeys, Teams, Mixed) for team-level and API key-level budgets
- **A/B Testing**: Experiment management (draft/active/paused/completed lifecycle), variants with model references or config overrides, traffic allocation with percentage-based distribution, consistent hashing for API key to variant assignment, per-variant metrics (latency, cost, tokens, success rate), Welch's t-test for statistical significance analysis
- **Plugin System**: Extensible provider architecture with Plugin trait, PluginRegistry, ProviderRouter; built-in plugins for OpenAI, Anthropic, Azure OpenAI, AWS Bedrock; per-request routing based on model's credential type; provider caching by (credential_type, credential_id); TOML configuration for plugin enable/disable (`plugins.toml.example`); RoutingProviderResolver for workflow execution with per-model provider resolution
- **Model Execution**: Direct model execution via `/admin/models/:id/execute` with prompt selection, variable substitution, and temperature/max_tokens overrides; UI with dynamic variable forms
- **Workflow Execution**: Direct workflow execution via `/admin/workflows/:id/execute` with JSON input; UI with input_schema-based forms and step-by-step result display
- **Test Cases**: Create and run test cases for model+prompt and workflow testing; assertion operators (contains, regex, JSON path, length checks); execution history with pass/fail tracking
- **App Configuration**: Key-value settings with categories (General, Persistence, Logging, Security, Cache, RateLimit); settings persisted via Storage trait; admin endpoints and UI for management
- **Execution Logs**: Track model/workflow/chat executions with status, cost, tokens, executor info; filterable logs with statistics; cleanup by retention period; uses Storage trait for persistence
- **Teams**: Team entity as organizational unit; all users and API keys must belong to a team; TeamRole (Owner/Admin/Member) for role-based permissions; built-in Administrators team; Team CRUD via admin API and UI
- **Webhooks**: HTTP callbacks for events (budget alerts, workflow failures, API key changes); HMAC-SHA256 signatures; configurable retries with exponential backoff; delivery tracking

## Current Status
1604 unit tests (75.47% coverage, target 90%) + 26 hurl integration test files. Coverage audit: `doc/COVERAGE_AUDIT.md`. All admin and v1 endpoint modules have comprehensive type tests. Remaining coverage gap is primarily infrastructure code requiring mocked dependencies. Models require an associated credential of the same provider type. Credentials cannot be deleted if models are assigned. Workflows require at least one step with ChatCompletion steps requiring prompt_id, CragScoring steps requiring model_id and prompt_id, HttpRequest steps requiring external_api_id (credential_id is optional for authentication). Resource IDs (model_id, prompt_id, knowledge_base_id, external_api_id, credential_id) must be configured directly in workflow steps, not as input variables.

## Integration Tests
Run with `bin/test-integration.bat` (Windows) or `docker compose --profile test up`.
Tests use pmp-mock-http (ironedge/pmp-mock-http:v0.4.0) to mock LLM provider APIs.

## E2E Tests
Run with `bin/test-e2e.bat` (Windows) or `bin/test-e2e.sh` (Linux/Mac).
Playwright tests for Admin UI authentication, navigation, and CRUD operations.
