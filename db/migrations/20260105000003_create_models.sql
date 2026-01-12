-- migrate:up

CREATE TABLE models (
    key VARCHAR(255) PRIMARY KEY,
    data JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_models_created_at ON models(created_at);
CREATE INDEX idx_models_enabled ON models((data->>'enabled'));

-- migrate:down

-- DON'T EVER INCLUDE DOWN MIGRATIONS!
