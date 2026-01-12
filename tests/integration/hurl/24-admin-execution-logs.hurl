# Admin Execution Logs Endpoints
# Test execution log management and statistics

# List execution logs (initially empty or with default data)
GET {{app_url}}/admin/execution-logs
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.logs" isCollection
jsonpath "$.total" isInteger

# List execution logs with limit
GET {{app_url}}/admin/execution-logs?limit=10
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.logs" isCollection
jsonpath "$.total" isInteger

# List execution logs with offset
GET {{app_url}}/admin/execution-logs?limit=10&offset=0
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.logs" isCollection
jsonpath "$.total" isInteger

# List execution logs filtered by type - model
GET {{app_url}}/admin/execution-logs?execution_type=model
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.logs" isCollection
jsonpath "$.total" isInteger

# List execution logs filtered by type - workflow
GET {{app_url}}/admin/execution-logs?execution_type=workflow
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.logs" isCollection
jsonpath "$.total" isInteger

# List execution logs filtered by type - chat_completion
GET {{app_url}}/admin/execution-logs?execution_type=chat_completion
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.logs" isCollection
jsonpath "$.total" isInteger

# List execution logs filtered by status - success
GET {{app_url}}/admin/execution-logs?status=success
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.logs" isCollection
jsonpath "$.total" isInteger

# List execution logs filtered by status - failed
GET {{app_url}}/admin/execution-logs?status=failed
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.logs" isCollection
jsonpath "$.total" isInteger

# List execution logs filtered by resource
GET {{app_url}}/admin/execution-logs?resource_id=test-resource
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.logs" isCollection
jsonpath "$.total" isInteger

# List execution logs filtered by API key
GET {{app_url}}/admin/execution-logs?api_key_id=test-key
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.logs" isCollection
jsonpath "$.total" isInteger

# Invalid execution type returns error
GET {{app_url}}/admin/execution-logs?execution_type=invalid
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 400
[Asserts]
jsonpath "$.error.type" exists

# Invalid status returns error
GET {{app_url}}/admin/execution-logs?status=invalid
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 400
[Asserts]
jsonpath "$.error.type" exists

# Get execution statistics
GET {{app_url}}/admin/execution-logs/stats
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.total_executions" isInteger
jsonpath "$.successful_executions" isInteger
jsonpath "$.failed_executions" isInteger
jsonpath "$.success_rate" isFloat
jsonpath "$.total_cost_micros" isInteger
jsonpath "$.total_input_tokens" isInteger
jsonpath "$.total_output_tokens" isInteger
jsonpath "$.avg_execution_time_ms" isFloat
jsonpath "$.executions_by_type" isCollection
jsonpath "$.executions_by_resource" isCollection

# Get execution statistics with filters
GET {{app_url}}/admin/execution-logs/stats?execution_type=model
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.total_executions" isInteger
jsonpath "$.success_rate" isFloat

# Get execution statistics filtered by status
GET {{app_url}}/admin/execution-logs/stats?status=success
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.total_executions" isInteger

# Cleanup old execution logs with days parameter
POST {{app_url}}/admin/execution-logs/cleanup
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "days": 365
}
HTTP 200
[Asserts]
jsonpath "$.deleted_count" isInteger
jsonpath "$.deleted_count" >= 0

# Cleanup old execution logs with null days (uses default)
POST {{app_url}}/admin/execution-logs/cleanup
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "days": null
}
HTTP 200
[Asserts]
jsonpath "$.deleted_count" isInteger

# Get non-existent execution log returns not found
GET {{app_url}}/admin/execution-logs/nonexistent-log-id
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 404
[Asserts]
jsonpath "$.error.type" exists

# Delete non-existent execution log returns not found
DELETE {{app_url}}/admin/execution-logs/nonexistent-log-id
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 404
[Asserts]
jsonpath "$.error.type" exists

# Test authentication required for list endpoint
GET {{app_url}}/admin/execution-logs
HTTP 401
[Asserts]
jsonpath "$.error.type" exists

# Test authentication required for stats endpoint
GET {{app_url}}/admin/execution-logs/stats
HTTP 401
[Asserts]
jsonpath "$.error.type" exists

# Test authentication required for cleanup endpoint
POST {{app_url}}/admin/execution-logs/cleanup
Content-Type: application/json
{
    "days": 30
}
HTTP 401
[Asserts]
jsonpath "$.error.type" exists

# Test authentication required for get endpoint
GET {{app_url}}/admin/execution-logs/test-log-id
HTTP 401
[Asserts]
jsonpath "$.error.type" exists

# Test authentication required for delete endpoint
DELETE {{app_url}}/admin/execution-logs/test-log-id
HTTP 401
[Asserts]
jsonpath "$.error.type" exists
