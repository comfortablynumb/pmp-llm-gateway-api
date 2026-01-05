# Async Operations Tests
# Skip async operations for now due to stack size issues
# TODO: Re-enable when stack overflow is fixed

# List all operations (empty initially)
GET {{app_url}}/v1/operations
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.operations" isCollection
