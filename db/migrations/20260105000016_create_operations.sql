-- migrate:up

CREATE TABLE operations (
    key VARCHAR(255) PRIMARY KEY,
    data JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_operations_created_at ON operations(created_at);
CREATE INDEX idx_operations_status ON operations((data->>'status'));
CREATE INDEX idx_operations_type ON operations((data->>'operation_type'));

-- migrate:down

-- DON'T EVER INCLUDE DOWN MIGRATIONS!
