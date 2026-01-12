# Admin Webhooks Endpoints
# Test webhook management CRUD operations

# List event types
GET {{app_url}}/admin/webhooks/event-types
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.event_types" isCollection
jsonpath "$.event_types" count >= 9
jsonpath "$.event_types[0].name" exists
jsonpath "$.event_types[0].description" exists

# List webhooks (initially empty or existing)
GET {{app_url}}/admin/webhooks
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.webhooks" isCollection

# Create a webhook
POST {{app_url}}/admin/webhooks
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "id": "test-webhook-1",
    "name": "Test Webhook",
    "description": "A test webhook for integration testing",
    "url": "https://example.com/webhook",
    "secret": "test-secret-123",
    "events": ["budget_alert", "budget_exceeded"],
    "max_retries": 3,
    "retry_delay_secs": 60,
    "timeout_secs": 30
}
HTTP 201
[Asserts]
jsonpath "$.id" == "test-webhook-1"
jsonpath "$.name" == "Test Webhook"
jsonpath "$.description" == "A test webhook for integration testing"
jsonpath "$.url" == "https://example.com/webhook"
jsonpath "$.has_secret" == true
jsonpath "$.events" count == 2
jsonpath "$.status" == "active"
jsonpath "$.failure_count" == 0
jsonpath "$.max_retries" == 3
jsonpath "$.retry_delay_secs" == 60
jsonpath "$.timeout_secs" == 30

# Get the created webhook
GET {{app_url}}/admin/webhooks/test-webhook-1
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.id" == "test-webhook-1"
jsonpath "$.name" == "Test Webhook"

# Update webhook
PUT {{app_url}}/admin/webhooks/test-webhook-1
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "name": "Updated Test Webhook",
    "description": "Updated description",
    "url": "https://example.com/webhook/v2",
    "events": ["budget_alert", "workflow_failed"],
    "status": "active",
    "max_retries": 5,
    "retry_delay_secs": 120,
    "timeout_secs": 45
}
HTTP 200
[Asserts]
jsonpath "$.name" == "Updated Test Webhook"
jsonpath "$.description" == "Updated description"
jsonpath "$.url" == "https://example.com/webhook/v2"
jsonpath "$.events" count == 2
jsonpath "$.max_retries" == 5
jsonpath "$.retry_delay_secs" == 120
jsonpath "$.timeout_secs" == 45

# Verify update persisted
GET {{app_url}}/admin/webhooks/test-webhook-1
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.name" == "Updated Test Webhook"

# Get deliveries (should be empty)
GET {{app_url}}/admin/webhooks/test-webhook-1/deliveries
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.deliveries" isCollection

# Get deliveries with pagination
GET {{app_url}}/admin/webhooks/test-webhook-1/deliveries?limit=10&offset=0
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.deliveries" isCollection

# Create another webhook without secret
POST {{app_url}}/admin/webhooks
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "id": "test-webhook-2",
    "name": "Test Webhook No Secret",
    "url": "https://example.com/webhook2",
    "events": ["workflow_succeeded"]
}
HTTP 201
[Asserts]
jsonpath "$.id" == "test-webhook-2"
jsonpath "$.has_secret" == false
jsonpath "$.max_retries" == 3
jsonpath "$.retry_delay_secs" == 60
jsonpath "$.timeout_secs" == 30

# List webhooks should now have at least 2
GET {{app_url}}/admin/webhooks
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.webhooks" count >= 2

# Try to create duplicate webhook
POST {{app_url}}/admin/webhooks
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "id": "test-webhook-1",
    "name": "Duplicate Webhook",
    "url": "https://example.com/duplicate",
    "events": ["budget_alert"]
}
HTTP 409

# Try to create webhook without events (should fail)
POST {{app_url}}/admin/webhooks
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "id": "invalid-webhook",
    "name": "Invalid Webhook",
    "url": "https://example.com/invalid",
    "events": []
}
HTTP 400

# Try to create webhook with invalid URL
POST {{app_url}}/admin/webhooks
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "id": "invalid-url-webhook",
    "name": "Invalid URL Webhook",
    "url": "not-a-valid-url",
    "events": ["budget_alert"]
}
HTTP 400

# Reset webhook (simulate failure recovery)
POST {{app_url}}/admin/webhooks/test-webhook-1/reset
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.failure_count" == 0
jsonpath "$.status" == "active"

# Get non-existent webhook
GET {{app_url}}/admin/webhooks/non-existent-webhook
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 404

# Delete webhook 2
DELETE {{app_url}}/admin/webhooks/test-webhook-2
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 204

# Verify deletion
GET {{app_url}}/admin/webhooks/test-webhook-2
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 404

# Delete webhook 1
DELETE {{app_url}}/admin/webhooks/test-webhook-1
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 204

# Verify deletion
GET {{app_url}}/admin/webhooks/test-webhook-1
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 404
