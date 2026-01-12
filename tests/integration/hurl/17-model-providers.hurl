# Model Providers Tests
# Test different model provider configurations

# List all default models (should have OpenAI and Anthropic models)
GET {{app_url}}/admin/models
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.models" isCollection
jsonpath "$.models" count >= 5

# Get GPT-4 model (OpenAI provider)
GET {{app_url}}/admin/models/gpt-4
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.id" == "gpt-4"
jsonpath "$.provider" == "openai"
jsonpath "$.provider_model" == "gpt-4"

# Get Claude 3.5 Sonnet model (Anthropic provider)
GET {{app_url}}/admin/models/claude-3-5-sonnet
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.id" == "claude-3-5-sonnet"
jsonpath "$.provider" == "anthropic"
jsonpath "$.provider_model" == "claude-3-5-sonnet-20241022"

# Get GPT-4o model
GET {{app_url}}/admin/models/gpt-4o
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.id" == "gpt-4o"
jsonpath "$.provider" == "openai"

# Get GPT-3.5 Turbo model
GET {{app_url}}/admin/models/gpt-3-5-turbo
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.id" == "gpt-3-5-turbo"
jsonpath "$.provider_model" == "gpt-3.5-turbo"

# Get Claude 3 Opus model
GET {{app_url}}/admin/models/claude-3-opus
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.id" == "claude-3-opus"
jsonpath "$.provider" == "anthropic"

# Create a custom OpenAI model
POST {{app_url}}/admin/models
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "id": "custom-gpt4-turbo",
    "name": "Custom GPT-4 Turbo",
    "provider": "openai",
    "provider_model": "gpt-4-turbo-preview",
    "credential_id": "openai-default"
}
HTTP 200
[Asserts]
jsonpath "$.id" == "custom-gpt4-turbo"
jsonpath "$.provider" == "openai"

# Verify custom model appears in list
GET {{app_url}}/v1/models
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.data" isCollection

# Use custom model in chat (routed to OpenAI mock)
POST {{app_url}}/v1/chat/completions
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "model": "custom-gpt4-turbo",
    "messages": [{"role": "user", "content": "Hello"}],
    "stream": false
}
HTTP 200
[Asserts]
jsonpath "$.choices[0].message.content" exists

# Clean up custom model
DELETE {{app_url}}/admin/models/custom-gpt4-turbo
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200

# Create an Anthropic model
POST {{app_url}}/admin/models
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "id": "custom-claude",
    "name": "Custom Claude",
    "provider": "anthropic",
    "provider_model": "claude-3-haiku-20240307",
    "credential_id": "anthropic-default"
}
HTTP 200
[Asserts]
jsonpath "$.id" == "custom-claude"
jsonpath "$.provider" == "anthropic"

# Clean up
DELETE {{app_url}}/admin/models/custom-claude
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200

# Create Azure credential for testing
POST {{app_url}}/admin/credentials
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "id": "azure-test-credential",
    "name": "Azure Test Credential",
    "credential_type": "azure_openai",
    "api_key": "test-azure-api-key",
    "endpoint": "https://test.openai.azure.com"
}
HTTP 200
[Asserts]
jsonpath "$.id" == "azure-test-credential"

# Create an Azure OpenAI model
POST {{app_url}}/admin/models
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "id": "azure-gpt4",
    "name": "Azure GPT-4",
    "provider": "azure_openai",
    "provider_model": "gpt-4-deployment",
    "credential_id": "azure-test-credential"
}
HTTP 200
[Asserts]
jsonpath "$.id" == "azure-gpt4"
jsonpath "$.provider" == "azure_openai"

# Clean up model first (before credential)
DELETE {{app_url}}/admin/models/azure-gpt4
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200

# Clean up credential
DELETE {{app_url}}/admin/credentials/azure-test-credential
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
