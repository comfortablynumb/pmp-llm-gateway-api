-- migrate:up
-- Development seed data for local development environment
-- NOTE: This migration is only intended for development, not production

-- ============================================================================
-- CREDENTIALS
-- ============================================================================

-- OpenAI Default Credential (placeholder - set OPENAI_API_KEY env var)
INSERT INTO credentials (key, data) VALUES
('openai-default', '{
    "id": "openai-default",
    "name": "OpenAI Default",
    "credential_type": "open_ai",
    "api_key": "sk-placeholder-set-OPENAI_API_KEY-env-var",
    "enabled": true,
    "created_at": "2026-01-05T00:00:00Z",
    "updated_at": "2026-01-05T00:00:00Z"
}')
ON CONFLICT (key) DO NOTHING;

-- Anthropic Default Credential (placeholder - set ANTHROPIC_API_KEY env var)
INSERT INTO credentials (key, data) VALUES
('anthropic-default', '{
    "id": "anthropic-default",
    "name": "Anthropic Default",
    "credential_type": "anthropic",
    "api_key": "sk-placeholder-set-ANTHROPIC_API_KEY-env-var",
    "enabled": true,
    "created_at": "2026-01-05T00:00:00Z",
    "updated_at": "2026-01-05T00:00:00Z"
}')
ON CONFLICT (key) DO NOTHING;

-- PgVector Default Credential (for knowledge base)
INSERT INTO credentials (key, data) VALUES
('pgvector-default', '{
    "id": "pgvector-default",
    "name": "PgVector Default",
    "credential_type": "pgvector",
    "api_key": "postgres://gateway:gateway_dev@localhost:5432/llm_gateway?sslmode=disable",
    "enabled": true,
    "created_at": "2026-01-05T00:00:00Z",
    "updated_at": "2026-01-05T00:00:00Z"
}')
ON CONFLICT (key) DO NOTHING;

-- ============================================================================
-- MODELS
-- ============================================================================

-- GPT-4
INSERT INTO models (key, data) VALUES
('gpt-4', '{
    "id": "gpt-4",
    "name": "GPT-4",
    "provider": "open_ai",
    "provider_model": "gpt-4",
    "credential_id": "openai-default",
    "config": {},
    "version": 1,
    "enabled": true,
    "created_at": "2026-01-05T00:00:00Z",
    "updated_at": "2026-01-05T00:00:00Z"
}')
ON CONFLICT (key) DO NOTHING;

-- GPT-4 Turbo
INSERT INTO models (key, data) VALUES
('gpt-4-turbo', '{
    "id": "gpt-4-turbo",
    "name": "GPT-4 Turbo",
    "provider": "open_ai",
    "provider_model": "gpt-4-turbo",
    "credential_id": "openai-default",
    "config": {},
    "version": 1,
    "enabled": true,
    "created_at": "2026-01-05T00:00:00Z",
    "updated_at": "2026-01-05T00:00:00Z"
}')
ON CONFLICT (key) DO NOTHING;

-- GPT-3.5 Turbo
INSERT INTO models (key, data) VALUES
('gpt-35-turbo', '{
    "id": "gpt-35-turbo",
    "name": "GPT-3.5 Turbo",
    "provider": "open_ai",
    "provider_model": "gpt-3.5-turbo",
    "credential_id": "openai-default",
    "config": {},
    "version": 1,
    "enabled": true,
    "created_at": "2026-01-05T00:00:00Z",
    "updated_at": "2026-01-05T00:00:00Z"
}')
ON CONFLICT (key) DO NOTHING;

-- Claude 3 Opus
INSERT INTO models (key, data) VALUES
('claude-3-opus', '{
    "id": "claude-3-opus",
    "name": "Claude 3 Opus",
    "provider": "anthropic",
    "provider_model": "claude-3-opus-20240229",
    "credential_id": "anthropic-default",
    "config": {},
    "version": 1,
    "enabled": true,
    "created_at": "2026-01-05T00:00:00Z",
    "updated_at": "2026-01-05T00:00:00Z"
}')
ON CONFLICT (key) DO NOTHING;

