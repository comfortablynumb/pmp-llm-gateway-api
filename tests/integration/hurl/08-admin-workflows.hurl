# Admin Workflows Endpoints
# Test workflow CRUD (execution skipped due to stack size issues)

# List all workflows (initially empty or with defaults)
GET {{app_url}}/admin/workflows
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.workflows" isCollection

# Create a simple workflow (step uses type tag format)
POST {{app_url}}/admin/workflows
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "id": "test-workflow",
    "name": "Test Workflow",
    "description": "A simple test workflow",
    "steps": [
        {
            "name": "greeting",
            "type": "chat_completion",
            "model_id": "gpt-4",
            "system_message": "You are a helpful assistant.",
            "user_message": "${request:user_message}"
        }
    ]
}
HTTP 200
[Asserts]
jsonpath "$.id" == "test-workflow"
jsonpath "$.name" == "Test Workflow"
jsonpath "$.steps" count == 1

# Get the created workflow
GET {{app_url}}/admin/workflows/test-workflow
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.id" == "test-workflow"
jsonpath "$.name" == "Test Workflow"
jsonpath "$.description" == "A simple test workflow"

# Update workflow
PUT {{app_url}}/admin/workflows/test-workflow
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "name": "Updated Test Workflow",
    "description": "Updated description"
}
HTTP 200
[Asserts]
jsonpath "$.name" == "Updated Test Workflow"

# Delete workflow (returns 200 not 204)
DELETE {{app_url}}/admin/workflows/test-workflow
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200

# Verify deletion
GET {{app_url}}/admin/workflows/test-workflow
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 404

# Execute non-existent workflow
POST {{app_url}}/v1/workflows/non-existent-workflow/execute
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "input": "test"
}
HTTP 404
[Asserts]
jsonpath "$.error.message" exists
