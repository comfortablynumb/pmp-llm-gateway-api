-- migrate:up

CREATE TABLE webhook_deliveries (
    key VARCHAR(255) PRIMARY KEY,
    data JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_webhook_deliveries_created_at ON webhook_deliveries(created_at);
CREATE INDEX idx_webhook_deliveries_webhook_id ON webhook_deliveries((data->'webhook_id'->>'0'));
CREATE INDEX idx_webhook_deliveries_status ON webhook_deliveries((data->>'status'));

-- migrate:down

-- DON'T EVER INCLUDE DOWN MIGRATIONS!
