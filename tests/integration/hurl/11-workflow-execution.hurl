# Workflow Execution Tests
# Test workflow creation and execution with variable resolution

# Create a simple workflow for execution testing
POST {{app_url}}/admin/workflows
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "id": "exec-test-workflow",
    "name": "Execution Test Workflow",
    "description": "A workflow for testing execution",
    "steps": [
        {
            "name": "greet",
            "type": "chat_completion",
            "model_id": "gpt-4",
            "system_message": "You are a helpful assistant. Respond briefly.",
            "user_message": "${request:user_message}"
        }
    ]
}
HTTP 200
[Asserts]
jsonpath "$.id" == "exec-test-workflow"
jsonpath "$.steps" count == 1

# Execute the workflow with input
POST {{app_url}}/v1/workflows/exec-test-workflow/execute
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "input": {
        "user_message": "Say hello!"
    }
}
HTTP 200
[Asserts]
jsonpath "$.success" == true
jsonpath "$.output" exists
jsonpath "$.execution_time_ms" >= 0
jsonpath "$.steps" count == 1
jsonpath "$.steps[0].name" == "greet"
jsonpath "$.steps[0].success" == true

# Execute workflow with empty input (uses null for missing variables)
POST {{app_url}}/v1/workflows/exec-test-workflow/execute
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "input": {}
}
HTTP 200
[Asserts]
jsonpath "$.success" exists
jsonpath "$.steps" count == 1

# Create a multi-step workflow
POST {{app_url}}/admin/workflows
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "id": "multi-step-workflow",
    "name": "Multi-Step Workflow",
    "description": "A workflow with multiple steps",
    "steps": [
        {
            "name": "step1",
            "type": "chat_completion",
            "model_id": "gpt-4",
            "system_message": "You analyze text.",
            "user_message": "${request:text}"
        },
        {
            "name": "step2",
            "type": "chat_completion",
            "model_id": "gpt-4",
            "system_message": "You summarize analysis.",
            "user_message": "Summarize: ${step:step1:content}"
        }
    ]
}
HTTP 200
[Asserts]
jsonpath "$.id" == "multi-step-workflow"
jsonpath "$.steps" count == 2

# Execute multi-step workflow
POST {{app_url}}/v1/workflows/multi-step-workflow/execute
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "input": {
        "text": "This is a test document about software testing."
    }
}
HTTP 200
[Asserts]
jsonpath "$.success" == true
jsonpath "$.steps" count == 2
jsonpath "$.steps[0].name" == "step1"
jsonpath "$.steps[1].name" == "step2"

# Execute workflow without auth - should fail
POST {{app_url}}/v1/workflows/exec-test-workflow/execute
Content-Type: application/json
{
    "input": {}
}
HTTP 401
[Asserts]
jsonpath "$.error.type" exists

# Clean up - delete test workflows
DELETE {{app_url}}/admin/workflows/exec-test-workflow
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200

DELETE {{app_url}}/admin/workflows/multi-step-workflow
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