-- Claude 3 Sonnet
INSERT INTO models (key, data) VALUES
('claude-3-sonnet', '{
    "id": "claude-3-sonnet",
    "name": "Claude 3 Sonnet",
    "provider": "anthropic",
    "provider_model": "claude-3-sonnet-20240229",
    "credential_id": "anthropic-default",
    "config": {},
    "version": 1,
    "enabled": true,
    "created_at": "2026-01-05T00:00:00Z",
    "updated_at": "2026-01-05T00:00:00Z"
}')
ON CONFLICT (key) DO NOTHING;

-- Text Embedding Ada 002 (for knowledge bases)
INSERT INTO models (key, data) VALUES
('text-embedding-ada-002', '{
    "id": "text-embedding-ada-002",
    "name": "Text Embedding Ada 002",
    "provider": "open_ai",
    "provider_model": "text-embedding-ada-002",
    "credential_id": "openai-default",
    "config": {},
    "version": 1,
    "enabled": true,
    "created_at": "2026-01-05T00:00:00Z",
    "updated_at": "2026-01-05T00:00:00Z"
}')
ON CONFLICT (key) DO NOTHING;

-- ============================================================================
-- PROMPTS
-- ============================================================================

-- System Assistant
INSERT INTO prompts (key, data) VALUES
('system-assistant', '{
    "id": "system-assistant",
    "name": "System Assistant",
    "content": "You are a helpful assistant.",
    "version": 1,
    "max_history": 10,
    "enabled": true,
    "tags": ["system", "default"],
    "created_at": "2026-01-05T00:00:00Z",
    "updated_at": "2026-01-05T00:00:00Z"
}')
ON CONFLICT (key) DO NOTHING;

-- Code Reviewer
INSERT INTO prompts (key, data) VALUES
('code-reviewer', '{
    "id": "code-reviewer",
    "name": "Code Reviewer",
    "content": "You are an expert code reviewer. Analyze the code and provide constructive feedback.",
    "version": 1,
    "max_history": 10,
    "enabled": true,
    "tags": ["code", "review"],
    "created_at": "2026-01-05T00:00:00Z",
    "updated_at": "2026-01-05T00:00:00Z"
}')
ON CONFLICT (key) DO NOTHING;

-- Summarizer
INSERT INTO prompts (key, data) VALUES
('summarizer', '{
    "id": "summarizer",
    "name": "Summarizer",
    "content": "Summarize the following text concisely while preserving key information.",
    "version": 1,
    "max_history": 10,
    "enabled": true,
    "tags": ["summary"],
    "created_at": "2026-01-05T00:00:00Z",
    "updated_at": "2026-01-05T00:00:00Z"
}')
ON CONFLICT (key) DO NOTHING;

-- Translator
INSERT INTO prompts (key, data) VALUES
('translator', '{
    "id": "translator",
    "name": "Translator",
    "content": "Translate the following text to ${var:target_language:English}.",
    "version": 1,
    "max_history": 10,
    "enabled": true,
    "tags": ["translation"],
    "created_at": "2026-01-05T00:00:00Z",
    "updated_at": "2026-01-05T00:00:00Z"
}')
ON CONFLICT (key) DO NOTHING;

-- CRAG Relevance Scorer
INSERT INTO prompts (key, data) VALUES
('crag-relevance-scorer', '{
    "id": "crag-relevance-scorer",
    "name": "CRAG Relevance Scorer",
    "content": "Rate the relevance of the following document to the query on a scale of 0-10. Query: ${var:query}\n\nDocument: ${var:document}\n\nRespond with only a number.",
    "version": 1,
    "max_history": 10,
    "enabled": true,
    "tags": ["crag", "scoring"],
    "created_at": "2026-01-05T00:00:00Z",
    "updated_at": "2026-01-05T00:00:00Z"
}')
ON CONFLICT (key) DO NOTHING;

-- CRAG Knowledge Refiner
INSERT INTO prompts (key, data) VALUES
('crag-knowledge-refiner', '{
    "id": "crag-knowledge-refiner",
    "name": "CRAG Knowledge Refiner",
    "content": "Extract and refine the most relevant information from the following documents to answer the query.\n\nQuery: ${var:query}\n\nDocuments:\n${var:documents}",
    "version": 1,
    "max_history": 10,
    "enabled": true,
    "tags": ["crag", "refinement"],
    "created_at": "2026-01-05T00:00:00Z",
    "updated_at": "2026-01-05T00:00:00Z"
}')
ON CONFLICT (key) DO NOTHING;

