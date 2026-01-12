-- migrate:up

CREATE TABLE app_configurations (
    key VARCHAR(255) PRIMARY KEY,
    data JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_app_configurations_created_at ON app_configurations(created_at);

-- migrate:down

-- DON'T EVER INCLUDE DOWN MIGRATIONS!
