-- migrate:up

CREATE TABLE credentials (
    key VARCHAR(255) PRIMARY KEY,
    data JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_credentials_created_at ON credentials(created_at);
CREATE INDEX idx_credentials_type ON credentials((data->>'credential_type'));

-- migrate:down

-- DON'T EVER INCLUDE DOWN MIGRATIONS!
