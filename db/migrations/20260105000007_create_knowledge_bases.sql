-- migrate:up

CREATE TABLE knowledge_bases (
    key VARCHAR(255) PRIMARY KEY,
    data JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_knowledge_bases_created_at ON knowledge_bases(created_at);
CREATE INDEX idx_knowledge_bases_type ON knowledge_bases((data->>'kb_type'));

-- migrate:down

-- DON'T EVER INCLUDE DOWN MIGRATIONS!
