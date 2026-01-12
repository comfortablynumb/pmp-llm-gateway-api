# PMP LLM Gateway API - Roadmap

## Overview

This document tracks the development milestones for the LLM Gateway API, a Rust-based unified interface for multiple LLM providers with advanced features like model chaining, knowledge bases, and caching.

---

## Milestone 1: Project Foundation

### Status: Completed

### Tasks

- [x] **1.1** Initialize Rust project with Cargo workspace structure
- [x] **1.2** Set up project structure (src/domain, src/infrastructure, src/api, src/config)
- [x] **1.3** Configure dependencies (tokio, axum, serde, thiserror, async-trait)
- [x] **1.4** Create core traits and error types
- [x] **1.5** Set up logging and tracing infrastructure
- [x] **1.6** Create Docker Compose base configuration
- [x] **1.7** Create bin/up.sh, bin/up.bat, bin/down.sh, bin/down.bat scripts
- [x] **1.8** Initialize README.md and CLAUDE.md

---

## Milestone 2: Configuration & Credentials Management

### Status: Completed

### Tasks

- [x] **2.1** Define `CredentialProvider` trait
- [x] **2.2** Implement `EnvCredentialProvider` (environment variables)
- [x] **2.3** Implement `AwsSecretsCredentialProvider` (AWS Secrets Manager)
- [x] **2.4** Implement `VaultCredentialProvider` (HashiCorp Vault)
- [x] **2.5** Create `CredentialProviderFactory` for runtime selection
- [x] **2.6** Implement credential caching with TTL
- [x] **2.7** Add credential rotation support (via refresh method)
- [x] **2.8** Write unit tests with mocked providers

---

## Milestone 3: LLM Provider Integration

### Status: Completed

### Tasks

- [x] **3.1** Define `LlmProvider` trait with chat/completion methods
- [x] **3.2** Define `LlmRequest` and `LlmResponse` domain models
- [x] **3.3** Implement `OpenAiProvider`
- [x] **3.4** Implement `AnthropicProvider`
- [x] **3.5** Implement `AzureOpenAiProvider`
- [x] **3.6** Implement `BedrockProvider` (AWS Bedrock)
- [x] **3.7** Create `ProviderFactory` for dynamic provider instantiation
- [x] **3.8** Add streaming support for all providers
- [x] **3.9** Write unit tests with mocked HTTP clients

---

## Milestone 4: Model Configuration

### Status: Completed

### Tasks

- [x] **4.1** Define `Model` domain entity
  - ID: alphanumeric + hyphens, max 50 chars
  - Provider reference
  - Model name (provider-specific)
  - Configuration (temperature, max_tokens, top_p, etc.)
- [x] **4.2** Define `ModelRepository` trait
- [x] **4.3** Implement model validation (ID format, config constraints)
- [x] **4.4** Create model CRUD service
- [x] **4.5** Add model configuration versioning support
- [x] **4.6** Write unit tests

---

## Milestone 5: Model Chains (Fallback & Retry)

### Status: Completed

### Tasks

- [x] **5.1** Define `ModelChain` domain entity
  - Chain ID: alphanumeric + hyphens, max 50 chars
  - Ordered list of chain steps
- [x] **5.2** Define `ChainStep` with:
  - Model reference
  - Max retries
  - Max latency (ms)
  - Fallback behavior
- [x] **5.3** Implement `ChainExecutor` with:
  - Retry logic with exponential backoff
  - Latency threshold monitoring
  - Automatic fallback to next model
- [x] **5.4** Add circuit breaker pattern for failing models
- [x] **5.5** Implement chain metrics collection
- [x] **5.6** Write unit tests with simulated failures

---

## Milestone 6: Prompt Management

### Status: Completed

### Tasks

- [x] **6.1** Define `Prompt` domain entity
  - ID: alphanumeric + hyphens, max 50 chars
  - Name and description
  - Content with variable support
  - Version tracking
- [x] **6.2** Implement variable templating syntax: `${var:variable-name:default-value}`
  - Parse and extract variables from prompt content
  - Support optional default values
  - Validate variable names (alphanumeric + hyphens)
- [x] **6.3** Define `PromptRepository` trait
- [x] **6.4** Create prompt CRUD service
- [x] **6.5** Implement prompt rendering with variable substitution
- [x] **6.6** Add prompt reference support in LlmRequest (by ID)
- [x] **6.7** Implement prompt versioning and history
- [x] **6.8** Write unit tests

---

## Milestone 7: Storage Layer

### Status: Completed

### Tasks

- [x] **7.1** Define `Storage` trait (generic CRUD operations)
- [x] **7.2** Implement `InMemoryStorage`
- [x] **7.3** Implement `PostgresStorage`
- [x] **7.4** Add connection pooling for PostgresStorage
- [x] **7.5** Create database migrations infrastructure
- [x] **7.6** Implement `StorageFactory` for runtime selection
- [x] **7.7** Write unit tests with mocked storage

---

## Milestone 8: Cache Layer

### Status: Completed

### Tasks

- [x] **8.1** Define `Cache` trait
- [x] **8.2** Implement `InMemoryCache` with TTL support
- [x] **8.3** Implement `RedisCache`
- [x] **8.4** Add cache key generation strategy
- [x] **8.5** Implement cache invalidation patterns
- [x] **8.6** Create `CacheFactory` for runtime selection
- [x] **8.7** Add LLM response caching (semantic caching optional)
- [x] **8.8** Write unit tests with mocked cache

---

## Milestone 9: Knowledge Bases

### Status: Completed

### Tasks

- [x] **9.1** Define `KnowledgeBase` domain entity
  - ID: alphanumeric + hyphens, max 50 chars
  - Type: Pgvector, AWS Knowledge Base, Pinecone, Weaviate, Qdrant
  - Embeddings model reference with dimensions and provider
- [x] **9.2** Define `KnowledgeBaseProvider` trait
  - Search with similarity threshold and top_k
  - Document CRUD operations
  - Health check and document count
- [x] **9.3** Implement `PgvectorKnowledgeBase`
  - Vector similarity search with cosine/euclidean/inner product
  - Automatic embedding generation via `EmbeddingProvider` trait
  - Metadata filtering with SQL WHERE clause generation
- [x] **9.4** Implement `AwsKnowledgeBase`
  - AWS Bedrock Knowledge Base integration
  - Retrieval filter support for metadata
  - Read-only (documents managed via S3)
- [x] **9.5** Implement metadata filters:
  - FilterOperator: eq, ne, gt, gte, lt, lte, contains, startsWith, endsWith, in, notIn, exists, notExists
  - FilterConnector: AND, OR
  - Nested groups support with MetadataFilter enum
- [x] **9.6** Create filter query builder
  - Fluent builder API with FilterBuilder
  - Support for all operators and nested groups
- [x] **9.7** Write unit tests
- [x] **9.8** Create `KnowledgeBaseFactory` for runtime provider selection

---

## Milestone 10: RAG Document Ingestion

### Status: Completed

### Tasks

- [x] **10.1** Define `DocumentParser` trait
  - Parse raw files into text content
  - Support formats: TXT, Markdown, HTML, JSON (PDF deferred)
  - Extract document-level metadata (title, author, dates)
- [x] **10.2** Implement document parsers
  - `PlainTextParser` for .txt files
  - `MarkdownParser` for .md files (using pulldown-cmark)
  - `HtmlParser` for .html files (using scraper, strips tags)
  - `JsonParser` for JSON (full serialization)
  - PDF deferred to future milestone
- [x] **10.3** Define `ChunkingStrategy` trait
  - Split documents into chunks suitable for embedding
  - Return chunks with positional metadata
- [x] **10.4** Implement chunking strategies
  - `FixedSizeChunker`: chunk by character count with overlap
  - `SentenceChunker`: split by sentences using unicode-segmentation
  - `ParagraphChunker`: split by paragraphs (double newlines)
  - `RecursiveChunker`: hierarchical splitting (headers → paragraphs → sentences)
- [x] **10.5** Define `IngestionConfig` structure
  - Parser selection (auto-detect or explicit)
  - Chunking strategy and parameters (size, overlap)
  - Metadata extraction options
  - Batch size for embedding generation
- [x] **10.6** Implement `IngestionPipeline` service
  - Orchestrate: parse → chunk → store
  - Support single document and batch ingestion
  - Error handling with partial success reporting
- [x] **10.7** Implement metadata handling
  - Merge document-level and chunk-level metadata
  - Support custom metadata from API caller
  - Automatic metadata: chunk_index, total_chunks, document_id