-- CRAG Web Search Generator
INSERT INTO prompts (key, data) VALUES
('crag-web-search-generator', '{
    "id": "crag-web-search-generator",
    "name": "CRAG Web Search Query Generator",
    "content": "Generate a concise web search query to find information about: ${var:query}",
    "version": 1,
    "max_history": 10,
    "enabled": true,
    "tags": ["crag", "search"],
    "created_at": "2026-01-05T00:00:00Z",
    "updated_at": "2026-01-05T00:00:00Z"
}')
ON CONFLICT (key) DO NOTHING;

-- CRAG Final Answer
INSERT INTO prompts (key, data) VALUES
('crag-final-answer', '{
    "id": "crag-final-answer",
    "name": "CRAG Final Answer Generator",
    "content": "Using the following knowledge, provide a comprehensive answer to the query.\n\nQuery: ${var:query}\n\nKnowledge:\n${var:knowledge}",
    "version": 1,
    "max_history": 10,
    "enabled": true,
    "tags": ["crag", "answer"],
    "created_at": "2026-01-05T00:00:00Z",
    "updated_at": "2026-01-05T00:00:00Z"
}')
ON CONFLICT (key) DO NOTHING;

-- RAG System Prompt
INSERT INTO prompts (key, data) VALUES
('rag-system', '{
    "id": "rag-system",
    "name": "RAG System Prompt",
    "content": "You are a helpful assistant. Use the following context to answer questions.\n\nContext:\n${var:context}\n\nIf the context doesn''t contain relevant information, say so and provide what help you can.",
    "version": 1,
    "max_history": 10,
    "enabled": true,
    "tags": ["rag", "system"],
    "created_at": "2026-01-05T00:00:00Z",
    "updated_at": "2026-01-05T00:00:00Z"
}')
ON CONFLICT (key) DO NOTHING;

-- Document Analyzer
INSERT INTO prompts (key, data) VALUES
('document-analyzer', '{
    "id": "document-analyzer",
    "name": "Document Analyzer",
    "content": "Analyze the following document and extract key information:\n${var:document}",
    "version": 1,
    "max_history": 10,
    "enabled": true,
    "tags": ["document", "analysis"],
    "created_at": "2026-01-05T00:00:00Z",
    "updated_at": "2026-01-05T00:00:00Z"
}')
ON CONFLICT (key) DO NOTHING;

-- Sentiment Analyzer
INSERT INTO prompts (key, data) VALUES
('sentiment-analyzer', '{
    "id": "sentiment-analyzer",
    "name": "Sentiment Analyzer",
    "content": "Analyze the sentiment of the following text. Respond with: positive, negative, or neutral.\n\nText: ${var:text}",
    "version": 1,
    "max_history": 10,
    "enabled": true,
    "tags": ["sentiment", "analysis"],
    "created_at": "2026-01-05T00:00:00Z",
    "updated_at": "2026-01-05T00:00:00Z"
}')
ON CONFLICT (key) DO NOTHING;

-- Entity Extractor
INSERT INTO prompts (key, data) VALUES
('entity-extractor', '{
    "id": "entity-extractor",
    "name": "Entity Extractor",
    "content": "Extract named entities (people, organizations, locations, dates) from the following text. Return as JSON.\n\nText: ${var:text}",
    "version": 1,
    "max_history": 10,
    "enabled": true,
    "tags": ["entities", "extraction"],
    "created_at": "2026-01-05T00:00:00Z",
    "updated_at": "2026-01-05T00:00:00Z"
}')
ON CONFLICT (key) DO NOTHING;

-- Q&A Generator
INSERT INTO prompts (key, data) VALUES
('qa-generator', '{
    "id": "qa-generator",
    "name": "Q&A Generator",
    "content": "Generate questions and answers based on the following content:\n${var:content}",
    "version": 1,
    "max_history": 10,
    "enabled": true,
    "tags": ["qa", "generation"],
    "created_at": "2026-01-05T00:00:00Z",
    "updated_at": "2026-01-05T00:00:00Z"
}')
ON CONFLICT (key) DO NOTHING;

