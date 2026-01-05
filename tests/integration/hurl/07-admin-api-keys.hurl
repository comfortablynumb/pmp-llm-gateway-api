# Admin API Keys Endpoints
# Test API key management

# List all API keys
GET {{app_url}}/admin/api-keys
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.api_keys" isCollection
jsonpath "$.api_keys" count >= 1

# Create a new API key (id is auto-generated, flattened response)
POST {{app_url}}/admin/api-keys
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "name": "Test Read-Only Key",
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
created_key_id: jsonpath "$.id"
readonly_key_secret: jsonpath "$.secret"
[Asserts]
jsonpath "$.name" == "Test Read-Only Key"
jsonpath "$.status" == "active"
jsonpath "$.secret" startsWith "pk_"
jsonpath "$.id" exists

# Get the created API key
GET {{app_url}}/admin/api-keys/{{created_key_id}}
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.name" == "Test Read-Only Key"
jsonpath "$.status" == "active"

# Suspend the API key
POST {{app_url}}/admin/api-keys/{{created_key_id}}/suspend
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.status" == "suspended"

# Activate the API key
POST {{app_url}}/admin/api-keys/{{created_key_id}}/activate
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.status" == "active"

# Revoke the API key
POST {{app_url}}/admin/api-keys/{{created_key_id}}/revoke
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.status" == "revoked"

# Delete the revoked key (returns 200 not 204)
DELETE {{app_url}}/admin/api-keys/{{created_key_id}}
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200

# Verify deletion
GET {{app_url}}/admin/api-keys/{{created_key_id}}
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 404

# Get non-existent API key
GET {{app_url}}/admin/api-keys/non-existent-key
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 404
[Asserts]
jsonpath "$.error.message" exists
