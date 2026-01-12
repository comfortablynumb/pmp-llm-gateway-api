-- migrate:up

CREATE TABLE experiment_records (
    key VARCHAR(255) PRIMARY KEY,
    data JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_experiment_records_created_at ON experiment_records(created_at);
CREATE INDEX idx_experiment_records_experiment_id ON experiment_records((data->>'experiment_id'));
CREATE INDEX idx_experiment_records_variant_id ON experiment_records((data->>'variant_id'));
CREATE INDEX idx_experiment_records_api_key_id ON experiment_records((data->>'api_key_id'));
CREATE INDEX idx_experiment_records_timestamp ON experiment_records((data->>'timestamp'));

-- migrate:down

-- DON'T EVER INCLUDE DOWN MIGRATIONS!