-- Classification Prompt
INSERT INTO prompts (key, data) VALUES
('classification-prompt', '{
    "id": "classification-prompt",
    "name": "Classification Prompt",
    "content": "Classify the following text into one of these categories: ${var:categories}\n\nText: ${var:text}\n\nRespond with only the category name.",
    "version": 1,
    "max_history": 10,
    "enabled": true,
    "tags": ["classification"],
    "created_at": "2026-01-05T00:00:00Z",
    "updated_at": "2026-01-05T00:00:00Z"
}')
ON CONFLICT (key) DO NOTHING;

-- Chain of Thought
INSERT INTO prompts (key, data) VALUES
('chain-of-thought', '{
    "id": "chain-of-thought",
    "name": "Chain of Thought",
    "content": "Think through this problem step by step:\n${var:problem}\n\nShow your reasoning before giving the final answer.",
    "version": 1,
    "max_history": 10,
    "enabled": true,
    "tags": ["reasoning", "cot"],
    "created_at": "2026-01-05T00:00:00Z",
    "updated_at": "2026-01-05T00:00:00Z"
}')
ON CONFLICT (key) DO NOTHING;

-- Few-Shot Template
INSERT INTO prompts (key, data) VALUES
('few-shot-template', '{
    "id": "few-shot-template",
    "name": "Few-Shot Template",
    "content": "Here are some examples:\n${var:examples}\n\nNow, following the same pattern:\n${var:input}",
    "version": 1,
    "max_history": 10,
    "enabled": true,
    "tags": ["few-shot", "template"],
    "created_at": "2026-01-05T00:00:00Z",
    "updated_at": "2026-01-05T00:00:00Z"
}')
ON CONFLICT (key) DO NOTHING;

-- JSON Output
INSERT INTO prompts (key, data) VALUES
('json-output', '{
    "id": "json-output",
    "name": "JSON Output",
    "content": "Based on the input, generate a JSON response with the following schema:\n${var:schema}\n\nInput: ${var:input}\n\nRespond with valid JSON only.",
    "version": 1,
    "max_history": 10,
    "enabled": true,
    "tags": ["json", "structured"],
    "created_at": "2026-01-05T00:00:00Z",
    "updated_at": "2026-01-05T00:00:00Z"
}')
ON CONFLICT (key) DO NOTHING;

-- ============================================================================
-- KNOWLEDGE BASES
-- ============================================================================

-- Default Knowledge Base (PgVector)
INSERT INTO knowledge_bases (key, data) VALUES
('default-kb', '{
    "id": "default-kb",
    "name": "Default Knowledge Base",
    "description": "Default knowledge base for development using PgVector",
    "kb_type": "pgvector",
    "embedding": {
        "model": "text-embedding-ada-002",
        "dimensions": 1536
    },
    "config": {
        "default_top_k": 10,
        "default_similarity_threshold": 0.7,
        "include_embeddings": false,
        "include_metadata": true
    },
    "connection_config": {
        "credential_id": "pgvector-default",
        "embedding_model_id": "text-embedding-ada-002"
    },
    "enabled": true,
    "created_at": "2026-01-05T00:00:00Z",
    "updated_at": "2026-01-05T00:00:00Z"
}')
ON CONFLICT (key) DO NOTHING;

-- ============================================================================
-- WORKFLOWS
-- ============================================================================

-- Basic RAG Workflow
INSERT INTO workflows (key, data) VALUES
('basic-rag', '{
    "id": "basic-rag",
    "name": "Basic RAG",
    "description": "Simple retrieval-augmented generation workflow",
    "input_schema": {
        "type": "object",
        "properties": {
            "query": {
                "type": "string",
                "description": "The search query or question"
            }
        },
        "required": ["query"]
    },
    "steps": [
        {
            "name": "search",
            "type": "knowledge_base_search",
            "knowledge_base_id": "default-kb",
            "query": "${request:query}",
            "top_k": 5
        },
        {
            "name": "generate",
            "type": "chat_completion",
            "model_id": "gpt-4",
            "prompt_id": "rag-system",
            "user_message": "${request:query}",
            "temperature": 0.7,
            "max_tokens": 1000
        }
    ],
    "version": 1,
    "enabled": true,
    "created_at": "2026-01-05T00:00:00Z",
    "updated_at": "2026-01-05T00:00:00Z"
}')
ON CONFLICT (key) DO NOTHING;

