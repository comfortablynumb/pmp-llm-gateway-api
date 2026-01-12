# Execute Endpoints Tests
# Test model and workflow execution endpoints

# === Model Execution ===

# Execute model without prompt
POST {{app_url}}/admin/models/gpt-4/execute
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "user_message": "Say hello"
}
HTTP 200
[Asserts]
jsonpath "$.model_id" == "gpt-4"
jsonpath "$.content" exists
jsonpath "$.usage" exists
jsonpath "$.usage.prompt_tokens" isInteger
jsonpath "$.usage.completion_tokens" isInteger
jsonpath "$.usage.total_tokens" isInteger
jsonpath "$.execution_time_ms" isInteger

# Execute model with prompt
POST {{app_url}}/admin/models/gpt-4/execute
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "prompt_id": "default-chat",
    "user_message": "What is your name?"
}
HTTP 200
[Asserts]
jsonpath "$.model_id" == "gpt-4"
jsonpath "$.prompt_id" == "default-chat"
jsonpath "$.content" exists

# Execute model with temperature override
POST {{app_url}}/admin/models/gpt-4/execute
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "user_message": "Hello",
    "temperature": 0.5,
    "max_tokens": 100
}
HTTP 200
[Asserts]
jsonpath "$.content" exists

# Execute non-existent model (404)
POST {{app_url}}/admin/models/non-existent-model/execute
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "user_message": "Hello"
}
HTTP 404

# Execute model without user message (400)
POST {{app_url}}/admin/models/gpt-4/execute
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "prompt_id": "default-chat"
}
HTTP 400

# Execute model without auth (401)
POST {{app_url}}/admin/models/gpt-4/execute
Content-Type: application/json
{
    "user_message": "Hello"
}
HTTP 401

# === Workflow Execution ===

# Execute basic-rag workflow
POST {{app_url}}/admin/workflows/basic-rag/execute
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "input": {
        "query": "What is Rust programming language?"
    }
}
HTTP 200
[Asserts]
jsonpath "$.workflow_id" == "basic-rag"
jsonpath "$.success" exists
jsonpath "$.output" exists
jsonpath "$.step_results" isCollection
jsonpath "$.execution_time_ms" isInteger

# Execute workflow with empty input
POST {{app_url}}/admin/workflows/basic-rag/execute
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "input": {}
}
HTTP 200
[Asserts]
jsonpath "$.workflow_id" == "basic-rag"

# Execute non-existent workflow (404)
POST {{app_url}}/admin/workflows/non-existent-workflow/execute
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "input": {}
}
HTTP 404

# Execute workflow without auth (401)
POST {{app_url}}/admin/workflows/basic-rag/execute
Content-Type: application/json
{
    "input": {}
}
HTTP 401
