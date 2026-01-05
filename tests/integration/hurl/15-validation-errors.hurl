# Validation Error Tests
# Test various validation and error scenarios

# Chat completion with invalid model
POST {{app_url}}/v1/chat/completions
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "model": "non-existent-model",
    "messages": [{"role": "user", "content": "Hello"}],
    "stream": false
}
HTTP 404
[Asserts]
jsonpath "$.error.type" exists

# Chat completion with invalid temperature (too high)
POST {{app_url}}/v1/chat/completions
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "model": "gpt-4",
    "messages": [{"role": "user", "content": "Hello"}],
    "stream": false,
    "temperature": 3.0
}
HTTP 400
[Asserts]
jsonpath "$.error.type" == "invalid_request_error"
jsonpath "$.error.message" contains "temperature"

# Chat completion with invalid temperature (negative)
POST {{app_url}}/v1/chat/completions
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "model": "gpt-4",
    "messages": [{"role": "user", "content": "Hello"}],
    "stream": false,
    "temperature": -0.5
}
HTTP 400
[Asserts]
jsonpath "$.error.message" contains "temperature"

# Chat completion with invalid top_p (too high)
POST {{app_url}}/v1/chat/completions
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "model": "gpt-4",
    "messages": [{"role": "user", "content": "Hello"}],
    "stream": false,
    "top_p": 1.5
}
HTTP 400
[Asserts]
jsonpath "$.error.message" contains "top_p"

# Chat completion with invalid presence_penalty
POST {{app_url}}/v1/chat/completions
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "model": "gpt-4",
    "messages": [{"role": "user", "content": "Hello"}],
    "stream": false,
    "presence_penalty": 3.0
}
HTTP 400
[Asserts]
jsonpath "$.error.message" contains "presence_penalty"

# Chat completion with invalid frequency_penalty
POST {{app_url}}/v1/chat/completions
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "model": "gpt-4",
    "messages": [{"role": "user", "content": "Hello"}],
    "stream": false,
    "frequency_penalty": -3.0
}
HTTP 400
[Asserts]
jsonpath "$.error.message" contains "frequency_penalty"

# Streaming and async are incompatible
POST {{app_url}}/v1/chat/completions?async=true
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "model": "gpt-4",
    "messages": [{"role": "user", "content": "Hello"}],
    "stream": true
}
HTTP 400
[Asserts]
jsonpath "$.error.message" contains "async"

# Create model with invalid ID (empty)
POST {{app_url}}/admin/models
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "id": "",
    "name": "Invalid Model",
    "provider": "openai",
    "provider_model": "gpt-4"
}
HTTP 400
[Asserts]
jsonpath "$.error.message" exists

# Create prompt with missing required fields
POST {{app_url}}/admin/prompts
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "id": "test-prompt"
}
HTTP 400
[Asserts]
jsonpath "$.error" exists

# Create API key with invalid permissions
POST {{app_url}}/admin/api-keys
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "name": "Invalid Key"
}
HTTP 400
[Asserts]
jsonpath "$.error" exists

# Create workflow with empty steps
POST {{app_url}}/admin/workflows
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "id": "empty-workflow",
    "name": "Empty Workflow",
    "description": "Has no steps",
    "steps": []
}
HTTP 400
[Asserts]
jsonpath "$.error.message" exists

# Invalid JSON body
POST {{app_url}}/v1/chat/completions
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
not valid json
HTTP 400

# Update non-existent model
PUT {{app_url}}/admin/models/definitely-does-not-exist
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "name": "Updated",
    "provider": "openai",
    "provider_model": "gpt-4"
}
HTTP 404
[Asserts]
jsonpath "$.error.type" exists

# Delete non-existent prompt
DELETE {{app_url}}/admin/prompts/definitely-does-not-exist
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 404

# Update non-existent workflow
PUT {{app_url}}/admin/workflows/definitely-does-not-exist
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "name": "Updated"
}
HTTP 404
