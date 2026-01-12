-- migrate:up

CREATE TABLE test_cases (
    key VARCHAR(255) PRIMARY KEY,
    data JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_test_cases_created_at ON test_cases(created_at);
CREATE INDEX idx_test_cases_enabled ON test_cases((data->>'enabled'));

-- migrate:down

-- DON'T EVER INCLUDE DOWN MIGRATIONS!
