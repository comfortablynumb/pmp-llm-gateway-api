-- migrate:up

CREATE TABLE app_configurations (
    key VARCHAR(255) PRIMARY KEY,
    value JSONB NOT NULL,
    metadata JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_app_configurations_created_at ON app_configurations(created_at);

-- Seed default configuration values
-- Persistence settings
INSERT INTO app_configurations (key, value, metadata) VALUES
('persistence.enabled', '{"type": "boolean", "value": true}', '{"category": "persistence", "description": "Enable execution logging", "value_type": "boolean"}'),
('persistence.enabled_models', '{"type": "string_list", "value": []}', '{"category": "persistence", "description": "List of model IDs to log executions for (empty = all if enabled)", "value_type": "string_list"}'),
('persistence.enabled_workflows', '{"type": "string_list", "value": []}', '{"category": "persistence", "description": "List of workflow IDs to log executions for (empty = all if enabled)", "value_type": "string_list"}'),
('persistence.log_retention_days', '{"type": "integer", "value": 30}', '{"category": "persistence", "description": "Number of days to retain execution logs", "value_type": "integer"}'),
('persistence.log_sensitive_data', '{"type": "boolean", "value": true}', '{"category": "persistence", "description": "Whether to log full input/output (may contain sensitive data)", "value_type": "boolean"}'),
-- Logging settings
('logging.level', '{"type": "string", "value": "info"}', '{"category": "logging", "description": "Log level (trace, debug, info, warn, error)", "value_type": "string"}'),
('logging.format', '{"type": "string", "value": "json"}', '{"category": "logging", "description": "Log format (json, pretty)", "value_type": "string"}'),
-- Cache settings
('cache.enabled', '{"type": "boolean", "value": true}', '{"category": "cache", "description": "Enable response caching", "value_type": "boolean"}'),
('cache.ttl_seconds', '{"type": "integer", "value": 3600}', '{"category": "cache", "description": "Cache TTL in seconds", "value_type": "integer"}'),
('cache.max_entries', '{"type": "integer", "value": 10000}', '{"category": "cache", "description": "Maximum cache entries", "value_type": "integer"}'),
-- Security settings
('security.require_api_key', '{"type": "boolean", "value": true}', '{"category": "security", "description": "Require API key for all requests", "value_type": "boolean"}'),
('security.allowed_origins', '{"type": "string_list", "value": ["*"]}', '{"category": "security", "description": "Allowed CORS origins", "value_type": "string_list"}'),
-- Rate limit settings
('rate_limit.enabled', '{"type": "boolean", "value": true}', '{"category": "rate_limit", "description": "Enable rate limiting", "value_type": "boolean"}'),
('rate_limit.default_rpm', '{"type": "integer", "value": 60}', '{"category": "rate_limit", "description": "Default requests per minute", "value_type": "integer"}');

-- migrate:down

-- DON'T EVER INCLUDE DOWN MIGRATIONS!
