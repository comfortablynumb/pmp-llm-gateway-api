-- migrate:up

-- Enable the pgvector extension for vector similarity search
CREATE EXTENSION IF NOT EXISTS vector;

-- migrate:down

-- DON'T EVER INCLUDE DOWN MIGRATIONS!
