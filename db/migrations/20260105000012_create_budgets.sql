-- migrate:up

CREATE TABLE budgets (
    key VARCHAR(255) PRIMARY KEY,
    data JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_budgets_created_at ON budgets(created_at);
CREATE INDEX idx_budgets_status ON budgets((data->>'status'));
CREATE INDEX idx_budgets_enabled ON budgets((data->>'enabled'));

-- migrate:down

-- DON'T EVER INCLUDE DOWN MIGRATIONS!
