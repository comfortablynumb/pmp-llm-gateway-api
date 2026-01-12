# Admin Configuration Endpoints
# Test configuration management

# List all configuration entries
GET {{app_url}}/admin/config
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.config" isCollection

# List configuration by category - General
GET {{app_url}}/admin/config/category/general
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.config" isCollection

# List configuration by category - Persistence
GET {{app_url}}/admin/config/category/persistence
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.config" isCollection

# List configuration by category - Logging
GET {{app_url}}/admin/config/category/logging
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.config" isCollection

# List configuration by category - Security
GET {{app_url}}/admin/config/category/security
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.config" isCollection

# List configuration by category - Cache
GET {{app_url}}/admin/config/category/cache
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.config" isCollection

# List configuration by category - Rate Limit
GET {{app_url}}/admin/config/category/rate_limit
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.config" isCollection

# Invalid category returns error
GET {{app_url}}/admin/config/category/invalid
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 400
[Asserts]
jsonpath "$.error.type" exists

# Get specific config key
GET {{app_url}}/admin/config/log_retention_days
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.key" == "log_retention_days"
jsonpath "$.value" isCollection
jsonpath "$.category" exists
jsonpath "$.updated_at" exists

# Get non-existent config key returns not found
GET {{app_url}}/admin/config/nonexistent_key
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 404
[Asserts]
jsonpath "$.error.type" exists

# Update config value - integer type
PUT {{app_url}}/admin/config/log_retention_days
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "value": {
        "type": "integer",
        "value": 60
    }
}
HTTP 200
[Asserts]
jsonpath "$.key" == "log_retention_days"
jsonpath "$.value.type" == "integer"
jsonpath "$.value.value" == 60

# Get updated config value
GET {{app_url}}/admin/config/log_retention_days
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.key" == "log_retention_days"
jsonpath "$.value.type" == "integer"
jsonpath "$.value.value" == 60

# Update config value - boolean type
PUT {{app_url}}/admin/config/execution_logging_enabled
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "value": {
        "type": "boolean",
        "value": true
    }
}
HTTP 200
[Asserts]
jsonpath "$.key" == "execution_logging_enabled"
jsonpath "$.value.type" == "boolean"
jsonpath "$.value.value" == true

# Update config value - string type
PUT {{app_url}}/admin/config/default_timezone
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "value": {
        "type": "string",
        "value": "America/New_York"
    }
}
HTTP 200
[Asserts]
jsonpath "$.key" == "default_timezone"
jsonpath "$.value.type" == "string"
jsonpath "$.value.value" == "America/New_York"

# Reset configuration to defaults
DELETE {{app_url}}/admin/config
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.config" isCollection

# Test authentication required for config endpoints
GET {{app_url}}/admin/config
HTTP 401
[Asserts]
jsonpath "$.error.type" exists

# Test authentication required for category endpoint
GET {{app_url}}/admin/config/category/general
HTTP 401
[Asserts]
jsonpath "$.error.type" exists

# Test authentication required for get config
GET {{app_url}}/admin/config/log_retention_days
HTTP 401
[Asserts]
jsonpath "$.error.type" exists

# Test authentication required for update config
PUT {{app_url}}/admin/config/log_retention_days
Content-Type: application/json
{
    "value": {
        "type": "integer",
        "value": 30
    }
}
HTTP 401
[Asserts]
jsonpath "$.error.type" exists

# Test authentication required for reset config
DELETE {{app_url}}/admin/config
HTTP 401
[Asserts]
jsonpath "$.error.type" exists
