# Chat Completions Endpoints
# Test the /v1/chat/completions endpoint

# Streaming chat completion (default test mode)
POST {{app_url}}/v1/chat/completions
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "model": "gpt-4",
    "messages": [
        {"role": "user", "content": "Hello, how are you?"}
    ],
    "stream": true
}
HTTP 200
[Asserts]
header "Content-Type" contains "text/event-stream"
body contains "data:"

# Chat completion without auth - should fail
POST {{app_url}}/v1/chat/completions
Content-Type: application/json
{
    "model": "gpt-4",
    "messages": [
        {"role": "user", "content": "Hello"}
    ],
    "stream": true
}
HTTP 401
[Asserts]
jsonpath "$.error.type" exists

# Chat completion with empty messages - should fail
POST {{app_url}}/v1/chat/completions
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "model": "gpt-4",
    "messages": []
}
HTTP 400
[Asserts]
jsonpath "$.error.message" exists
jsonpath "$.error.type" == "invalid_request_error"