- [x] **10.8** Add document update/replace workflow
  - Delete existing chunks by source document
  - Re-ingest with new content
  - Preserve document ID across updates
- [x] **10.9** Write unit tests (118 tests)
- [x] **10.10** Create `ParserFactory` and `ChunkerFactory`

---

## Milestone 11: CRAG (Corrective RAG)

### Status: Completed

### Tasks

- [x] **11.1** Define `CragConfig` structure
  - Evaluation prompt template with variable substitution
  - RelevanceClassification enum (Correct, Ambiguous, Incorrect)
  - Configurable score thresholds (correct_threshold, ambiguous_threshold)
  - ScoringStrategy enum (LlmBased, ThresholdBased, Hybrid)
- [x] **11.2** Implement document scoring service
  - `DocumentScorer` trait with async `score_document` and `score_documents` methods
  - `ScoredDocument` with relevance score, classification, and optional reason
  - `ScoringInput` for batch scoring requests
- [x] **11.3** Implement document filtering based on score
  - `CragFilter` for filtering scored documents by classification
  - `CragResult` with categorized documents (correct, ambiguous, incorrect)
  - `CragSummary` with statistics and percentages
- [x] **11.4** Add configurable scoring strategies
  - `ThresholdDocumentScorer`: Uses similarity scores with thresholds only
  - `LlmDocumentScorer`: Uses LLM for relevance evaluation with structured output
  - Hybrid scoring: Threshold first, then LLM for ambiguous documents
- [x] **11.5** Implement CRAG pipeline integration with knowledge bases
  - `CragPipeline<K, P>`: Combines knowledge base search with scoring
  - Supports all three strategies with automatic fallback
  - `needs_correction()` and `should_try_web_search()` helpers
- [x] **11.6** Write unit tests (48 new tests, total 411 tests)

---

## Milestone 12: API Keys & Permissions

### Status: Completed

### Tasks

- [x] **12.1** Define `ApiKey` domain entity
  - `ApiKeyId`: alphanumeric + hyphens, max 50 chars
  - `ApiKeyStatus`: Active, Suspended, Revoked, Expired
  - SHA256 hashed secret with constant-time comparison
  - `ApiKeyPermissions`: models, knowledge bases, prompts, chains, admin flag
  - `ResourcePermission`: All, Specific(set), None
  - `RateLimitConfig`: per-minute, per-hour, per-day, tokens-per-minute
  - Expiration with `expires_at` timestamp
  - Usage tracking with `last_used_at` and `record_usage()`
- [x] **12.2** Define `ApiKeyRepository` trait
  - CRUD operations with get, create, update, delete
  - `get_by_prefix` for authentication lookup
  - `record_usage` for access tracking
  - `get_expiring_before` for expiration management
  - `MockApiKeyRepository` for testing
- [x] **12.3** Implement API key generation (secure random)
  - `ApiKeyGenerator` with configurable prefix (pk_live_, pk_test_)
  - 32 bytes of cryptographically secure random data
  - Base64 URL-safe encoding
  - Unique prefix = type prefix + 8 random chars for lookup
  - SHA256 hashing for storage
  - Constant-time comparison for verification
- [x] **12.4** Implement `InMemoryApiKeyRepository`
  - Thread-safe with RwLock
  - Key index and prefix index for fast lookups
  - Conflict detection for duplicate IDs/prefixes
- [x] **12.5** Add permission checking for model access
  - `can_access_model()` with ResourcePermission matching
- [x] **12.6** Add permission checking for knowledge base access
  - `can_access_knowledge_base()`, `can_access_prompt()`, `can_access_chain()`
- [x] **12.7** Implement rate limiting per API key
  - `RateLimiter` with sliding window algorithm
  - Per-minute, per-hour, per-day limits
  - Token-based rate limiting for LLM requests
  - `RateLimitResult` with remaining/reset info
  - Auto-cleanup of old records every 5 minutes
- [x] **12.8** Implement `ApiKeyService`
  - Create, validate, suspend, revoke, activate, delete
  - Rate limit checking and permission checking
  - Update permissions and rate limits
- [x] **12.9** Write unit tests (65 new tests, total 476 tests)

---

## Milestone 13: OpenAI-Compatible API

### Status: Completed

### Tasks

- [x] **13.1** Set up Axum router structure
  - Router with state support (`create_router_with_state`)
  - Nested `/v1` routes for OpenAI-compatible endpoints
  - TraceLayer middleware for request logging
- [x] **13.2** Implement health check endpoints (`/health`, `/ready`, `/live`)
- [x] **13.3** Implement OpenAI-compatible chat completions endpoint
  - `POST /v1/chat/completions` - matches OpenAI API format
  - Support `model` field to reference gateway model IDs
  - Support `stream` parameter for SSE streaming
  - Request validation (messages, temperature, top_p, penalties)
- [x] **13.4** Implement OpenAI-compatible models endpoint
  - `GET /v1/models` - list available enabled models
  - `GET /v1/models/{model_id}` - get model details
- [x] **13.5** Add prompt reference support in messages
  - Allow `{"role": "system", "prompt_id": "my-prompt", "variables": {...}}`
  - Variable substitution via PromptService.render_by_id
- [x] **13.6** Implement streaming with Server-Sent Events (SSE)
  - Initial chunk with role, content chunks, finish chunk, [DONE] marker
  - Uses tokio channels for async streaming
- [x] **13.7** Add request/response mapping between OpenAI format and internal format
  - ChatMessage ↔ Message conversion
  - ChatCompletionResponse/ChatCompletionStreamResponse builders
- [x] **13.8** Implement error responses in OpenAI format
  - ApiError with status codes and error types
  - ApiErrorType enum: InvalidRequestError, AuthenticationError, etc.
  - DomainError → ApiError conversion
- [x] **13.9** Implement API key authentication middleware
  - RequireApiKey extractor for protected endpoints
  - Bearer token and X-API-Key header support
  - Key validation, expiration, and status checks
- [x] **13.10** Write unit tests (504 tests total)

---

## Milestone 14: Admin API

### Status: Completed

### Tasks

- [x] **14.1** Implement credential provider info endpoint (list providers)
- [x] **14.2** Implement model endpoints (CRUD)
- [x] **14.3** Implement prompt endpoints (CRUD)
- [x] **14.4** Implement API key management endpoints (CRUD + status operations)
- [x] **14.5** Add admin authentication via RequireApiKey extractor
- [x] **14.6** Write unit tests

### Notes

- Admin API available at `/admin/*` endpoints
- All endpoints require API key with `admin: true` permission
- Model chain CRUD and knowledge base CRUD deferred (runtime entities, not stored)
- Bulk import/export deferred to future milestone

---

## Milestone 15: Workflows

### Status: Completed

### Description

Configurable multi-step workflows that chain operations together. A workflow receives custom input in the request that can be referenced by any node. Supports conditional branching, structured outputs, and integration with all gateway features.

### Variable Reference Syntax

- `${request:field}` - Reference to workflow execution request input
- `${request:field:default}` - With default value
- `${step:step-name:field}` - Reference to previous step output
- `${step:step-name:field:default}` - With default value

### Example Workflow

```json
{
  "id": "answer-with-context",
  "name": "Answer with Context",
  "input_schema": {
    "type": "object",
    "properties": { "question": {"type": "string"} },
    "required": ["question"]
  },
  "steps": [
    {
      "name": "search",
      "type": "knowledge_base_search",
      "knowledge_base_id": "product-docs",
      "query": "${request:question}",
      "top_k": 10
    },
    {
      "name": "check-results",
      "type": "conditional",
      "conditions": [{
        "field": "${step:search:documents}",
        "operator": "is_empty",
        "action": {"end_workflow": {"answer": "No results found"}}
      }],
      "default_action": "continue"
    },
    {
      "name": "score",
      "type": "crag_scoring",
      "input_documents": "${step:search:documents}",
      "query": "${request:question}",
      "threshold": 0.5,
      "strategy": "hybrid"
    },
    {
      "name": "answer",
      "type": "chat_completion",
      "model_id": "gpt-4",
      "user_message": "Question: ${request:question}\n\nContext:\n${step:score:correct_documents}"
    }
  ],
  "enabled": true
}
```

### Tasks

- [x] **15.1** Define `Workflow` domain entity (ID, name, steps, input schema)
- [x] **15.2** Define `WorkflowStep` types:
  - ChatCompletion (model_id, prompt_id, user_message, system_message, temperature, max_tokens)
  - KnowledgeBaseSearch (knowledge_base_id, query, top_k, similarity_threshold, filter)
  - CragScoring (input_documents, query, threshold, strategy, prompt_id)
  - Conditional (conditions with field/operator/value, actions: continue/go_to_step/end_workflow)
