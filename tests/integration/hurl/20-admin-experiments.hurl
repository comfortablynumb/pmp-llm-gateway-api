# Admin Experiments Endpoints (A/B Testing)
# Test experiment management lifecycle

# List all experiments (initially empty or minimal)
GET {{app_url}}/admin/experiments
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.experiments" isCollection

# Create a new experiment
POST {{app_url}}/admin/experiments
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "id": "test-experiment",
    "name": "Test A/B Experiment",
    "description": "Testing different model configurations",
    "variants": [
        {
            "id": "control",
            "name": "Control (GPT-4)",
            "description": "Standard GPT-4 model",
            "config": {
                "type": "model_reference",
                "model_id": "gpt-4"
            },
            "control": true
        },
        {
            "id": "treatment",
            "name": "Treatment (GPT-4 Lower Temp)",
            "description": "GPT-4 with lower temperature",
            "config": {
                "type": "config_override",
                "model_id": "gpt-4",
                "temperature": 0.3,
                "max_tokens": 500
            },
            "control": false
        }
    ],
    "traffic_allocation": [
        {"variant_id": "control", "percentage": 50},
        {"variant_id": "treatment", "percentage": 50}
    ]
}
HTTP 200
[Asserts]
jsonpath "$.id" == "test-experiment"
jsonpath "$.name" == "Test A/B Experiment"
jsonpath "$.status" == "draft"
jsonpath "$.variants" count == 2
jsonpath "$.traffic_allocation" count == 2

# Get the created experiment
GET {{app_url}}/admin/experiments/test-experiment
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.id" == "test-experiment"
jsonpath "$.status" == "draft"

# Update experiment
PUT {{app_url}}/admin/experiments/test-experiment
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "name": "Updated A/B Experiment",
    "description": "Updated description"
}
HTTP 200
[Asserts]
jsonpath "$.name" == "Updated A/B Experiment"
jsonpath "$.description" == "Updated description"

# Start experiment (draft -> active)
POST {{app_url}}/admin/experiments/test-experiment/start
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.status" == "active"
jsonpath "$.started_at" exists

# Pause experiment (active -> paused)
POST {{app_url}}/admin/experiments/test-experiment/pause
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.status" == "paused"

# Resume experiment (paused -> active)
POST {{app_url}}/admin/experiments/test-experiment/resume
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.status" == "active"

# Complete experiment (active -> completed)
POST {{app_url}}/admin/experiments/test-experiment/complete
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.status" == "completed"
jsonpath "$.completed_at" exists

# Get experiment results
GET {{app_url}}/admin/experiments/test-experiment/results
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.experiment_id" == "test-experiment"
jsonpath "$.variant_metrics" isCollection

# List experiments with status filter
GET {{app_url}}/admin/experiments?status=completed
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.experiments" isCollection

# Delete experiment
DELETE {{app_url}}/admin/experiments/test-experiment
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200

# Verify deletion
GET {{app_url}}/admin/experiments/test-experiment
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 404

# Test invalid status transitions
# Create another experiment for transition tests
POST {{app_url}}/admin/experiments
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "id": "transition-test",
    "name": "Transition Test Experiment",
    "variants": [
        {
            "id": "control",
            "name": "Control",
            "config": {"type": "model_reference", "model_id": "gpt-4"},
            "control": true
        },
        {
            "id": "treatment",
            "name": "Treatment",
            "config": {"type": "model_reference", "model_id": "gpt-3.5-turbo"},
            "control": false
        }
    ],
    "traffic_allocation": [
        {"variant_id": "control", "percentage": 50},
        {"variant_id": "treatment", "percentage": 50}
    ]
}
HTTP 200

# Try to pause a draft experiment (invalid transition)
POST {{app_url}}/admin/experiments/transition-test/pause
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 400
[Asserts]
jsonpath "$.error" exists

# Clean up transition test experiment
DELETE {{app_url}}/admin/experiments/transition-test
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200

# Test traffic allocation validation
POST {{app_url}}/admin/experiments
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "id": "invalid-traffic",
    "name": "Invalid Traffic Test",
    "variants": [
        {
            "id": "control",
            "name": "Control",
            "config": {"type": "model_reference", "model_id": "gpt-4"},
            "control": true
        },
        {
            "id": "treatment",
            "name": "Treatment",
            "config": {"type": "model_reference", "model_id": "gpt-3.5-turbo"},
            "control": false
        }
    ],
    "traffic_allocation": [
        {"variant_id": "control", "percentage": 30},
        {"variant_id": "treatment", "percentage": 30}
    ]
}
HTTP 400
[Asserts]
jsonpath "$.error" exists
