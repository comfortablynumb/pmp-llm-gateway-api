-- migrate:up

CREATE TABLE external_apis (
    key VARCHAR(255) PRIMARY KEY,
    data JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_external_apis_created_at ON external_apis(created_at);
CREATE INDEX idx_external_apis_name ON external_apis((data->>'name'));

-- migrate:down

-- DON'T EVER INCLUDE DOWN MIGRATIONS!
