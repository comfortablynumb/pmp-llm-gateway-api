-- migrate:up

CREATE TABLE test_case_results (
    key VARCHAR(255) PRIMARY KEY,
    data JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_test_case_results_created_at ON test_case_results(created_at);
CREATE INDEX idx_test_case_results_test_case_id ON test_case_results((data->>'test_case_id'));
CREATE INDEX idx_test_case_results_passed ON test_case_results((data->>'passed'));
CREATE INDEX idx_test_case_results_executed_at ON test_case_results((data->>'executed_at'));

-- migrate:down

-- DON'T EVER INCLUDE DOWN MIGRATIONS!
