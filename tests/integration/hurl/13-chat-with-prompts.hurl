# Chat Completions with Prompt References
# Test using prompt templates in chat messages

# Create a test prompt for chat
POST {{app_url}}/admin/prompts
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "id": "chat-system-prompt",
    "name": "Chat System Prompt",
    "content": "You are ${var:personality:a helpful assistant}. Always respond in ${var:language:English}."
}
HTTP 200
[Asserts]
jsonpath "$.id" == "chat-system-prompt"

# Create a user prompt template
POST {{app_url}}/admin/prompts
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "id": "question-template",
    "name": "Question Template",
    "content": "Please answer this question: ${var:question}"
}
HTTP 200
[Asserts]
jsonpath "$.id" == "question-template"

# Chat with system prompt reference
POST {{app_url}}/v1/chat/completions
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "model": "gpt-4",
    "messages": [
        {
            "role": "system",
            "prompt_id": "chat-system-prompt",
            "variables": {
                "personality": "a friendly tutor",
                "language": "Spanish"
            }
        },
        {
            "role": "user",
            "content": "What is photosynthesis?"
        }
    ],
    "stream": false
}
HTTP 200
[Asserts]
jsonpath "$.choices[0].message.content" exists

# Chat with user prompt reference
POST {{app_url}}/v1/chat/completions
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "model": "gpt-4",
    "messages": [
        {
            "role": "system",
            "content": "You are helpful."
        },
        {
            "role": "user",
            "prompt_id": "question-template",
            "variables": {
                "question": "What is the speed of light?"
            }
        }
    ],
    "stream": false
}
HTTP 200
[Asserts]
jsonpath "$.choices[0].message" exists

# Chat with prompt using default values
POST {{app_url}}/v1/chat/completions
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "model": "gpt-4",
    "messages": [
        {
            "role": "system",
            "prompt_id": "chat-system-prompt"
        },
        {
            "role": "user",
            "content": "Hello!"
        }
    ],
    "stream": false
}
HTTP 200
[Asserts]
jsonpath "$.choices" count >= 1

# Chat with pre-loaded helpful-assistant prompt
POST {{app_url}}/v1/chat/completions
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "model": "gpt-4",
    "messages": [
        {
            "role": "system",
            "prompt_id": "helpful-assistant"
        },
        {
            "role": "user",
            "content": "What day is today?"
        }
    ],
    "stream": false
}
HTTP 200
[Asserts]
jsonpath "$.id" exists

# Chat with non-existent prompt - should fail
POST {{app_url}}/v1/chat/completions
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "model": "gpt-4",
    "messages": [
        {
            "role": "system",
            "prompt_id": "non-existent-prompt"
        },
        {
            "role": "user",
            "content": "Hello"
        }
    ],
    "stream": false
}
HTTP 400
[Asserts]
jsonpath "$.error.message" contains "prompt"

# Clean up test prompts
DELETE {{app_url}}/admin/prompts/chat-system-prompt
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200

DELETE {{app_url}}/admin/prompts/question-template
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
