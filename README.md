# PMP LLM Gateway API

A Rust-based unified gateway for multiple LLM providers with advanced features.

## Features

- **OpenAI-Compatible API**: Drop-in replacement for OpenAI API with `/v1/chat/completions`
- **Multi-Provider Support**: OpenAI, Anthropic, Azure OpenAI, AWS Bedrock
- **Credential Management**: ENV, AWS Secrets Manager, HashiCorp Vault
- **Model Configuration**: Custom model definitions with provider mapping
- **Model Chains**: Fallback chains with retry logic and latency thresholds
- **Knowledge Bases**: Pgvector, AWS Knowledge Base with metadata filtering
- **CRAG Support**: Corrective RAG with configurable scoring
- **Workflows**: Multi-step workflows with chat completions, KB search, CRAG scoring, and conditionals
- **Caching**: In-memory and Redis strategies
- **Storage**: In-memory and PostgreSQL strategies
- **API Keys**: Permission-based access control with rate limiting
- **Streaming**: Server-Sent Events (SSE) for real-time responses
- **Admin UI**: Embedded web UI for managing models, prompts, API keys, and workflows

## Quick Start

### Prerequisites

- Rust 1.75+
- Docker & Docker Compose

### Development Setup

```bash
# Clone and enter directory
cd pmp-llm-gateway-api

# Start infrastructure (PostgreSQL, Redis, pgvector)
bin/up.bat full    # Windows
bin/up.sh full     # Linux/Mac

# Run the server (API + UI)
cargo run serve

# Server starts on http://localhost:8080
# Admin UI available at http://localhost:8080/ui/
```

### CLI Commands

```bash
# API + UI combined (default)
cargo run serve

# API server only
cargo run api

# UI server only (proxies to API at localhost:3001)
cargo run ui

# UI with custom API URL
cargo run ui --api-url http://localhost:8080

# UI without proxy (static files only)
cargo run ui --skip-proxy
```

### Configuration

Configuration can be set via:

1. `config/default.toml` - Default values
2. `config/local.toml` - Local overrides (gitignored)
3. Environment variables with `APP__` prefix

Example environment variables:
```bash
APP__SERVER__PORT=3000
APP__LOGGING__LEVEL=debug
```

## API Endpoints

### Health Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/health` | GET | Health check with version |
| `/ready` | GET | Readiness probe |
| `/live` | GET | Liveness probe |