- [x] **15.3** Implement `WorkflowContext` for variable resolution (`${step:name:field}`, `${request:field}`)
- [x] **15.4** Implement `WorkflowExecutor` with step-by-step execution
- [x] **15.5** Define `WorkflowRepository` trait and in-memory implementation
- [x] **15.6** Create `WorkflowService` for CRUD and execution operations
- [x] **15.7** Add workflow admin endpoints (CRUD: GET, POST, PUT, DELETE)
- [x] **15.8** Add workflow execution endpoint (`POST /v1/workflows/{id}/execute`)
- [x] **15.9** Implement workflow validation (step validation, reference checking)
- [x] **15.10** Write unit tests (98 new tests, total 604 tests)

### Notes

- OnErrorAction: FailWorkflow or SkipStep
- ConditionOperator: Eq, Ne, Gt, Gte, Lt, Lte, IsEmpty, IsNotEmpty, Contains
- ConditionalAction: Continue, GoToStep(name), EndWorkflow(output)
- ScoringStrategy: Threshold, Llm, Hybrid

---

## Milestone 16: Async Operations

### Status: Completed

### Description

Enable any AI operation (workflows, chat completions, etc.) to run asynchronously. Returns HTTP 202 with an operation ID that can be queried for status and results.

### API Design

```
# Start async operation
POST /v1/chat/completions?async=true
→ 202 Accepted
→ { "operation_id": "op-abc123", "status": "pending" }

# Start async workflow
POST /v1/workflows/{id}/execute?async=true
→ 202 Accepted
→ { "operation_id": "op-xyz789", "status": "pending" }

# Query single operation
GET /v1/operations/{id}
→ { "operation_id": "...", "status": "completed|pending|failed", "result": {...}, "error": {...} }

# Query multiple operations
GET /v1/operations?ids=op-abc123,op-xyz789
→ { "operations": [...] }

# Cancel operation
DELETE /v1/operations/{id}
→ { "operation_id": "...", "status": "cancelled" }
```

### Tasks

- [x] **16.1** Define `Operation` entity (ID, type, status, input, result, error, timestamps)
- [x] **16.2** Define `OperationStatus` enum (Pending, Running, Completed, Failed, Cancelled)
- [x] **16.3** Implement `OperationRepository` trait with in-memory backend
- [x] **16.4** Create `OperationService` for managing operations
- [x] **16.5** Implement async execution layer (tokio spawn with result capture)
- [x] **16.6** Add `?async=true` query param support to chat completions
- [x] **16.7** Add `?async=true` query param support to workflow execution
- [x] **16.8** Implement operation query endpoints (GET single, GET batch)
- [x] **16.9** Implement operation cancellation
- [x] **16.10** Add operation TTL and cleanup (configurable retention in OperationService)
- [x] **16.11** Write unit tests (39 new tests, total 643 tests)

### Notes

- Operation ID format: `op-{uuid}` (validated)
- State transitions: Pending → Running → Completed/Failed, or Pending/Running → Cancelled
- Background task execution using tokio::spawn
- Redis backend deferred to future milestone

---

## Milestone 17: Integration Tests

### Status: Completed

### Description

Comprehensive integration testing using hurl with Docker Compose infrastructure. All external API dependencies (LLM providers) are mocked using pmp-mock-http to enable reliable, repeatable tests.

### Infrastructure

```
docker-compose.yml (test profile)
├── app                    # Gateway API built from Dockerfile
├── hurl                   # hurl test runner
├── mock-openai            # pmp-mock-http for OpenAI API
├── mock-anthropic         # pmp-mock-http for Anthropic API
├── mock-azure-openai      # pmp-mock-http for Azure OpenAI API
└── postgres               # PostgreSQL for storage tests
```

### Tasks

- [x] **17.1** Create Dockerfile for the gateway API
- [x] **17.2** Set up pmp-mock-http service for OpenAI API mocking
- [x] **17.3** Set up pmp-mock-http service for Anthropic API mocking
- [x] **17.4** Set up pmp-mock-http service for Azure OpenAI API mocking
- [x] **17.5** Create docker-compose.yml with test profile
- [x] **17.6** Create hurl tests for health endpoints
- [x] **17.7** Create hurl tests for chat completions (non-streaming)
- [x] **17.8** Create hurl tests for chat completions (streaming SSE)
- [x] **17.9** Create hurl tests for models endpoints
- [x] **17.10** Create hurl tests for prompts admin endpoints
- [x] **17.11** Create hurl tests for API keys admin endpoints
- [x] **17.12** Create hurl tests for workflows admin endpoints
- [x] **17.13** Create hurl tests for workflow execution
- [x] **17.14** Create hurl tests for async operations
- [x] **17.15** Create hurl tests for authentication and authorization
- [x] **17.16** Create hurl tests for error handling scenarios
- [x] **17.17** Add CI script to run integration tests

### Notes

- pmp-mock-http: https://github.com/comfortablynumb/pmp-mock-http
- Mock configurations define expected requests and canned responses
- Tests validate full request/response cycle through the gateway
- Streaming tests verify SSE format and chunk handling
- Run tests with: `bin/test-integration.bat` (Windows) or `bin/test-integration.sh` (Linux/Mac)

---

## Milestone 18: Admin UI

### Status: Completed

### Description

Embedded Admin UI using jQuery + Tailwind CSS (via CDN), served by the Axum server. CLI subcommands separate API-only from API+UI modes.

### CLI Commands

```bash
pmp-llm-gateway serve              # API + UI combined (default)
pmp-llm-gateway api                # API server only
pmp-llm-gateway ui                 # UI + proxy /api/* to http://localhost:3001
pmp-llm-gateway ui --api-url URL   # UI + proxy to custom API URL
pmp-llm-gateway ui --skip-proxy    # UI only (static files, no proxy)
```

### Tasks

- [x] **18.1** Choose UI framework: Embedded jQuery + Tailwind CSS
- [x] **18.2** Implement CLI subcommands (serve, api, ui) with clap
- [x] **18.3** Implement API proxy for UI command
- [x] **18.4** Create main SPA layout (sidebar, header, content area, auth modal)
- [x] **18.5** Implement authentication via sessionStorage (API key input)
- [x] **18.6** Create dashboard view (summary cards, quick actions, credential providers)
- [x] **18.7** Create models management view (CRUD)
- [x] **18.8** Create prompts management view (CRUD + render preview)
- [x] **18.9** Create API keys management view (CRUD + suspend/activate/revoke)
- [x] **18.10** Create workflows management view (CRUD with JSON editor)
- [x] **18.11** Create credentials view (read-only providers list)

### Notes

- UI served at `/ui/` with root redirect
- API proxy rewrites `/api/*` to `/admin/*`
- Model chains and knowledge bases views deferred (runtime entities)
- E2E tests deferred to future milestone

---

## Milestone 19: Observability & Production Readiness

### Status: Completed

### Tasks

- [x] **19.1** Implement structured logging (JSON format with tracing)
- [x] **19.2** Add distributed tracing (OpenTelemetry with OTLP export)
- [x] **19.3** Implement Prometheus metrics (/metrics endpoint, HTTP metrics middleware)
- [x] **19.4** Add request/response logging (redacted headers, sensitive JSON fields)
- [x] **19.5** Implement graceful shutdown (SIGTERM/SIGINT handling)
- [x] **19.6** Enhance health probes (liveness, readiness with dependency checks)
- [x] **19.7** Create Kubernetes manifests (Kustomize-based, HPA, ServiceMonitor)
- [x] **19.8** Write load tests (k6 tests for health and chat endpoints)
- [x] **19.9** Security audit and hardening (security headers, input validation, SECURITY.md)

### Notes

- Observability module: `src/infrastructure/observability/`
- Security middleware: `src/api/middleware/security.rs`
- K8s manifests: `k8s/base/` with production overlay
- Load tests: `tests/load/` with k6 scripts
- Security checklist: `doc/SECURITY.md`
- 771 unit tests passing

---

## Milestone 20: Semantic Caching

### Status: Completed

### Description

Semantic caching for LLM responses that matches semantically similar queries using embeddings, rather than requiring exact key matches. This enables cache hits for queries that are similar in meaning but differ in wording.

### Architecture

