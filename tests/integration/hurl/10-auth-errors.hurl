# Authentication and Authorization Tests
# Test various auth scenarios and error handling

# Access admin endpoint without auth
GET {{app_url}}/admin/models
HTTP 401
[Asserts]
jsonpath "$.error.type" exists
jsonpath "$.error.message" contains "API key"

# Access admin endpoint with malformed auth header
GET {{app_url}}/admin/models
Authorization: InvalidFormat
HTTP 401

# Access admin endpoint with wrong prefix
GET {{app_url}}/admin/models
Authorization: Basic dGVzdDp0ZXN0
HTTP 401

# Access v1 endpoint without auth
POST {{app_url}}/v1/chat/completions
Content-Type: application/json
{
    "model": "gpt-4",
    "messages": [{"role": "user", "content": "Hello"}]
}
HTTP 401
[Asserts]
jsonpath "$.error.type" exists

# Health endpoints should work without auth
GET {{app_url}}/health
HTTP 200

GET {{app_url}}/ready
HTTP 200

GET {{app_url}}/live
HTTP 200

# Test invalid model reference
GET {{app_url}}/v1/models/this-model-does-not-exist
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 404
[Asserts]
jsonpath "$.error.type" == "not_found_error"
