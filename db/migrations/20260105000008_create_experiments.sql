-- migrate:up

CREATE TABLE experiments (
    key VARCHAR(255) PRIMARY KEY,
    data JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_experiments_created_at ON experiments(created_at);
CREATE INDEX idx_experiments_status ON experiments((data->>'status'));

-- migrate:down

-- DON'T EVER INCLUDE DOWN MIGRATIONS!