-- CRAG Pipeline
INSERT INTO workflows (key, data) VALUES
('crag-pipeline', '{
    "id": "crag-pipeline",
    "name": "CRAG Pipeline",
    "description": "Corrective RAG with document scoring and refinement",
    "input_schema": {
        "type": "object",
        "properties": {
            "query": {
                "type": "string",
                "description": "The search query or question"
            }
        },
        "required": ["query"]
    },
    "steps": [
        {
            "name": "search",
            "type": "knowledge_base_search",
            "knowledge_base_id": "default-kb",
            "query": "${request:query}",
            "top_k": 10
        },
        {
            "name": "score",
            "type": "crag_scoring",
            "input_documents": "${step:search:documents}",
            "query": "${request:query}",
            "model_id": "gpt-4",
            "prompt_id": "crag-relevance-scorer",
            "threshold": 0.7,
            "strategy": "hybrid"
        },
        {
            "name": "answer",
            "type": "chat_completion",
            "model_id": "gpt-4",
            "prompt_id": "crag-final-answer",
            "user_message": "${request:query}",
            "temperature": 0.7,
            "max_tokens": 1500
        }
    ],
    "version": 1,
    "enabled": true,
    "created_at": "2026-01-05T00:00:00Z",
    "updated_at": "2026-01-05T00:00:00Z"
}')
ON CONFLICT (key) DO NOTHING;

-- Sentiment Analysis Workflow
INSERT INTO workflows (key, data) VALUES
('sentiment-analysis', '{
    "id": "sentiment-analysis",
    "name": "Sentiment Analysis",
    "description": "Analyzes sentiment of input text",
    "input_schema": {
        "type": "object",
        "properties": {
            "text": {
                "type": "string",
                "description": "The input text to analyze"
            }
        },
        "required": ["text"]
    },
    "steps": [
        {
            "name": "analyze",
            "type": "chat_completion",
            "model_id": "gpt-35-turbo",
            "prompt_id": "sentiment-analyzer",
            "user_message": "${request:text}",
            "temperature": 0.1,
            "max_tokens": 50
        }
    ],
    "version": 1,
    "enabled": true,
    "created_at": "2026-01-05T00:00:00Z",
    "updated_at": "2026-01-05T00:00:00Z"
}')
ON CONFLICT (key) DO NOTHING;

-- Entity Extraction Workflow
INSERT INTO workflows (key, data) VALUES
('entity-extraction', '{
    "id": "entity-extraction",
    "name": "Entity Extraction",
    "description": "Extracts named entities from text",
    "input_schema": {
        "type": "object",
        "properties": {
            "text": {
                "type": "string",
                "description": "The input text to extract entities from"
            }
        },
        "required": ["text"]
    },
    "steps": [
        {
            "name": "extract",
            "type": "chat_completion",
            "model_id": "gpt-4",
            "prompt_id": "entity-extractor",
            "user_message": "${request:text}",
            "temperature": 0.1,
            "max_tokens": 1000
        }
    ],
    "version": 1,
    "enabled": true,
    "created_at": "2026-01-05T00:00:00Z",
    "updated_at": "2026-01-05T00:00:00Z"
}')
ON CONFLICT (key) DO NOTHING;

-- Document Analysis Workflow
INSERT INTO workflows (key, data) VALUES
('document-analysis', '{
    "id": "document-analysis",
    "name": "Document Analysis",
    "description": "Multi-step document analysis with summary and Q&A generation",
    "input_schema": {
        "type": "object",
        "properties": {
            "document": {
                "type": "string",
                "description": "The document content to analyze"
            }
        },
        "required": ["document"]
    },
    "steps": [
        {
            "name": "summarize",
            "type": "chat_completion",
            "model_id": "gpt-4",
            "prompt_id": "summarizer",
            "user_message": "${request:document}",
            "temperature": 0.5,
            "max_tokens": 500
        },
        {
            "name": "extract_entities",
            "type": "chat_completion",
            "model_id": "gpt-4",
            "prompt_id": "entity-extractor",
            "user_message": "${request:document}",
            "temperature": 0.1,
            "max_tokens": 1000
        },
        {
            "name": "generate_qa",
            "type": "chat_completion",
            "model_id": "gpt-4",
            "prompt_id": "qa-generator",
            "user_message": "${step:summarize:content}",
            "temperature": 0.7,
            "max_tokens": 1500
        }
    ],
    "version": 1,
    "enabled": true,
    "created_at": "2026-01-05T00:00:00Z",
    "updated_at": "2026-01-05T00:00:00Z"
}')
ON CONFLICT (key) DO NOTHING;

