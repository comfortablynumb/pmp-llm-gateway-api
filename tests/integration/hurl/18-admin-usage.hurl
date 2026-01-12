# Admin Usage Tracking Endpoints
# Test usage tracking and analytics

# List usage records (initially empty or with default data)
GET {{app_url}}/admin/usage
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.records" isCollection
jsonpath "$.count" isInteger

# List usage records with query parameters
GET {{app_url}}/admin/usage?limit=10&offset=0
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.records" isCollection
jsonpath "$.count" isInteger

# List usage records filtered by API key
GET {{app_url}}/admin/usage?api_key_id=test-key
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.records" isCollection

# List usage records filtered by model
GET {{app_url}}/admin/usage?model_id=gpt-4o
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.records" isCollection

# List usage records filtered by time range
GET {{app_url}}/admin/usage?from_timestamp=0&to_timestamp=9999999999
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.records" isCollection

# Get usage aggregate
GET {{app_url}}/admin/usage/aggregate
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.total_requests" isInteger
jsonpath "$.successful_requests" isInteger
jsonpath "$.failed_requests" isInteger
jsonpath "$.total_input_tokens" isInteger
jsonpath "$.total_output_tokens" isInteger
jsonpath "$.total_tokens" isInteger
jsonpath "$.total_cost_usd" isFloat
jsonpath "$.avg_latency_ms" isFloat
jsonpath "$.success_rate" isFloat

# Get usage aggregate with filters
GET {{app_url}}/admin/usage/aggregate?api_key_id=test-key
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.total_requests" isInteger

# Get usage summary with daily breakdown
GET {{app_url}}/admin/usage/summary
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.period_start" isInteger
jsonpath "$.period_end" isInteger
jsonpath "$.aggregate" isCollection
jsonpath "$.aggregate.total_requests" isInteger
jsonpath "$.daily" isCollection

# Get usage summary with time range
GET {{app_url}}/admin/usage/summary?from_timestamp=0&to_timestamp=9999999999
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.period_start" == 0
jsonpath "$.period_end" == 9999999999
jsonpath "$.aggregate" isCollection
jsonpath "$.daily" isCollection

# Get usage summary filtered by API key
GET {{app_url}}/admin/usage/summary?api_key_id=test-key
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.aggregate" isCollection

# Delete old usage records (use timestamp 0 to delete nothing)
DELETE {{app_url}}/admin/usage?before_timestamp=0
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.deleted" isInteger
jsonpath "$.deleted" >= 0

# Test authentication required for usage endpoints
GET {{app_url}}/admin/usage
HTTP 401
[Asserts]
jsonpath "$.error.type" exists

# Test authentication required for aggregate endpoint
GET {{app_url}}/admin/usage/aggregate
HTTP 401
[Asserts]
jsonpath "$.error.type" exists

# Test authentication required for summary endpoint
GET {{app_url}}/admin/usage/summary
HTTP 401
[Asserts]
jsonpath "$.error.type" exists