```
┌────────────────────┐     ┌─────────────────────┐
│  LLM Request       │────▶│ SemanticLlmCache    │
└────────────────────┘     │ Service             │
                           │ ┌─────────────────┐ │
                           │ │ Generate Query  │ │
                           │ │ Embedding       │ │
                           │ └────────┬────────┘ │
                           │          ▼          │
                           │ ┌─────────────────┐ │
                           │ │ Vector Search   │ │
                           │ │ (cosine sim)    │ │
                           │ └────────┬────────┘ │
                           │          ▼          │
                           │ ┌─────────────────┐ │
                           │ │ Return if       │ │
                           │ │ sim > threshold │ │
                           │ └─────────────────┘ │
                           └─────────────────────┘
```

### Tasks

- [x] **20.1** Define `EmbeddingProvider` trait (embed, provider_name, default_model, dimensions)
- [x] **20.2** Define `EmbeddingRequest` and `EmbeddingResponse` types
- [x] **20.3** Implement `OpenAiEmbeddingProvider` for OpenAI embeddings API
- [x] **20.4** Define `SemanticCache` trait (search, store, delete, stats)
- [x] **20.5** Define `SemanticCacheConfig` (similarity_threshold, max_entries, ttl, embedding_model)
- [x] **20.6** Define `CachedEntry` with embedding vector, query text, value, metadata
- [x] **20.7** Implement `InMemorySemanticCache` with linear search
- [x] **20.8** Implement `SemanticLlmCacheService` combining cache + embedding provider
- [x] **20.9** Add cosine similarity calculation utility
- [x] **20.10** Write unit tests (55 new tests, 826 total)

### Key Types

```rust
// Domain: EmbeddingProvider trait
pub trait EmbeddingProvider: Send + Sync + Debug {
    async fn embed(&self, request: EmbeddingRequest) -> Result<EmbeddingResponse, DomainError>;
    fn provider_name(&self) -> &'static str;
    fn default_model(&self) -> &'static str;
    fn dimensions(&self, model: &str) -> Option<usize>;
}

// Domain: SemanticCache trait
pub trait SemanticCache: Send + Sync + Debug {
    async fn search(&self, embedding: &[f32], params: &SemanticSearchParams) -> Result<Vec<SemanticSearchResult>, DomainError>;
    async fn find_similar(&self, embedding: &[f32], params: &SemanticSearchParams) -> Result<Option<SemanticSearchResult>, DomainError>;
    async fn store(&self, entry: CachedEntry) -> Result<(), DomainError>;
    async fn delete(&self, id: &str) -> Result<bool, DomainError>;
    async fn stats(&self) -> Result<SemanticCacheStats, DomainError>;
}

// Config
pub struct SemanticCacheConfig {
    pub enabled: bool,
    pub similarity_threshold: f32,  // Default: 0.95
    pub max_entries: usize,         // Default: 10000
    pub ttl_secs: u64,              // Default: 3600
    pub embedding_model: String,    // Default: text-embedding-3-small
    pub include_model_in_key: bool, // Cache per model
}
```

### Notes

- Uses cosine similarity for vector matching
- Default similarity threshold is 0.95 (very similar queries)
- Model filtering ensures responses are only matched for same model
- Streaming requests not cached by default (configurable)
- InMemorySemanticCache suitable for development; PgvectorSemanticCache available for production
- Integration with chat endpoint is opt-in via AppState configuration

---

## Milestone 21: Cost Tracking & Budgets

### Status: Completed

### Description

Usage tracking and budget management for LLM API requests. Tracks token usage, costs (in micro-dollars for precision), and enforces spending limits with configurable alerts.

### Architecture

```
┌────────────────────┐     ┌─────────────────────┐
│  LLM Request       │────▶│ UsageTrackingService│
└────────────────────┘     │ - Record usage      │
                           │ - Calculate cost    │
                           │ - Query history     │
                           └──────────┬──────────┘
                                      │
                           ┌──────────▼──────────┐
                           │   BudgetService     │
                           │ - Check limits      │
                           │ - Trigger alerts    │
                           │ - Reset periods     │
                           └─────────────────────┘
```

### Tasks

- [x] **21.1** Define `UsageRecord` entity (id, type, api_key_id, model_id, tokens, cost_micros, latency)
- [x] **21.2** Define `UsageType` enum (ChatCompletion, Embedding, Workflow, KnowledgeBaseSearch)
- [x] **21.3** Define `UsageAggregate` for aggregated statistics (totals, averages, breakdowns)
- [x] **21.4** Define `UsageSummary` with daily breakdown
- [x] **21.5** Define `ModelPricing` entity with volume-based tiers
- [x] **21.6** Implement default pricing for common models (GPT-4o, Claude 3.5, etc.)
- [x] **21.7** Define `Budget` entity (id, period, limits, alerts, status)
- [x] **21.8** Define `BudgetPeriod` enum (Daily, Weekly, Monthly, Lifetime)
- [x] **21.9** Define `BudgetStatus` enum (Active, Warning, Exceeded, Paused)
- [x] **21.10** Define `BudgetAlert` with configurable thresholds
- [x] **21.11** Define `UsageRepository` and `BudgetRepository` traits
- [x] **21.12** Implement `InMemoryUsageRepository` with eviction
- [x] **21.13** Implement `InMemoryBudgetRepository`
- [x] **21.14** Create `UsageTrackingService` (record, query, aggregate, summary)
- [x] **21.15** Create `BudgetService` (CRUD, check_budget, record_usage, reset_periods)
- [x] **21.16** Add admin endpoints for usage (list, aggregate, summary, delete)
- [x] **21.17** Add admin endpoints for budgets (CRUD, check, reset)
- [x] **21.18** Write unit tests (869 total)

### Key Types

```rust
// Domain: UsageRecord
pub struct UsageRecord {
    pub id: UsageRecordId,
    pub usage_type: UsageType,
    pub api_key_id: String,
    pub model_id: Option<String>,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub total_tokens: u32,
    pub cost_micros: i64,  // Micro-dollars for precision
    pub latency_ms: u64,
    pub success: bool,
    pub timestamp: u64,
}

// Domain: Budget
pub struct Budget {
    pub id: BudgetId,
    pub name: String,
    pub period: BudgetPeriod,
    pub hard_limit_micros: i64,
    pub soft_limit_micros: Option<i64>,
    pub current_usage_micros: i64,
    pub status: BudgetStatus,
    pub api_key_ids: Vec<String>,  // Empty = all keys
    pub model_ids: Vec<String>,    // Empty = all models
    pub alerts: Vec<BudgetAlert>,
    pub enabled: bool,
}

// Domain: ModelPricing
pub struct ModelPricing {
    pub model_id: String,
    pub provider: String,
    pub input_price_per_1k_micros: i64,
    pub output_price_per_1k_micros: i64,
    pub tiers: Vec<PricingTier>,  // Volume discounts
}
```

### Admin API Endpoints

```
# Usage Tracking
GET    /admin/usage                  # List usage records (with filters)
GET    /admin/usage/aggregate        # Get aggregated usage stats
GET    /admin/usage/summary          # Get summary with daily breakdown
DELETE /admin/usage                  # Delete old usage records

# Budget Management
GET    /admin/budgets                # List all budgets
POST   /admin/budgets                # Create budget
GET    /admin/budgets/{id}           # Get budget
PUT    /admin/budgets/{id}           # Update budget
DELETE /admin/budgets/{id}           # Delete budget
POST   /admin/budgets/{id}/reset     # Reset budget period
POST   /admin/budgets/check          # Check if request is allowed
```

### Notes

- Costs stored in micro-dollars (1 USD = 1,000,000 micros) to avoid floating-point precision issues
- Budget alerts trigger at configurable percentage thresholds (e.g., 50%, 75%, 90%)
- Usage records can be filtered by API key, model, and time range
- Budget periods auto-reset (Daily, Weekly, Monthly) or Lifetime (no reset)
- Budgets can be scoped to specific API keys and/or models

---

## Milestone 22: A/B Testing for Models

### Status: Completed

### Description

A/B testing system for comparing different LLM models and configurations. Enables data-driven decisions about model selection based on metrics like latency, cost, and success rate.

### Features

- **Experiment entity**: id, name, status (Draft/Active/Paused/Completed), variants, traffic allocation
- **Variant types**: ModelReference (use existing model) or ConfigOverride (same model, different config)
- **Assignment**: Consistent hashing by API key - same key always gets same variant
- **Traffic allocation**: Percentage-based (e.g., 50/50, 80/20), must sum to 100
- **Metrics tracking**: Per-variant latency (avg/p50/p95/p99), cost, token usage, success rate
- **Statistical analysis**: Welch's t-test for significance testing with confidence levels

### API Endpoints

