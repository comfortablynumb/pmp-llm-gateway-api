# Admin Test Cases Endpoints
# Test case management for model+prompt and workflow testing

# List all test cases (should be empty initially)
GET {{app_url}}/admin/test-cases
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.test_cases" isCollection
jsonpath "$.total" >= 0

# Create a model+prompt test case
POST {{app_url}}/admin/test-cases
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "id": "test-case-model-1",
    "name": "Model Prompt Test Case",
    "description": "Tests GPT-4 with a simple prompt",
    "input": {
        "type": "model_prompt",
        "model_id": "gpt-4",
        "user_message": "What is 2+2?"
    },
    "assertions": [
        {
            "name": "response-contains-4",
            "operator": "contains",
            "expected": "4"
        }
    ],
    "tags": ["math", "simple"],
    "enabled": true
}
HTTP 200
[Asserts]
jsonpath "$.id" == "test-case-model-1"
jsonpath "$.name" == "Model Prompt Test Case"
jsonpath "$.test_type" == "model_prompt"
jsonpath "$.enabled" == true
jsonpath "$.tags" count == 2
jsonpath "$.assertions" count == 1

# Get the created test case
GET {{app_url}}/admin/test-cases/test-case-model-1
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.id" == "test-case-model-1"
jsonpath "$.name" == "Model Prompt Test Case"
jsonpath "$.input.type" == "model_prompt"
jsonpath "$.input.model_id" == "gpt-4"

# Update the test case
PUT {{app_url}}/admin/test-cases/test-case-model-1
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "name": "Updated Model Test Case",
    "description": "Updated description",
    "enabled": false
}
HTTP 200
[Asserts]
jsonpath "$.id" == "test-case-model-1"
jsonpath "$.name" == "Updated Model Test Case"
jsonpath "$.description" == "Updated description"
jsonpath "$.enabled" == false

# List test cases - should now have one
GET {{app_url}}/admin/test-cases
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.test_cases" count >= 1
jsonpath "$.total" >= 1

# Filter by enabled=false
GET {{app_url}}/admin/test-cases?enabled=false
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.test_cases" count >= 1

# Filter by tag
GET {{app_url}}/admin/test-cases?tag=math
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.test_cases" count >= 1

# Create a workflow test case
POST {{app_url}}/admin/test-cases
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "id": "test-case-workflow-1",
    "name": "Workflow Test Case",
    "description": "Tests a workflow",
    "input": {
        "type": "workflow",
        "workflow_id": "chain-of-thought",
        "input": {"question": "What is AI?"}
    },
    "assertions": [
        {
            "name": "response-not-empty",
            "operator": "length_greater_than",
            "expected": "0"
        }
    ],
    "tags": ["workflow"],
    "enabled": true
}
HTTP 200
[Asserts]
jsonpath "$.id" == "test-case-workflow-1"
jsonpath "$.test_type" == "workflow"
jsonpath "$.input.type" == "workflow"
jsonpath "$.input.workflow_id" == "chain-of-thought"

# Filter by test type
GET {{app_url}}/admin/test-cases?test_type=model_prompt
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.test_cases" count >= 1

GET {{app_url}}/admin/test-cases?test_type=workflow
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.test_cases" count >= 1

# Get test case results (should be empty)
GET {{app_url}}/admin/test-cases/test-case-model-1/results
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.results" isCollection
jsonpath "$.total" == 0

# Delete test cases
DELETE {{app_url}}/admin/test-cases/test-case-model-1
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.deleted" == true

DELETE {{app_url}}/admin/test-cases/test-case-workflow-1
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.deleted" == true

# Verify deletion
GET {{app_url}}/admin/test-cases/test-case-model-1
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 404

# Test 404 for non-existent test case
GET {{app_url}}/admin/test-cases/non-existent-test-case
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 404

# Test validation - missing required fields
POST {{app_url}}/admin/test-cases
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "id": "",
    "name": ""
}
HTTP 400

# Test invalid test type filter
GET {{app_url}}/admin/test-cases?test_type=invalid
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 400
