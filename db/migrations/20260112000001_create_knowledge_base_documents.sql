-- migrate:up

-- Table for storing documents (parent of chunks)
CREATE TABLE knowledge_base_documents (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    kb_id VARCHAR(50) NOT NULL,
    title VARCHAR(1000),
    description TEXT,
    source_filename VARCHAR(1000),
    content_type VARCHAR(100),
    original_size_bytes BIGINT,
    chunk_count INTEGER NOT NULL DEFAULT 0,
    metadata JSONB DEFAULT '{}',
    disabled BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_kb_documents_kb_id ON knowledge_base_documents(kb_id);
CREATE INDEX idx_kb_documents_title ON knowledge_base_documents(title);
CREATE INDEX idx_kb_documents_disabled ON knowledge_base_documents(disabled);
CREATE INDEX idx_kb_documents_created_at ON knowledge_base_documents(created_at);

-- Table for storing document chunks with embeddings
CREATE TABLE knowledge_base_document_chunks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    document_id UUID NOT NULL REFERENCES knowledge_base_documents(id) ON DELETE CASCADE,
    kb_id VARCHAR(50) NOT NULL,
    chunk_index INTEGER NOT NULL,
    content TEXT NOT NULL,
    embedding vector(1536) NOT NULL,
    token_count INTEGER,
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_kb_chunks_document_id ON knowledge_base_document_chunks(document_id);
CREATE INDEX idx_kb_chunks_kb_id ON knowledge_base_document_chunks(kb_id);
CREATE INDEX idx_kb_chunks_chunk_index ON knowledge_base_document_chunks(chunk_index);

-- IVFFlat index for vector similarity search (cosine distance)
CREATE INDEX idx_kb_chunks_embedding ON knowledge_base_document_chunks
    USING ivfflat (embedding vector_cosine_ops) WITH (lists = 100);

-- migrate:down

-- DON'T EVER INCLUDE DOWN MIGRATIONS!
