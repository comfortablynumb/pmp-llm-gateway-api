-- migrate:up

CREATE TABLE execution_logs (
    key VARCHAR(255) PRIMARY KEY,
    data JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_execution_logs_created_at ON execution_logs(created_at);
CREATE INDEX idx_execution_logs_type ON execution_logs((data->>'execution_type'));
CREATE INDEX idx_execution_logs_status ON execution_logs((data->>'status'));

-- migrate:down

-- DON'T EVER INCLUDE DOWN MIGRATIONS!
