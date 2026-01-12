-- migrate:up

CREATE TABLE webhooks (
    key VARCHAR(255) PRIMARY KEY,
    data JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_webhooks_created_at ON webhooks(created_at);
CREATE INDEX idx_webhooks_status ON webhooks((data->>'status'));

-- migrate:down

-- DON'T EVER INCLUDE DOWN MIGRATIONS!
