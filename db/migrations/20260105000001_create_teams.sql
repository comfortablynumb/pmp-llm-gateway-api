-- migrate:up

CREATE TABLE teams (
    key VARCHAR(255) PRIMARY KEY,
    data JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_teams_created_at ON teams(created_at);

-- migrate:down

-- DON'T EVER INCLUDE DOWN MIGRATIONS!
