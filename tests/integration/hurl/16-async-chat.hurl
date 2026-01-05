# Async Chat Completion Tests
# Test async mode for chat completions

# Async chat completion returns 202 Accepted
POST {{app_url}}/v1/chat/completions?async=true
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "model": "gpt-4",
    "messages": [{"role": "user", "content": "What is the meaning of life?"}],
    "stream": false
}
HTTP 202
[Captures]
operation_id: jsonpath "$.operation_id"
[Asserts]
jsonpath "$.status" == "pending"
jsonpath "$.operation_id" exists

# Wait a moment for the operation to complete
# Note: In real tests you'd poll, but mocks respond instantly
GET {{app_url}}/v1/operations/{{operation_id}}
Authorization: Bearer pk_test_{{admin_api_key}}
[Options]
delay: 500
HTTP 200
[Asserts]
jsonpath "$.id" == {{operation_id}}
jsonpath "$.status" exists

# List all operations (should include our operation)
GET {{app_url}}/v1/operations
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.operations" isCollection

# Non-existent operation returns 404
GET {{app_url}}/v1/operations/non-existent-operation-id
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 404
[Asserts]
jsonpath "$.error.type" exists
