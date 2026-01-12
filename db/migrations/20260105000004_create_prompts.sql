-- migrate:up

CREATE TABLE prompts (
    key VARCHAR(255) PRIMARY KEY,
    data JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_prompts_created_at ON prompts(created_at);
CREATE INDEX idx_prompts_enabled ON prompts((data->>'enabled'));

-- migrate:down

-- DON'T EVER INCLUDE DOWN MIGRATIONS!
