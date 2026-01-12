-- migrate:up

CREATE TABLE usage_records (
    key VARCHAR(255) PRIMARY KEY,
    data JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_usage_records_created_at ON usage_records(created_at);
CREATE INDEX idx_usage_records_api_key_id ON usage_records((data->>'api_key_id'));
CREATE INDEX idx_usage_records_model_id ON usage_records((data->>'model_id'));
CREATE INDEX idx_usage_records_timestamp ON usage_records((data->>'timestamp'));

-- migrate:down

-- DON'T EVER INCLUDE DOWN MIGRATIONS!