```
GET    /admin/experiments           # List all experiments
POST   /admin/experiments           # Create experiment
GET    /admin/experiments/:id       # Get experiment
PUT    /admin/experiments/:id       # Update experiment
DELETE /admin/experiments/:id       # Delete experiment
POST   /admin/experiments/:id/start # Start experiment
POST   /admin/experiments/:id/pause # Pause experiment
POST   /admin/experiments/:id/resume # Resume experiment
POST   /admin/experiments/:id/complete # Complete experiment
GET    /admin/experiments/:id/results # Get results with metrics
```

### Tasks

- [x] **22.1** Define `Experiment`, `Variant`, `ExperimentStatus` entities
- [x] **22.2** Define `ExperimentId`, `VariantId` newtypes with validation
- [x] **22.3** Define `VariantConfig` enum (ModelReference, ConfigOverride)
- [x] **22.4** Define `TrafficAllocation` with percentage validation
- [x] **22.5** Define `ExperimentRecord` for tracking requests
- [x] **22.6** Define `VariantMetrics`, `ExperimentResult` for analytics
- [x] **22.7** Define `ExperimentRepository` trait
- [x] **22.8** Define `ExperimentRecordRepository` trait
- [x] **22.9** Implement `InMemoryExperimentRepository`
- [x] **22.10** Implement `InMemoryExperimentRecordRepository`
- [x] **22.11** Implement `ConsistentHasher` for API key → variant assignment
- [x] **22.12** Implement `ExperimentService` (CRUD, lifecycle, assignment, results)
- [x] **22.13** Add `ExperimentServiceTrait` to AppState
- [x] **22.14** Create admin endpoints for experiments
- [x] **22.15** Integrate experiment routing into chat endpoint
- [x] **22.16** Implement statistical significance calculation (Welch's t-test)
- [x] **22.17** Create Admin UI for experiments management
- [x] **22.18** Write unit tests (979 tests total)
- [x] **22.19** Create hurl integration tests

### File Structure

```
src/domain/experiment/
├── mod.rs, entity.rs, result.rs, record.rs, repository.rs, assignment.rs, validation.rs
src/infrastructure/experiment/
├── mod.rs, in_memory_repository.rs, in_memory_record_repo.rs, consistent_hashing.rs, statistical.rs
src/infrastructure/services/experiment_service.rs
src/api/admin/experiments.rs
public/js/views/experiments.js
tests/integration/hurl/20-admin-experiments.hurl
```

### Notes

- Experiment lifecycle: Draft → Active ↔ Paused → Completed
- Consistent hashing ensures same API key always routes to same variant
- ConfigOverride allows testing different parameters (temperature, max_tokens, top_p, presence_penalty, frequency_penalty) on same model
- Results include Welch's t-test for latency and cost comparisons with p-values and confidence levels
- Winner recommendation based on specified primary metric

---

## Milestone 23: Plugin System

### Status: Completed

### Description

Compiled-in plugin registry for extensible provider support. Enables multiple LLM providers to be active simultaneously with per-request routing based on model configuration.

### Features

- **Plugin trait**: metadata, extension types, initialize, health_check, shutdown
- **Extension types**: LlmProvider, EmbeddingProvider, KnowledgeBaseProvider, CredentialProvider, StorageBackend
- **PluginRegistry**: Central registry keyed by credential type
- **ProviderRouter**: Routes model_id → Model entity → plugin → provider instance
- **Provider caching**: By (credential_type, credential_id) for efficiency
- **Built-in plugins**: OpenAI, Anthropic, Azure OpenAI, AWS Bedrock

### Plugin Traits

```rust
#[async_trait]
pub trait Plugin: Send + Sync + Debug {
    fn metadata(&self) -> &PluginMetadata;
    fn extension_types(&self) -> Vec<ExtensionType>;
    async fn initialize(&self, context: PluginContext) -> Result<(), PluginError>;
    async fn health_check(&self) -> Result<bool, PluginError>;
    async fn shutdown(&self) -> Result<(), PluginError>;
    fn state(&self) -> PluginState;
}

#[async_trait]
pub trait LlmProviderPlugin: Plugin {
    fn supported_credential_types(&self) -> Vec<CredentialType>;
    fn supports_credential_type(&self, cred_type: &CredentialType) -> bool;
    async fn create_llm_provider(&self, config: LlmProviderConfig) -> Result<Arc<dyn LlmProvider>, PluginError>;
    fn available_models(&self) -> Vec<&'static str>;
}
```

### Tasks

- [x] **23.1** Define `Plugin` trait with metadata and lifecycle methods
- [x] **23.2** Define `ExtensionType` enum
- [x] **23.3** Define `PluginMetadata`, `PluginContext`, `PluginState`, `PluginError`
- [x] **23.4** Define `LlmProviderPlugin` trait
- [x] **23.5** Define `EmbeddingProviderPlugin` trait
- [x] **23.6** Define `KnowledgeBaseProviderPlugin` trait
- [x] **23.7** Define `CredentialProviderPlugin` trait
- [x] **23.8** Implement `PluginRegistry`
- [x] **23.9** Implement `ProviderRouter` with caching
- [x] **23.10** Wrap OpenAI provider as `OpenAiPlugin`
- [x] **23.11** Wrap Anthropic provider as `AnthropicPlugin`
- [x] **23.12** Wrap Azure OpenAI provider as `AzureOpenAiPlugin`
- [x] **23.13** Wrap Bedrock provider as `BedrockPlugin`
- [x] **23.14** Add `ProviderRouter` to AppState
- [x] **23.15** Update chat endpoint to use ProviderRouter
- [x] **23.16** Update workflow executor to use ProviderRouter
- [x] **23.17** Add plugin configuration loading (TOML)
- [x] **23.18** Write unit tests (1235 tests total)

### File Structure

```
src/domain/llm/
├── provider_resolver.rs       # ProviderResolver trait, StaticProviderResolver
src/domain/plugin/
├── mod.rs, error.rs, extensions.rs, entity.rs
├── llm_provider.rs, embedding_provider.rs, knowledge_base.rs, credential_provider.rs
src/infrastructure/plugin/
├── mod.rs, registry.rs, router.rs
├── config.rs                  # PluginConfig, TOML loading
├── routing_resolver.rs        # RoutingProviderResolver for workflows
├── builtin/
│   ├── mod.rs, openai.rs, anthropic.rs, azure.rs, bedrock.rs
plugins.toml.example           # Sample plugin configuration file
```

### Notes

- Built-in plugins registered automatically at startup via `register_builtin_plugins()`
- Can selectively enable/disable plugins via `register_builtin_plugins_with_config()`
- ProviderRouter caches provider instances by (credential_type, credential_id)
- Chat endpoint uses `get_provider_for_model()` helper which falls back to default provider
- Workflow executor uses `RoutingProviderResolver` for per-model provider resolution
- Plugin state management: Registered → Initializing → Ready → ShuttingDown → Stopped
- Plugin configuration via `plugins.toml` (copy from `plugins.toml.example`)

---

## Milestone 24: Model & Workflow Execution Endpoints

### Status: Completed

### Description

Add dedicated endpoints to execute specific models and workflows directly, with corresponding UI views. This enables users to test and run models with prompts and workflows without going through the chat completions API.

### Tasks

- [x] **24.1** Add `POST /admin/models/:id/execute` endpoint
  - Accept prompt_id, variables (optional), user_message
  - Return model response with usage info
- [x] **24.2** Add `POST /admin/workflows/:id/execute` endpoint
  - Accept workflow input JSON matching input_schema
  - Return workflow execution result with step outputs
- [x] **24.3** Create Model Execution UI view
  - Select model from list
  - Select prompt from list
  - Input variable values (dynamic form based on prompt variables)
  - Execute and display response
- [x] **24.4** Create Workflow Execution UI view
  - Select workflow from list
  - Input form generated from input_schema
  - Execute and display step-by-step results
- [x] **24.5** Write unit tests (8 new tests)
- [x] **24.6** Create hurl integration tests (21-execute-endpoints.hurl)

---

## Milestone 25: Test Cases

### Status: Completed

### Description

Add the ability to create, store, and run test cases for model+prompt combinations and workflows. This enables regression testing and quality assurance for AI configurations.

### Tasks

- [x] **25.1** Define `TestCase` domain entity
  - ID, name, description, type (ModelPrompt, Workflow)
  - Input configuration (model_id, prompt_id, variables OR workflow_id, input)
  - Expected output criteria (contains, regex match, JSON path, length checks)
  - Created/updated timestamps, tags, enabled status
- [x] **25.2** Define `TestCaseResult` entity
  - Test case reference, execution timestamp
  - Pass/fail status, actual output, execution time, token usage
  - Assertion results (which criteria passed/failed with actual values)
- [x] **25.3** Define `TestCaseRepository` trait and in-memory implementation
- [x] **25.4** Implement `TestCaseService` for CRUD and execution
- [x] **25.5** Add admin endpoints for test cases (CRUD)
  - `GET /admin/test-cases` - List test cases (with filters: test_type, enabled, tag, model_id, workflow_id)
  - `POST /admin/test-cases` - Create test case
  - `GET /admin/test-cases/:id` - Get test case
  - `PUT /admin/test-cases/:id` - Update test case
  - `DELETE /admin/test-cases/:id` - Delete test case
  - `POST /admin/test-cases/:id/execute` - Execute single test case
  - `GET /admin/test-cases/:id/results` - Get test case execution history
- [x] **25.6** Create Test Cases UI view
  - List, create, edit, delete test cases
  - Execute individual tests with inline result display
  - View test execution history with detailed result inspection
- [x] **25.7** Write unit tests (1086 tests total)
- [x] **25.8** Create hurl integration tests (22-admin-test-cases.hurl)

### File Structure

```
src/domain/test_case/
├── mod.rs, entity.rs, result.rs, repository.rs, validation.rs
src/infrastructure/test_case/
├── mod.rs, repository.rs (InMemoryTestCaseRepository, InMemoryTestCaseResultRepository)
src/infrastructure/services/test_case_service.rs
src/api/admin/test_cases.rs
public/js/views/test-cases.js
tests/integration/hurl/22-admin-test-cases.hurl
```

### Assertion Operators

- `Contains`, `NotContains` - Text containment checks
- `Equals`, `NotEquals` - Exact string matching
- `Regex` - Regular expression matching
- `JsonPathExists`, `JsonPathEquals` - JSON path validation
- `LengthGreaterThan`, `LengthLessThan` - Output length checks

### Notes

- Test cases can be filtered by type (model_prompt, workflow), enabled status, and tags
- Assertions are evaluated against the response content
- Test execution uses the ProviderRouter for model requests
- Results include token usage and execution time metrics

---

## Milestone 26: Application Configuration & Execution Persistence

### Status: Completed

### Description

Add a configurations section to change and persist application settings. Key feature: ability to persist execution logs for selected workflows and models with full audit trail. Configuration and execution logs use the existing Storage trait infrastructure for persistence.

### Tasks

- [x] **26.1** Define `AppConfiguration` domain entity
  - Key-value settings with types (string, bool, int, float, list)
  - Categories: General, Persistence, Logging, Security, Cache, RateLimit
  - `ConfigEntry` with key, value, category, description, updated_at
  - Implements `StorageEntity` trait for storage layer integration
- [x] **26.2** Define `ExecutionLog` entity
  - ID, execution_type (Model, Workflow, ChatCompletion), resource_id, resource_name
  - Status (Success, Failed, Timeout, Cancelled)
  - Cost (micro-dollars), token usage (input, output, total)
  - Executor (user_id, api_key_id, ip_address, user_agent)
  - Execution time (ms), error details, timestamps
  - Implements `StorageEntity` trait for storage layer integration
- [x] **26.3** Define `ConfigRepository` trait using Storage trait pattern
- [x] **26.4** Define `ExecutionLogRepository` trait using Storage trait pattern
- [x] **26.5** Implement `ConfigService` for settings management
  - Get/set/reset configuration entries
  - List all or by category
  - Default configuration on first access
- [x] **26.6** Implement `ExecutionLogService` for logging and queries
  - Create, get, delete, list with filters
  - Statistics (total, success rate, costs, token usage, by type/resource)
  - Cleanup old logs based on retention period
- [x] **26.7** Add admin endpoints for configuration
  - `GET /admin/config` - Get all settings
  - `GET /admin/config/category/:category` - Get by category
  - `GET /admin/config/:key` - Get specific setting
  - `PUT /admin/config/:key` - Update setting
  - `DELETE /admin/config` - Reset to defaults
- [x] **26.8** Add admin endpoints for execution logs
  - `GET /admin/execution-logs` - List logs with filters (type, status, resource, api_key, user, date range, limit, offset)
  - `GET /admin/execution-logs/stats` - Get execution statistics
  - `GET /admin/execution-logs/:id` - Get log details
  - `DELETE /admin/execution-logs/:id` - Delete specific log
  - `POST /admin/execution-logs/cleanup` - Delete old logs
- [x] **26.9** Create Configuration UI view
  - Settings grouped by category
  - Edit modal for each setting with type-specific inputs
  - Reset to defaults button
- [x] **26.10** Create Execution Logs UI view
  - Statistics dashboard (total, success rate, avg time, cost)
  - Filterable log list with pagination
  - Log detail modal with executor info
  - Cleanup dialog for old logs
- [x] **26.11** Write unit tests (1120 tests total)
- [x] **26.12** Create hurl integration tests (23-admin-config.hurl, 24-admin-execution-logs.hurl)

### File Structure

```
src/domain/config/
├── mod.rs, entity.rs, execution_log.rs, repository.rs
src/infrastructure/config/
├── mod.rs, repository.rs (StorageConfigRepository, StorageExecutionLogRepository)
src/infrastructure/services/
├── config_service.rs, execution_log_service.rs
src/api/admin/
├── config.rs, execution_logs.rs
public/js/views/
├── configuration.js, execution-logs.js
tests/integration/hurl/
├── 23-admin-config.hurl, 24-admin-execution-logs.hurl
```

### Default Configuration Entries

- **General**: default_timezone
- **Persistence**: log_retention_days
- **Logging**: execution_logging_enabled, log_sensitive_data
- **Security**: (reserved)
- **Cache**: (reserved)
- **RateLimit**: (reserved)

### Notes

- Uses existing Storage trait infrastructure (in-memory or PostgreSQL based on config)
- AppConfiguration uses singleton pattern with fixed key "app_config"
- ExecutionLogService checks configuration before logging (enabled, sensitive data)
- Statistics include breakdowns by execution type and resource

---

## Milestone 27: Teams & Multi-User Management

### Status: Completed

### Description

Add team support as the primary organizational unit. All users must belong to a team, and API keys are owned by teams (not users). This enables shared resource access and collective budget management.

### Key Constraints

- Every user must belong to exactly one team
- Every API key must be owned by exactly one team
- The "admin" user is auto-assigned to an "Administrators" team on first run
- The "Administrators" team cannot be suspended or deleted

### Tasks

- [x] **27.1** Define `Team` domain entity
  - `TeamId`: alphanumeric + hyphens, max 50 chars
  - `TeamStatus`: Active, Suspended
  - `TeamRole`: Owner, Admin, Member with privilege hierarchy
  - Created/updated timestamps, name, description
- [x] **27.2** Update `User` entity
  - Add `team_id` (required, all users must belong to a team)
  - Add `team_role` enum (Owner, Admin, Member) for user's role within their team
- [x] **27.3** Update `ApiKey` entity
  - Add `team_id` (required, API keys are always owned by a team)
- [x] **27.4** Define `TeamRepository` trait and in-memory implementation
- [x] **27.5** Implement `TeamService` for CRUD operations
  - Create, get, list, update, delete
  - Suspend and activate teams
  - `ensure_administrators_team()` for auto-creation
- [x] **27.6** Update `UserService` for team assignment (team_id required in CreateUserRequest)
- [x] **27.7** Update `ApiKeyService` for team ownership (team_id required in create)
- [x] **27.8** Update initialization logic
  - Create "Administrators" team on first run via `ensure_administrators_team()`
  - Assign auto-created "admin" user to "Administrators" team with Owner role
- [x] **27.9** Add admin endpoints for teams
  - `GET /admin/teams` - List teams
  - `POST /admin/teams` - Create team
  - `GET /admin/teams/:id` - Get team
  - `PUT /admin/teams/:id` - Update team
  - `DELETE /admin/teams/:id` - Delete team
  - `POST /admin/teams/:id/suspend` - Suspend team
  - `POST /admin/teams/:id/activate` - Activate team
- [x] **27.10** Create Teams UI view
  - Team list, create, edit, delete
  - Suspend/activate actions
- [x] **27.11** Update API Keys UI to show/assign team
  - Added team column to table
  - Added team selector in create form
- [x] **27.12** Write unit tests (1178 tests total)
- [x] **27.13** Create hurl integration tests (25-admin-teams.hurl)

### File Structure

```
src/domain/team/
├── mod.rs, entity.rs, repository.rs, validation.rs
src/infrastructure/team/
├── mod.rs, repository.rs (StorageTeamRepository - uses Storage trait), service.rs (TeamService)
src/api/admin/teams.rs
public/js/views/teams.js
tests/integration/hurl/25-admin-teams.hurl
```

### Notes

- TeamId has a special constant `ADMINISTRATORS` for the built-in team
- Cannot suspend or delete the administrators team
- TeamRole privilege hierarchy: Owner > Admin > Member
- User's team_role determines permissions within their team

---

## Milestone 28: Shared Budgets for Teams & API Keys

### Status: Completed

### Description

Extend the budget system to support shared budgets across multiple API keys and/or teams. Since API keys are owned by teams, budgets can be scoped at team level (all team's API keys share the budget) or at specific API key level for finer control.

### Budget Scopes

- **AllApiKeys**: Budget applies to all API keys (default, empty filters)
- **SpecificApiKeys**: Budget applies only to specific API keys
- **Teams**: Budget applies to all API keys belonging to specific teams
- **Mixed**: Budget applies to specific API keys AND/OR team-level budgets

### Tasks

- [x] **28.1** Update `Budget` entity with scope and team support
  - Add `BudgetScope` enum: AllApiKeys, SpecificApiKeys, Teams, Mixed
  - Add `team_ids` field to Budget entity
  - Keep `api_key_ids` for API key-specific budgets
  - Auto-calculate scope based on api_key_ids and team_ids
- [x] **28.2** Update budget applicability logic
  - `applies_to_api_key(api_key_id)` - Check without team context
  - `applies_to_team(team_id)` - Check team membership
  - `applies_to_api_key_with_team(api_key_id, team_id)` - Check with full context
- [x] **28.3** Update `BudgetRepository` trait
  - `find_applicable(api_key_id, model_id)` - Find budgets without team
  - `find_applicable_with_team(api_key_id, team_id, model_id)` - Find with team context
  - `find_by_team(team_id)` - Find all team-applicable budgets
- [x] **28.4** Update `BudgetService`
  - `list_by_team(team_id)` - List budgets for a team
  - `check_budget_with_team(api_key_id, team_id, model_id, cost)` - Check with team context
  - `record_usage_with_team(api_key_id, team_id, model_id, cost)` - Record with team context
- [x] **28.5** Update budget admin endpoints
  - Add `team_ids` support in create/update requests
  - Add `scope` field in responses
  - `GET /admin/budgets/by-team/:team_id` - List budgets for a team
  - `POST /admin/budgets/check` - Check with optional `team_id` parameter
- [x] **28.6** Write unit tests (1186 tests total, 8 new for budget scope)
- [x] **28.7** Update hurl integration tests (19-admin-budgets.hurl)

### Admin API Endpoints

```
GET    /admin/budgets                     # List all budgets (includes scope, team_ids)
POST   /admin/budgets                     # Create budget (with optional team_ids)
GET    /admin/budgets/:id                 # Get budget (includes scope, team_ids)
PUT    /admin/budgets/:id                 # Update budget (with optional team_ids)
DELETE /admin/budgets/:id                 # Delete budget
POST   /admin/budgets/:id/reset           # Reset budget period
GET    /admin/budgets/by-team/:team_id    # List budgets applicable to team
POST   /admin/budgets/check               # Check budget (with optional team_id)
```

### Notes

- Scope auto-calculated from api_key_ids and team_ids
- Empty both = AllApiKeys, only api_keys = SpecificApiKeys, only teams = Teams, both = Mixed
- Team-scoped budgets apply to all current and future API keys of that team
- Mixed scope allows organization-wide + specific key limits
---

## Milestone 29: Budget UI View

### Status: Completed

### Description

Add the Admin UI view for budget management, enabling users to create, edit, and manage budgets with team and API key assignment through the web interface.

### Tasks

- [x] **29.1** Add budget API methods to api.js
  - listBudgets, getBudget, createBudget, updateBudget, deleteBudget
  - resetBudget, checkBudget, listBudgetsByTeam
- [x] **29.2** Create budgets.js view file
  - List view with usage progress bars and status badges
  - Create/Edit form with team and API key multi-select
  - Period selection (daily, weekly, monthly, lifetime)
  - Alert threshold configuration
  - Scope display (All/Specific/Teams/Mixed)
- [x] **29.3** Add navigation item to index.html
- [x] **29.4** Register route in app.js

### UI Features

- **List View**: Shows all budgets with usage percentage bars, current/limit amounts, status badges
- **Scope Display**: Shows scope type with team/key counts
- **Create/Edit Form**:
  - ID, name, description
  - Period selection
  - Hard limit and soft limit (warning threshold)
  - Alert threshold percentages
  - Team multi-select
  - API key multi-select
  - Enabled toggle
- **Actions**: Edit, Reset period, Delete

### Notes

- Teams and API keys are loaded on form open for selection
- Scope is auto-calculated by the backend based on team_ids and api_key_ids
- Usage progress bars color-coded: green (<75%), yellow (75-90%), red (>90%)

---

## Milestone 30: Webhook Notifications

### Status: Completed

### Description

Add webhook notification support for event-driven integrations. Webhooks enable external systems to receive real-time notifications when specific events occur in the gateway, such as budget alerts, workflow completions, or API key state changes.

### Tasks

- [x] **30.1** Define Webhook domain entities
  - `WebhookId` - Unique identifier for webhooks
  - `Webhook` - Configuration with URL, secret, events, retry settings
  - `WebhookEventType` - Enum of supported events (budget_alert, budget_exceeded, workflow_failed, etc.)
  - `WebhookStatus` - Active, Disabled, Failing states
  - `WebhookDeliveryId`, `WebhookDelivery` - Delivery tracking with retry support
  - `DeliveryStatus` - Pending, Success, Failed, Retrying states
  - `WebhookEvent` - Event payload with type and data
- [x] **30.2** Define repository traits
  - `WebhookRepository` - CRUD + find_by_event, find_active_by_event
  - `WebhookDeliveryRepository` - CRUD + find_by_webhook, find_pending_retries, cleanup
- [x] **30.3** Implement InMemory repositories
  - `InMemoryWebhookRepository`
  - `InMemoryWebhookDeliveryRepository`
- [x] **30.4** Create WebhookService
  - CRUD operations with validation
  - `send_event(event)` - Dispatch to subscribed webhooks
  - `retry_failed_deliveries()` - Process pending retries
  - `cleanup_deliveries(retention_days)` - Remove old deliveries
  - HMAC-SHA256 signature generation for payload verification
  - Retry logic with exponential backoff
- [x] **30.5** Add webhook admin endpoints
  - `GET /admin/webhooks` - List all webhooks
  - `POST /admin/webhooks` - Create webhook
  - `GET /admin/webhooks/:id` - Get webhook
  - `PUT /admin/webhooks/:id` - Update webhook
  - `DELETE /admin/webhooks/:id` - Delete webhook
  - `POST /admin/webhooks/:id/reset` - Reset failure count and re-enable
  - `GET /admin/webhooks/:id/deliveries` - Get delivery history
  - `GET /admin/webhooks/event-types` - List available event types
- [x] **30.6** Create Webhook UI view
  - List view with status badges and failure counts
  - Create/Edit form with event type selection
  - Delivery history viewer
  - Reset action for failing webhooks
- [x] **30.7** Integrate webhooks into budget alerts
  - `send_budget_alerts(notifications)` - Send BudgetAlert events
  - `send_budget_exceeded(budget_id, ...)` - Send BudgetExceeded events
- [x] **30.8** Write unit tests (31 new tests)
- [x] **30.9** Create hurl integration tests (26-admin-webhooks.hurl)

### Webhook Event Types

| Event Type | Description |
|------------|-------------|
| budget_alert | Budget threshold reached (e.g., 50%, 80%, 90%) |
| budget_exceeded | Budget hard limit exceeded |
| experiment_completed | A/B experiment finished |
| workflow_failed | Workflow execution failed |
| workflow_succeeded | Workflow execution completed |
| model_failed | Model execution failed |
| api_key_suspended | API key suspended |
| api_key_revoked | API key revoked |
| test_case_failed | Test case execution failed |

### Webhook Payload

```json
{
  "id": "uuid",
  "event_type": "budget_alert",
  "timestamp": "2024-01-15T10:30:00Z",
  "data": {
    "budget_id": "monthly-budget",
    "budget_name": "Monthly API Budget",
    "threshold_percent": 80,
    "current_usage_micros": 80000000,
    "current_usage_dollars": 80.0,
    "limit_micros": 100000000,
    "limit_dollars": 100.0,
    "usage_percent": 80.0
  }
}
```

### Security

- Optional HMAC-SHA256 signature in `X-Webhook-Signature` header
- Signature format: `sha256=<hex_encoded_signature>`
- Custom headers support for authentication
- Configurable timeout (default 30s)

### Retry Configuration

- `max_retries` - Maximum delivery attempts (default: 3)
- `retry_delay_secs` - Delay between retries (default: 60s)
- Webhooks auto-disabled after `max_retries * 3` consecutive failures

---

## Milestone 31: Production Persistence & Test Coverage

### Status: Phase 2 In Progress (75.47% coverage)

### Description

Review the entire codebase to ensure production readiness by replacing in-memory persistence with PostgreSQL where appropriate. All persistence must use the trait-based Storage abstraction. Additionally, enhance test coverage to 90% with unit tests and ensure all features are covered by integration tests.

### Tasks

#### Phase 1: Persistence Review

- [x] **31.1** Audit codebase for in-memory persistence usage
  - Identified all `InMemory*Repository` implementations
  - Documented in `doc/PERSISTENCE_AUDIT.md`
  - Listed repositories missing PostgreSQL implementations
- [x] **31.2** Implement missing PostgreSQL repositories
  - Created Storage*Repository wrappers for all custom repository traits:
    - `StorageOperationRepository`
    - `StorageUsageRepository`, `StorageBudgetRepository`
    - `StorageExperimentRepository`, `StorageExperimentRecordRepository`
    - `StorageTestCaseRepository`, `StorageTestCaseResultRepository`
    - `StorageStoredCredentialRepository`
    - `StorageWebhookRepository`, `StorageWebhookDeliveryRepository`
    - `StorageApiKeyRepository`
  - Added missing migrations (external_apis, experiment_records, test_case_results)
  - Added `StorageFactory::create_postgres_with_pool()` method
- [x] **31.3** Verify trait-based storage pattern
  - All domain code uses repository traits
  - Storage*Repository wrappers implement domain traits using generic Storage<E>
  - Factory pattern supports runtime selection via `create_postgres_with_pool()`
- [x] **31.4** Update configuration for persistence selection
  - Added `storage.backend` config option (default: "memory")
  - Supports "memory" or "postgres" values
  - Environment variable: `APP__STORAGE__BACKEND`
- [x] **31.5** Reconstruct lib.rs with all services
  - lib.rs fully rebuilt with all 21 services required by AppState
  - Codebase compiles successfully with 1310 tests passing
- [x] **31.6** Integrate storage backend selection in lib.rs
  - Updated `create_app_state_with_config()` to read `config.storage.backend`
  - Uses StorageFactory.create_postgres_with_pool() when backend is "postgres"
  - Uses Storage*Repository wrappers with PostgresStorage
  - Modified services to use trait objects (Arc<dyn Storage<T>>) for runtime polymorphism
  - Refactored `WorkflowExecutorImpl`, `WorkflowService`, `KnowledgeBaseService`, `LazyKnowledgeBaseProviderRegistry` to use trait objects
- [x] **31.7** Test PostgreSQL persistence
  - All 1310 unit tests pass
  - Release build succeeds
  - docker-compose.yml updated with `APP__STORAGE__BACKEND=postgres` for integration tests
  - Integration tests require Docker (run with `bin/test-integration.bat`)

#### Phase 2: Unit Test Coverage (Target: 90%)

- [x] **31.8** Measure current test coverage
  - Used `cargo llvm-cov` for coverage analysis
  - Initial coverage: 72.83%, improved to 75.47%
  - Documented in `doc/COVERAGE_AUDIT.md`
  - Identified lowest coverage modules: API admin endpoints (4-25%)
- [x] **31.9** Add unit tests for domain layer
  - Entity validation tests (mostly complete, >85% coverage)
  - Business logic tests
  - Error handling tests
- [~] **31.10** Add unit tests for infrastructure layer
  - Repository implementations (mocked storage) - pending, requires mocked dependencies
  - Service implementations (mocked dependencies)
  - Provider implementations (mocked HTTP clients)
- [x] **31.11** Add unit tests for API layer
  - Request validation tests - COMPLETE (all 14 admin modules + 4 v1 modules)
  - Added 294 new tests for API types (request/response serialization, helper functions, From implementations)
  - Total tests: 1604 (up from 1310)
- [~] **31.12** Verify 90% coverage achieved
  - Current coverage: 75.47% (improved from 72.83%)
  - API layer coverage significantly improved
  - Remaining gap is primarily infrastructure code requiring mocked external dependencies (Redis, Postgres)
  - Some modules excluded (lib.rs, CLI code, migrations)

#### Phase 3: Integration Test Coverage

- [ ] **31.13** Audit existing integration tests
  - List all features without integration tests
  - Document test gaps
- [ ] **31.14** Add integration tests for missing features
  - Semantic caching endpoints
  - Knowledge base document ingestion
  - CRAG pipeline execution
  - Cost tracking and usage endpoints
  - All webhook event types
- [ ] **31.15** Add integration tests for edge cases
  - Error responses for all endpoints
  - Rate limiting behavior
  - Authentication failures
  - Concurrent request handling
- [ ] **31.16** Verify all features are covered
  - Run full integration test suite
  - Fix any features that fail (DO NOT modify tests to pass)
  - Document any known limitations

### Critical Rules

1. **Never modify tests just to make them pass** - If a test fails, investigate and fix the actual feature
2. **Code against traits, not implementations** - All persistence must use the Storage trait abstraction
3. **Mock external dependencies in unit tests** - No tests should depend on databases, caches, or external services
4. **Fix features, not tests** - If integration tests reveal bugs, fix the underlying functionality

### Notes

- Current test count: 1440 unit tests + 26 hurl integration test files
- Coverage: ~75% (target: 90%)
- Coverage audit: `doc/COVERAGE_AUDIT.md`
- All `InMemory*` implementations should have `Postgres*` or `Storage*` counterparts
- Integration tests use pmp-mock-http for LLM provider mocking
- Run integration tests with `bin/test-integration.bat`

---

## Milestone 32: Developer Experience & E2E Testing

### Status: Completed

### Description

Improve developer experience with configurable admin defaults and add end-to-end browser testing with Playwright.

### Tasks

- [x] **32.1** Add configurable default admin password
  - Add `ADMIN_DEFAULT_PASSWORD` environment variable support
  - Use this password when auto-creating the admin user on first run
  - Set default value to "admin" in docker-compose.yml
  - Document in README.md
- [x] **32.2** Add Playwright E2E tests
  - Create `resources/e2e` folder for Playwright tests
  - Set up Playwright configuration (playwright.config.ts)
  - Create tests for Admin UI authentication flow (auth.spec.ts)
  - Create tests for core CRUD operations (models.spec.ts, prompts.spec.ts, api-keys.spec.ts)
  - Create tests for navigation (navigation.spec.ts)
  - Add E2E test scripts to bin/ folder (test-e2e.sh, test-e2e.bat)

### File Structure

```
resources/e2e/
├── package.json           # Dependencies (Playwright)
├── playwright.config.ts   # Playwright configuration
├── .gitignore             # Ignore node_modules, reports
└── tests/
    ├── fixtures.ts        # Helper functions (login, navigate)
    ├── auth.spec.ts       # Authentication tests
    ├── navigation.spec.ts # Navigation tests
    ├── models.spec.ts     # Models CRUD tests
    ├── prompts.spec.ts    # Prompts CRUD tests
    └── api-keys.spec.ts   # API Keys CRUD tests

bin/
├── test-e2e.sh            # Run E2E tests (Linux/Mac)
└── test-e2e.bat           # Run E2E tests (Windows)
```

### Running E2E Tests

```bash
# Start the application first
bin/up.bat test              # Windows
bin/up.sh test               # Linux/Mac

# Run E2E tests
bin/test-e2e.bat             # Windows
bin/test-e2e.sh              # Linux/Mac

# Run with browser visible
bin/test-e2e.bat --headed    # Windows
bin/test-e2e.sh --headed     # Linux/Mac

# Run against custom URL
bin/test-e2e.bat --url http://localhost:3000
```

### Notes

- E2E tests run against the full Docker Compose stack
- Uses `ADMIN_DEFAULT_PASSWORD=admin` for test authentication
- Tests are independent and idempotent
- Playwright reports available in `resources/e2e/playwright-report/`

---

## Future Enhancements (Backlog)

- [ ] Multi-tenant support
- [x] Webhook notifications
- [ ] Additional LLM providers (Google Vertex, Cohere, etc.)
- [ ] Additional knowledge base providers (Pinecone, Weaviate, etc.)

---

## Legend

- [ ] Not started
- [x] Completed
- [~] In progress
