# Non-Streaming Chat Completions Tests
# Test chat completions with stream=false

# Non-streaming chat completion
POST {{app_url}}/v1/chat/completions
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "model": "gpt-4",
    "messages": [
        {"role": "system", "content": "You are a helpful assistant."},
        {"role": "user", "content": "What is 2+2?"}
    ],
    "stream": false
}
HTTP 200
[Asserts]
header "Content-Type" contains "application/json"
jsonpath "$.id" exists
jsonpath "$.object" == "chat.completion"
jsonpath "$.model" exists
jsonpath "$.choices" isCollection
jsonpath "$.choices" count >= 1
jsonpath "$.choices[0].message.role" == "assistant"
jsonpath "$.choices[0].message.content" exists
jsonpath "$.choices[0].finish_reason" exists
jsonpath "$.usage" exists
jsonpath "$.usage.prompt_tokens" >= 0
jsonpath "$.usage.completion_tokens" >= 0

# Non-streaming with temperature
POST {{app_url}}/v1/chat/completions
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "model": "gpt-4",
    "messages": [
        {"role": "user", "content": "Hello"}
    ],
    "stream": false,
    "temperature": 0.7
}
HTTP 200
[Asserts]
jsonpath "$.choices[0].message.content" exists

# Non-streaming with max_tokens
POST {{app_url}}/v1/chat/completions
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "model": "gpt-4",
    "messages": [
        {"role": "user", "content": "Tell me a story"}
    ],
    "stream": false,
    "max_tokens": 50
}
HTTP 200
[Asserts]
jsonpath "$.choices" count >= 1

# Non-streaming with top_p
POST {{app_url}}/v1/chat/completions
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "model": "gpt-4",
    "messages": [
        {"role": "user", "content": "Hi"}
    ],
    "stream": false,
    "top_p": 0.9
}
HTTP 200
[Asserts]
jsonpath "$.choices[0].message" exists

# Non-streaming with user identifier
POST {{app_url}}/v1/chat/completions
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "model": "gpt-4",
    "messages": [
        {"role": "user", "content": "Test"}
    ],
    "stream": false,
    "user": "test-user-123"
}
HTTP 200
[Asserts]
jsonpath "$.id" exists

# Multi-turn conversation
POST {{app_url}}/v1/chat/completions
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "model": "gpt-4",
    "messages": [
        {"role": "system", "content": "You are a math tutor."},
        {"role": "user", "content": "What is 5*5?"},
        {"role": "assistant", "content": "5*5 equals 25."},
        {"role": "user", "content": "And what about 6*6?"}
    ],
    "stream": false
}
HTTP 200
[Asserts]
jsonpath "$.choices[0].message.role" == "assistant"
