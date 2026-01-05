# Admin Prompts Endpoints
# Test CRUD operations for prompts

# List all prompts
GET {{app_url}}/admin/prompts
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.prompts" isCollection
jsonpath "$.prompts" count >= 2

# Get specific prompt (pre-loaded helpful-assistant)
GET {{app_url}}/admin/prompts/helpful-assistant
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.id" == "helpful-assistant"
jsonpath "$.name" == "Helpful Assistant"
jsonpath "$.content" exists

# Get code-reviewer prompt
GET {{app_url}}/admin/prompts/code-reviewer
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.id" == "code-reviewer"
jsonpath "$.name" == "Code Reviewer"
jsonpath "$.content" contains "${var:code}"

# Create a new prompt
POST {{app_url}}/admin/prompts
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "id": "test-prompt",
    "name": "Test Prompt",
    "content": "Hello ${var:name}, welcome to ${var:place:the system}!"
}
HTTP 200
[Asserts]
jsonpath "$.id" == "test-prompt"
jsonpath "$.name" == "Test Prompt"
jsonpath "$.content" contains "${var:name}"

# Get the created prompt
GET {{app_url}}/admin/prompts/test-prompt
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.id" == "test-prompt"

# Render prompt with variables
POST {{app_url}}/admin/prompts/test-prompt/render
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "variables": {
        "name": "John",
        "place": "our platform"
    }
}
HTTP 200
[Asserts]
jsonpath "$.rendered" == "Hello John, welcome to our platform!"

# Render prompt with default values
POST {{app_url}}/admin/prompts/test-prompt/render
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "variables": {
        "name": "Jane"
    }
}
HTTP 200
[Asserts]
jsonpath "$.rendered" == "Hello Jane, welcome to the system!"

# Update prompt
PUT {{app_url}}/admin/prompts/test-prompt
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "name": "Updated Test Prompt",
    "content": "Updated content: ${var:message}"
}
HTTP 200
[Asserts]
jsonpath "$.name" == "Updated Test Prompt"
jsonpath "$.content" contains "Updated content"

# Delete prompt (returns 200 not 204)
DELETE {{app_url}}/admin/prompts/test-prompt
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200

# Verify deletion
GET {{app_url}}/admin/prompts/test-prompt
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 404
[Asserts]
jsonpath "$.error.message" exists

# Get non-existent prompt
GET {{app_url}}/admin/prompts/non-existent-prompt
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 404
[Asserts]
jsonpath "$.error.type" exists
