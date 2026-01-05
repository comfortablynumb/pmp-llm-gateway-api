# API Key Usage Tests
# Test created API keys can be used for authentication

# Create an API key for testing
POST {{app_url}}/admin/api-keys
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "name": "Test Usage Key",
    "permissions": {
        "admin": false,
        "models": "all",
        "knowledge_bases": "none",
        "prompts": "all",
        "chains": "none"
    }
}
HTTP 200
[Captures]
test_key_id: jsonpath "$.id"
test_key_secret: jsonpath "$.secret"
[Asserts]
jsonpath "$.secret" startsWith "pk_"

# Use the created key with Bearer token
GET {{app_url}}/v1/models
Authorization: Bearer {{test_key_secret}}
HTTP 200
[Asserts]
jsonpath "$.data" isCollection

# Use the created key with X-API-Key header
GET {{app_url}}/v1/models
X-API-Key: {{test_key_secret}}
HTTP 200
[Asserts]
jsonpath "$.data" isCollection

# Use the key for chat completion
POST {{app_url}}/v1/chat/completions
Authorization: Bearer {{test_key_secret}}
Content-Type: application/json
{
    "model": "gpt-4",
    "messages": [{"role": "user", "content": "Hi"}],
    "stream": false
}
HTTP 200
[Asserts]
jsonpath "$.choices" exists

# Suspend the key
POST {{app_url}}/admin/api-keys/{{test_key_id}}/suspend
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.status" == "suspended"

# Suspended key should not work
GET {{app_url}}/v1/models
Authorization: Bearer {{test_key_secret}}
HTTP 401
[Asserts]
jsonpath "$.error.message" exists

# Reactivate the key
POST {{app_url}}/admin/api-keys/{{test_key_id}}/activate
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.status" == "active"

# Reactivated key should work again
GET {{app_url}}/v1/models
Authorization: Bearer {{test_key_secret}}
HTTP 200
[Asserts]
jsonpath "$.data" isCollection

# Revoke the key
POST {{app_url}}/admin/api-keys/{{test_key_id}}/revoke
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.status" == "revoked"

# Revoked key should not work
GET {{app_url}}/v1/models
Authorization: Bearer {{test_key_secret}}
HTTP 401
[Asserts]
jsonpath "$.error.message" exists

# Clean up
DELETE {{app_url}}/admin/api-keys/{{test_key_id}}
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200

# Test non-admin key cannot access admin endpoints
POST {{app_url}}/admin/api-keys
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "name": "Non-Admin Key",
    "permissions": {
        "admin": false,
        "models": "all",
        "knowledge_bases": "none",
        "prompts": "none",
        "chains": "none"
    }
}
HTTP 200
[Captures]
non_admin_key_id: jsonpath "$.id"
non_admin_key_secret: jsonpath "$.secret"

# Non-admin key can access v1 endpoints
GET {{app_url}}/v1/models
Authorization: Bearer {{non_admin_key_secret}}
HTTP 200

# Clean up non-admin key
DELETE {{app_url}}/admin/api-keys/{{non_admin_key_id}}
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
