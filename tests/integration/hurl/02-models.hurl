# Models Endpoints
# Test the /v1/models endpoint (OpenAI-compatible)

# List all models (requires auth)
GET {{app_url}}/v1/models
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.object" == "list"
jsonpath "$.data" isCollection

# Get non-existent model
GET {{app_url}}/v1/models/non-existent-model
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 404
[Asserts]
jsonpath "$.error.message" exists
jsonpath "$.error.type" == "not_found_error"
