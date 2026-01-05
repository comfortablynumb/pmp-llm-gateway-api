# Admin Models Endpoints
# Test model management (separate from v1/models)

# List all models via admin
GET {{app_url}}/admin/models
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.models" isCollection
jsonpath "$.models" count >= 5

# Get specific model via admin
GET {{app_url}}/admin/models/gpt-4
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.id" == "gpt-4"
jsonpath "$.name" == "GPT-4"
jsonpath "$.provider" exists
jsonpath "$.provider_model" == "gpt-4"

# Create a new model
POST {{app_url}}/admin/models
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "id": "test-model",
    "name": "Test Model",
    "provider": "openai",
    "provider_model": "gpt-4-test"
}
HTTP 200
[Asserts]
jsonpath "$.id" == "test-model"
jsonpath "$.name" == "Test Model"

# Get the created model
GET {{app_url}}/admin/models/test-model
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.id" == "test-model"

# Update model
PUT {{app_url}}/admin/models/test-model
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "name": "Updated Test Model",
    "provider": "openai",
    "provider_model": "gpt-4-updated"
}
HTTP 200
[Asserts]
jsonpath "$.name" == "Updated Test Model"
jsonpath "$.provider_model" == "gpt-4-updated"

# Delete model (returns 200 not 204)
DELETE {{app_url}}/admin/models/test-model
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200

# Verify deletion
GET {{app_url}}/admin/models/test-model
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 404

# Get credential providers info
GET {{app_url}}/admin/credentials/providers
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.providers" isCollection