-- Translation Workflow
INSERT INTO workflows (key, data) VALUES
('translate', '{
    "id": "translate",
    "name": "Translate",
    "description": "Translates text to target language",
    "input_schema": {
        "type": "object",
        "properties": {
            "text": {
                "type": "string",
                "description": "The text to translate"
            },
            "target_language": {
                "type": "string",
                "description": "Target language for translation (e.g., Spanish, French)",
                "default": "English"
            }
        },
        "required": ["text"]
    },
    "steps": [
        {
            "name": "translate",
            "type": "chat_completion",
            "model_id": "gpt-4",
            "prompt_id": "translator",
            "user_message": "${request:text}",
            "temperature": 0.3,
            "max_tokens": 2000
        }
    ],
    "version": 1,
    "enabled": true,
    "created_at": "2026-01-05T00:00:00Z",
    "updated_at": "2026-01-05T00:00:00Z"
}')
ON CONFLICT (key) DO NOTHING;

-- ============================================================================
-- BUDGETS
-- ============================================================================

-- Development Budget (Daily)
INSERT INTO budgets (key, data) VALUES
('dev-daily', '{
    "id": "dev-daily",
    "name": "Development Daily Budget",
    "description": "Daily budget for development environment",
    "period": "daily",
    "hard_limit_micros": 10000000,
    "soft_limit_micros": 8000000,
    "current_usage_micros": 0,
    "status": "active",
    "scope": "all_api_keys",
    "api_key_ids": [],
    "team_ids": [],
    "model_ids": [],
    "alerts": [
        {"threshold_percent": 50, "triggered": false},
        {"threshold_percent": 75, "triggered": false},
        {"threshold_percent": 90, "triggered": false}
    ],
    "period_start": 1704412800,
    "created_at": 1704412800,
    "updated_at": 1704412800,
    "enabled": true
}')
ON CONFLICT (key) DO NOTHING;

-- Development Budget (Monthly)
INSERT INTO budgets (key, data) VALUES
('dev-monthly', '{
    "id": "dev-monthly",
    "name": "Development Monthly Budget",
    "description": "Monthly budget for development environment",
    "period": "monthly",
    "hard_limit_micros": 100000000,
    "soft_limit_micros": 80000000,
    "current_usage_micros": 0,
    "status": "active",
    "scope": "all_api_keys",
    "api_key_ids": [],
    "team_ids": [],
    "model_ids": [],
    "alerts": [
        {"threshold_percent": 50, "triggered": false},
        {"threshold_percent": 75, "triggered": false},
        {"threshold_percent": 90, "triggered": false}
    ],
    "period_start": 1704412800,
    "created_at": 1704412800,
    "updated_at": 1704412800,
    "enabled": true
}')
ON CONFLICT (key) DO NOTHING;

-- GPT-4 Specific Budget
INSERT INTO budgets (key, data) VALUES
('gpt4-budget', '{
    "id": "gpt4-budget",
    "name": "GPT-4 Model Budget",
    "description": "Budget specifically for GPT-4 model usage",
    "period": "monthly",
    "hard_limit_micros": 50000000,
    "soft_limit_micros": 40000000,
    "current_usage_micros": 0,
    "status": "active",
    "scope": "all_api_keys",
    "api_key_ids": [],
    "team_ids": [],
    "model_ids": ["gpt-4", "gpt-4-turbo"],
    "alerts": [
        {"threshold_percent": 75, "triggered": false},
        {"threshold_percent": 90, "triggered": false}
    ],
    "period_start": 1704412800,
    "created_at": 1704412800,
    "updated_at": 1704412800,
    "enabled": true
}')
ON CONFLICT (key) DO NOTHING;

-- migrate:down

-- DON'T EVER INCLUDE DOWN MIGRATIONS!