### OpenAI-Compatible API (v1)

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/v1/chat/completions` | POST | Chat completions (streaming supported) |
| `/v1/chat/completions?async=true` | POST | Async chat completion (returns operation ID) |
| `/v1/models` | GET | List available models |
| `/v1/models/{model_id}` | GET | Get model details |
| `/v1/workflows/{id}/execute` | POST | Execute a workflow |
| `/v1/workflows/{id}/execute?async=true` | POST | Async workflow execution |
| `/v1/operations/{id}` | GET | Get operation status and result |
| `/v1/operations?ids=id1,id2` | GET | Get multiple operations |
| `/v1/operations/{id}` | DELETE | Cancel an operation |

#### Authentication

All `/v1/*` endpoints require API key authentication:

```bash
# Using Authorization header
curl -H "Authorization: Bearer sk-your-api-key" \
  http://localhost:8080/v1/models

# Using X-API-Key header
curl -H "X-API-Key: sk-your-api-key" \
  http://localhost:8080/v1/models
```

#### Chat Completions

```bash
# Non-streaming request
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Authorization: Bearer sk-your-api-key" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4",
    "messages": [
      {"role": "system", "content": "You are a helpful assistant."},
      {"role": "user", "content": "Hello!"}
    ]
  }'

# Streaming request
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Authorization: Bearer sk-your-api-key" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4",
    "messages": [{"role": "user", "content": "Hello!"}],
    "stream": true
  }'

# With prompt reference
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Authorization: Bearer sk-your-api-key" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4",
    "messages": [
      {
        "role": "system",
        "prompt_id": "assistant-prompt",
        "variables": {"topic": "programming"}
      },
      {"role": "user", "content": "Tell me more"}
    ]
  }'
```

#### Async Operations

Run long-running operations asynchronously by adding `?async=true`:

```bash
# Start async chat completion
curl -X POST "http://localhost:8080/v1/chat/completions?async=true" \
  -H "Authorization: Bearer sk-your-api-key" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4",
    "messages": [{"role": "user", "content": "Write a long essay..."}]
  }'
# Returns: {"operation_id": "op-abc123", "status": "pending", "message": "..."}

# Start async workflow execution
curl -X POST "http://localhost:8080/v1/workflows/my-workflow/execute?async=true" \
  -H "Authorization: Bearer sk-your-api-key" \
  -H "Content-Type: application/json" \
  -d '{"input": {"question": "Complex question..."}}'
# Returns: {"operation_id": "op-xyz789", "status": "pending", "message": "..."}

# Check operation status
curl "http://localhost:8080/v1/operations/op-abc123" \
  -H "Authorization: Bearer sk-your-api-key"
# Returns: {"operation_id": "...", "status": "completed", "result": {...}}

# Check multiple operations
curl "http://localhost:8080/v1/operations?ids=op-abc123,op-xyz789" \
  -H "Authorization: Bearer sk-your-api-key"

# Cancel an operation
curl -X DELETE "http://localhost:8080/v1/operations/op-abc123" \
  -H "Authorization: Bearer sk-your-api-key"
```

Operation statuses: `pending`, `running`, `completed`, `failed`, `cancelled`

### Admin API

All admin endpoints require API key with `admin: true` permission.

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/admin/models` | GET | List all models |
| `/admin/models` | POST | Create a model |
| `/admin/models/{id}` | GET | Get model by ID |
| `/admin/models/{id}` | PUT | Update model |
| `/admin/models/{id}` | DELETE | Delete model |
| `/admin/prompts` | GET | List all prompts |
| `/admin/prompts` | POST | Create a prompt |
| `/admin/prompts/{id}` | GET | Get prompt by ID |
| `/admin/prompts/{id}` | PUT | Update prompt |
| `/admin/prompts/{id}` | DELETE | Delete prompt |
| `/admin/prompts/{id}/render` | POST | Render prompt with variables |
| `/admin/api-keys` | GET | List all API keys |
| `/admin/api-keys` | POST | Create API key (returns secret) |
| `/admin/api-keys/{id}` | GET | Get API key by ID |
| `/admin/api-keys/{id}` | PUT | Update API key permissions |
| `/admin/api-keys/{id}` | DELETE | Delete API key |
| `/admin/api-keys/{id}/suspend` | POST | Suspend API key |
| `/admin/api-keys/{id}/activate` | POST | Activate suspended key |
| `/admin/api-keys/{id}/revoke` | POST | Permanently revoke key |
| `/admin/workflows` | GET | List all workflows |
| `/admin/workflows` | POST | Create a workflow |
| `/admin/workflows/{id}` | GET | Get workflow by ID |
| `/admin/workflows/{id}` | PUT | Update workflow |
| `/admin/workflows/{id}` | DELETE | Delete workflow |
| `/admin/credentials/providers` | GET | List credential provider types |

#### Examples

```bash
# List models (requires admin API key)
curl -H "Authorization: Bearer sk-admin-key" \
  http://localhost:8080/admin/models

# Create a model
curl -X POST http://localhost:8080/admin/models \
  -H "Authorization: Bearer sk-admin-key" \
  -H "Content-Type: application/json" \
  -d '{
    "id": "gpt-4-custom",
    "name": "Custom GPT-4",
    "provider": "openai",
    "provider_model": "gpt-4",
    "config": {"temperature": 0.7, "max_tokens": 2048}
  }'

# Create API key with specific permissions
curl -X POST http://localhost:8080/admin/api-keys \
  -H "Authorization: Bearer sk-admin-key" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Client Key",
    "permissions": {
      "admin": false,
      "models": "all",
      "prompts": {"specific": ["greeting", "support"]}
    }
  }'
```

## Docker Compose Profiles

| Profile | Services |
|---------|----------|
| `full` | PostgreSQL, Redis, pgvector |
| `postgres` | PostgreSQL only |
| `redis` | Redis only |
| `pgvector` | pgvector only |
| `test` | App, mock services, hurl (for integration tests) |

```bash
# Start specific profile
bin/up.bat postgres
bin/up.sh redis
```

## Integration Tests

Integration tests use [hurl](https://hurl.dev/) with mocked LLM providers via [pmp-mock-http](https://github.com/comfortablynumb/pmp-mock-http).

```bash
# Run integration tests
bin/test-integration.bat    # Windows
bin/test-integration.sh     # Linux/Mac
```

Test files are located in `tests/integration/hurl/` (17 files, 123 requests):
- Health endpoints (liveness, readiness, health check)
- Chat completions (streaming and non-streaming, with parameters)
- Chat with prompt references and variable substitution
- Models API (list, get, CRUD operations)
- Admin endpoints (prompts, API keys, workflows, models, credentials)
- Workflow execution with variable resolution
- API key lifecycle (create, suspend, activate, revoke, delete)
- API key authentication (Bearer and X-API-Key headers)
- Async operations (async chat, operation status)
- Model provider configuration (OpenAI, Anthropic, Azure)
- Request validation and error handling

## Project Structure

```
src/
├── api/               # HTTP endpoints and middleware
├── config/            # Application configuration
├── domain/            # Business logic and entities
│   ├── error.rs       # Domain errors
│   ├── cache/         # Cache trait and key generation
│   ├── credentials/   # Credential types and provider trait
│   ├── knowledge_base/# Knowledge base entities and filtering
│   ├── llm/           # LLM request/response models and provider trait
│   ├── model/         # Model configuration entities
│   ├── chain/         # Model chains with fallback and retry
│   ├── prompt/        # Prompt management with templating
│   ├── storage/       # Storage trait and entity abstractions
│   └── traits/        # Core traits (Repository, etc.)
└── infrastructure/    # External service implementations
    ├── logging.rs     # Tracing setup
    ├── cache/         # Cache implementations (InMemory, Redis)
    ├── credentials/   # Credential providers (ENV, AWS Secrets, Vault)
    ├── knowledge_base/# Knowledge base providers (Pgvector, AWS)
    ├── llm/           # LLM providers (OpenAI, Anthropic, Azure, Bedrock)
    ├── services/      # Business services (ModelService, PromptService, LlmCacheService)
    └── storage/       # Storage implementations (InMemory, PostgreSQL)
```

## LLM Providers

All providers implement the `LlmProvider` trait with streaming support:

```rust
// Example: Creating an OpenAI provider
let provider = LlmProviderFactory::create_openai("sk-your-key");
let request = LlmRequest::builder()
    .system("You are helpful")
    .user("Hello!")
    .build();
let response = provider.chat("gpt-4o", request).await?;
```

### Supported Providers

| Provider | Models | Features |
|----------|--------|----------|
| OpenAI | gpt-4o, gpt-4o-mini, gpt-4-turbo, gpt-4, gpt-3.5-turbo | Chat, Streaming |
| Anthropic | claude-opus-4-5, claude-sonnet-4, claude-3-5-sonnet, claude-3-5-haiku | Chat, Streaming |
| Azure OpenAI | Deployment-based | Chat, Streaming |
| AWS Bedrock | Claude models, Titan models | Chat |

## Credential Providers

Credentials can be fetched from multiple sources with caching:

```rust
// Environment variables
let provider = EnvCredentialProvider::new(CredentialType::OpenAi, "OPENAI_API_KEY");

// AWS Secrets Manager
let provider = AwsSecretsCredentialProvider::new(config, "secret-name", CredentialType::OpenAi);

// HashiCorp Vault
let provider = VaultCredentialProvider::new("https://vault:8200", "token", "secret/path", CredentialType::OpenAi);

// Chained providers (fallback chain)
let factory = CredentialProviderFactory::builder()
    .with_env()
    .with_aws_secrets()
    .build();
```

## Cache Layer

Pluggable caching with support for in-memory (moka) and Redis backends:

```rust
use pmp_llm_gateway::infrastructure::cache::{
    CacheFactory, CacheConfig, InMemoryCache, RedisCache
};
use pmp_llm_gateway::domain::cache::CacheExt;

// In-memory cache (default)
let cache = CacheFactory::new().create_in_memory();

// Redis cache
let cache = CacheFactory::new().create_redis("redis://localhost:6379").await?;

// Dynamic cache selection via config
let config = CacheConfig::redis("redis://localhost:6379")
    .with_key_prefix("myapp")
    .with_default_ttl(Duration::from_secs(3600));
let cache = CacheFactory::new().create(&config).await?;

// Using the cache (typed get/set via CacheExt trait)
cache.set("key", &my_value, Duration::from_secs(60)).await?;
let value: Option<MyType> = cache.get("key").await?;
```

### LLM Response Caching

Automatic caching of LLM responses to reduce API calls and costs:

```rust
use pmp_llm_gateway::infrastructure::services::{LlmCacheService, LlmCacheConfig};

let config = LlmCacheConfig::default()
    .with_namespace("llm:responses")
    .with_default_ttl(Duration::from_secs(3600));

let cache_service = LlmCacheService::with_config(cache, config);

// Cache a response
cache_service.set("gpt-4", &request, response).await?;

// Retrieve from cache
if let Some(cached) = cache_service.get("gpt-4", &request).await? {
    println!("Cache hit! Response: {:?}", cached.response);
}

// Invalidate by model
cache_service.invalidate_model("gpt-4").await?;
```

## Storage Layer

Pluggable storage with support for multiple backends:

```rust
use pmp_llm_gateway::infrastructure::storage::{
    StorageFactory, StorageConfig, PostgresConfig, InMemoryStorage
};

// In-memory storage (for testing/development)
let storage = StorageFactory::create_in_memory::<MyEntity>();

// PostgreSQL with connection pooling
let config = PostgresConfig::new("postgres://localhost/mydb")
    .with_max_connections(20)
    .with_min_connections(5);
let storage = StorageFactory::create_postgres::<MyEntity>(&config, "my_table").await?;

// Dynamic storage selection via config
let storage_config = StorageConfig::postgres_url("postgres://localhost/mydb");
let storage = StorageFactory::create::<MyEntity>(&storage_config, "my_table").await?;
```

### Storage Traits

Entities must implement `StorageEntity` and their keys must implement `StorageKey`:

```rust
impl StorageKey for MyId {
    fn as_str(&self) -> &str { &self.0 }
}

impl StorageEntity for MyEntity {
    type Key = MyId;
    fn key(&self) -> &Self::Key { &self.id }
}
```

### Migrations

Database migrations are handled automatically:

```rust
use pmp_llm_gateway::infrastructure::storage::run_storage_migrations;

// Run all pending migrations
run_storage_migrations(&pool).await?;
```

## Knowledge Bases

Vector-based knowledge base support for RAG (Retrieval-Augmented Generation):

```rust
use pmp_llm_gateway::domain::knowledge_base::{
    KnowledgeBaseId, FilterBuilder, SearchParams
};
use pmp_llm_gateway::infrastructure::knowledge_base::{
    PgvectorKnowledgeBase, PgvectorConfig, KnowledgeBaseFactory
};

// Create a pgvector knowledge base
let kb_id = KnowledgeBaseId::new("product-docs")?;
let config = PgvectorConfig::new(1536)  // OpenAI embedding dimensions
    .with_table_name("kb_vectors")
    .with_distance_metric(DistanceMetric::Cosine);

let kb = PgvectorKnowledgeBase::new(kb_id, pool, config, embedding_provider);
kb.ensure_table().await?;

// Add documents
let doc = Document::new("doc-1", "Product manual content...")
    .with_metadata("category", serde_json::json!("manual"))
    .with_source("manual.pdf");
kb.add_documents(vec![doc]).await?;

// Search with metadata filtering
let filter = FilterBuilder::new()
    .eq("category", "manual")
    .gte("version", 2i64)
    .build();

let params = SearchParams::new("How do I reset the device?")
    .with_top_k(5)
    .with_similarity_threshold(0.7)
    .with_filter(filter.unwrap());

let results = kb.search(params).await?;
```

### Metadata Filtering

Powerful filtering with AND/OR combinations:

```rust
use pmp_llm_gateway::domain::knowledge_base::{FilterBuilder, MetadataFilter};

// Simple filter
let filter = FilterBuilder::new()
    .eq("status", "published")
    .gt("score", 0.5f64)
    .build();

// OR filter
let filter = FilterBuilder::or()
    .eq("category", "faq")
    .eq("category", "manual")
    .build();

// Nested groups: (category = "docs" AND version > 1) OR (type = "faq")
let docs_filter = FilterBuilder::new()
    .eq("category", "docs")
    .gt("version", 1i64)
    .build()
    .unwrap();

let filter = FilterBuilder::or()
    .group(docs_filter)
    .eq("type", "faq")
    .build();
```

### Supported Providers

| Provider | Search | Add Documents | Delete | Notes |
|----------|--------|---------------|--------|-------|
| Pgvector | Yes | Yes | Yes | Full CRUD, requires EmbeddingProvider |
| AWS Knowledge Base | Yes | No | No | Read-only, documents via S3 |

## Workflows

Multi-step workflows that chain operations together with variable references between steps.

### Variable Reference Syntax

- `${request:field}` - Reference to workflow execution request input
- `${request:field:default}` - With default value
- `${step:step-name:field}` - Reference to previous step output
- `${step:step-name:field:default}` - With default value

### Step Types

| Type | Description |
|------|-------------|
| `chat_completion` | Execute LLM chat completion |
| `knowledge_base_search` | Search a knowledge base |
| `crag_scoring` | Score documents for relevance |
| `conditional` | Branch based on conditions |

### Example Workflow

```bash
# Create a workflow
curl -X POST http://localhost:8080/admin/workflows \
  -H "Authorization: Bearer sk-admin-key" \
  -H "Content-Type: application/json" \
  -d '{
    "id": "answer-with-context",
    "name": "Answer with Context",
    "steps": [
      {
        "name": "search",
        "type": "knowledge_base_search",
        "knowledge_base_id": "product-docs",
        "query": "${request:question}",
        "top_k": 10
      },
      {
        "name": "answer",
        "type": "chat_completion",
        "model_id": "gpt-4",
        "user_message": "Question: ${request:question}\n\nContext:\n${step:search:documents}"
      }
    ]
  }'

# Execute the workflow
curl -X POST http://localhost:8080/v1/workflows/answer-with-context/execute \
  -H "Authorization: Bearer sk-your-api-key" \
  -H "Content-Type: application/json" \
  -d '{"input": {"question": "How do I reset my password?"}}'
```

### Conditional Steps

```json
{
  "name": "check-results",
  "type": "conditional",
  "conditions": [
    {
      "field": "${step:search:documents}",
      "operator": "is_empty",
      "action": {"end_workflow": {"answer": "No results found"}}
    }
  ],
  "default_action": "continue"
}
```

### Condition Operators

`eq`, `ne`, `gt`, `gte`, `lt`, `lte`, `is_empty`, `is_not_empty`, `contains`

## Admin UI

The gateway includes an embedded web UI for administration, accessible at `/ui/` when running with `serve` or `ui` commands.

### Features

- **Dashboard**: Overview of models, prompts, API keys, and workflows
- **Models**: Create, edit, delete model configurations
- **Prompts**: Manage prompts with variable templating and live preview
- **API Keys**: Create, suspend, activate, revoke API keys with granular permissions
- **Workflows**: Visual editor for multi-step workflows
- **Credentials**: View available credential providers

### Authentication

The UI requires an admin API key for authentication. When you first access the UI, you'll be prompted to enter your API key. The key is stored in session storage and cleared when you close the browser.

To create an admin API key, set the `ADMIN_API_KEY` environment variable before starting the server:

```bash
ADMIN_API_KEY=your-secret-key cargo run serve
```

### Screenshots

The UI uses Tailwind CSS for styling and jQuery for interactivity. It's a single-page application that communicates with the Admin API endpoints.

## License

MIT
