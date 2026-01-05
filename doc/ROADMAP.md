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

### Status: Not Started

### Tasks

- [ ] **19.1** Implement structured logging
- [ ] **19.2** Add distributed tracing (OpenTelemetry)
- [ ] **19.3** Implement Prometheus metrics
- [ ] **19.4** Add request/response logging (redacted)
- [ ] **19.5** Implement graceful shutdown
- [ ] **19.6** Add health probes (liveness, readiness)
- [ ] **19.7** Create Kubernetes manifests
- [ ] **19.8** Write load tests
- [ ] **19.9** Security audit and hardening

---

## Future Enhancements (Backlog)

- [ ] Semantic caching for similar queries
- [ ] Multi-tenant support
- [ ] Webhook notifications
- [ ] A/B testing for models
- [ ] Cost tracking and budgets
- [ ] Additional LLM providers (Google Vertex, Cohere, etc.)
- [ ] Additional knowledge base providers (Pinecone, Weaviate, etc.)
- [ ] Plugin system for custom providers

---

## Legend

- [ ] Not started
- [x] Completed
- [~] In progress
