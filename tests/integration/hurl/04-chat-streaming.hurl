# Chat Completions Streaming Tests
# Test SSE streaming responses

# Streaming chat completion
POST {{app_url}}/v1/chat/completions
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "model": "gpt-4",
    "messages": [
        {"role": "user", "content": "Hello, stream me a response!"}
    ],
    "stream": true
}
HTTP 200
[Asserts]
header "Content-Type" contains "text/event-stream"
body contains "data:"
