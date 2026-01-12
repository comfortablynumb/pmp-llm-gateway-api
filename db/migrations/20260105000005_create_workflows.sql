-- migrate:up

CREATE TABLE workflows (
    key VARCHAR(255) PRIMARY KEY,
    data JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_workflows_created_at ON workflows(created_at);
CREATE INDEX idx_workflows_enabled ON workflows((data->>'enabled'));

-- migrate:down

-- DON'T EVER INCLUDE DOWN MIGRATIONS!
