-- migrate:up

CREATE TABLE api_keys (
    key VARCHAR(255) PRIMARY KEY,
    data JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_api_keys_created_at ON api_keys(created_at);
CREATE INDEX idx_api_keys_team_id ON api_keys((data->>'team_id'));
CREATE INDEX idx_api_keys_status ON api_keys((data->>'status'));

-- migrate:down

-- DON'T EVER INCLUDE DOWN MIGRATIONS!
