# Admin Budget Management Endpoints
# Test CRUD operations for budgets

# List all budgets (initially empty)
GET {{app_url}}/admin/budgets
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.budgets" isCollection

# Create a new monthly budget with api_key_ids
POST {{app_url}}/admin/budgets
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "id": "test-monthly-budget",
    "name": "Test Monthly Budget",
    "description": "A test budget for integration testing",
    "period": "monthly",
    "hard_limit_usd": 100.0,
    "soft_limit_usd": 80.0,
    "api_key_ids": ["test-key-1", "test-key-2"],
    "model_ids": ["gpt-4o"],
    "alert_thresholds": [50, 75, 90]
}
HTTP 200
[Asserts]
jsonpath "$.id" == "test-monthly-budget"
jsonpath "$.name" == "Test Monthly Budget"
jsonpath "$.description" == "A test budget for integration testing"
jsonpath "$.period" == "monthly"
jsonpath "$.hard_limit_usd" == 100.0
jsonpath "$.soft_limit_usd" == 80.0
jsonpath "$.current_usage_usd" == 0.0
jsonpath "$.remaining_usd" == 100.0
jsonpath "$.usage_percent" == 0.0
jsonpath "$.status" == "active"
jsonpath "$.scope" == "specific_api_keys"
jsonpath "$.api_key_ids" isCollection
jsonpath "$.api_key_ids" count == 2
jsonpath "$.team_ids" isCollection
jsonpath "$.team_ids" count == 0
jsonpath "$.model_ids" isCollection
jsonpath "$.model_ids" count == 1
jsonpath "$.alerts" isCollection
jsonpath "$.alerts" count == 3
jsonpath "$.enabled" == true

# Get the created budget
GET {{app_url}}/admin/budgets/test-monthly-budget
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.id" == "test-monthly-budget"
jsonpath "$.name" == "Test Monthly Budget"
jsonpath "$.hard_limit_usd" == 100.0

# Create a daily budget (global scope)
POST {{app_url}}/admin/budgets
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "id": "test-daily-budget",
    "name": "Test Daily Budget",
    "period": "daily",
    "hard_limit_usd": 10.0
}
HTTP 200
[Asserts]
jsonpath "$.id" == "test-daily-budget"
jsonpath "$.period" == "daily"
jsonpath "$.hard_limit_usd" == 10.0
jsonpath "$.scope" == "all_api_keys"

# Create a team-scoped budget
POST {{app_url}}/admin/budgets
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "id": "test-team-budget",
    "name": "Test Team Budget",
    "period": "monthly",
    "hard_limit_usd": 200.0,
    "team_ids": ["team-alpha", "team-beta"]
}
HTTP 200
[Asserts]
jsonpath "$.id" == "test-team-budget"
jsonpath "$.scope" == "teams"
jsonpath "$.team_ids" isCollection
jsonpath "$.team_ids" count == 2
jsonpath "$.api_key_ids" count == 0

# Create a mixed-scope budget (both api_keys and teams)
POST {{app_url}}/admin/budgets
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "id": "test-mixed-budget",
    "name": "Test Mixed Budget",
    "period": "monthly",
    "hard_limit_usd": 300.0,
    "api_key_ids": ["special-key"],
    "team_ids": ["team-gamma"]
}
HTTP 200
[Asserts]
jsonpath "$.id" == "test-mixed-budget"
jsonpath "$.scope" == "mixed"
jsonpath "$.api_key_ids" count == 1
jsonpath "$.team_ids" count == 1

# List budgets by team (team-alpha)
GET {{app_url}}/admin/budgets/by-team/team-alpha
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.budgets" isCollection
# Should include: test-daily-budget (global) and test-team-budget (teams scope)
jsonpath "$.budgets" count >= 2

# List budgets by team (team-gamma)
GET {{app_url}}/admin/budgets/by-team/team-gamma
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.budgets" isCollection
# Should include: test-daily-budget (global) and test-mixed-budget (mixed scope)
jsonpath "$.budgets" count >= 2

# Create a weekly budget
POST {{app_url}}/admin/budgets
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "id": "test-weekly-budget",
    "name": "Test Weekly Budget",
    "period": "weekly",
    "hard_limit_usd": 50.0,
    "soft_limit_usd": 40.0
}
HTTP 200
[Asserts]
jsonpath "$.id" == "test-weekly-budget"
jsonpath "$.period" == "weekly"

# Create a lifetime budget
POST {{app_url}}/admin/budgets
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "id": "test-lifetime-budget",
    "name": "Test Lifetime Budget",
    "period": "lifetime",
    "hard_limit_usd": 1000.0
}
HTTP 200
[Asserts]
jsonpath "$.id" == "test-lifetime-budget"
jsonpath "$.period" == "lifetime"

# List all budgets (should have 6 now: monthly, daily, team, mixed, weekly, lifetime)
GET {{app_url}}/admin/budgets
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.budgets" isCollection
jsonpath "$.budgets" count == 6

# Update a budget
PUT {{app_url}}/admin/budgets/test-monthly-budget
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "name": "Updated Monthly Budget",
    "hard_limit_usd": 150.0,
    "soft_limit_usd": 120.0,
    "enabled": true
}
HTTP 200
[Asserts]
jsonpath "$.name" == "Updated Monthly Budget"
jsonpath "$.hard_limit_usd" == 150.0
jsonpath "$.soft_limit_usd" == 120.0

# Verify update
GET {{app_url}}/admin/budgets/test-monthly-budget
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.name" == "Updated Monthly Budget"
jsonpath "$.hard_limit_usd" == 150.0

# Check budget for a request (should be allowed)
POST {{app_url}}/admin/budgets/check
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "api_key_id": "test-key-1",
    "model_id": "gpt-4o",
    "estimated_cost_usd": 1.0
}
HTTP 200
[Asserts]
jsonpath "$.allowed" == true
jsonpath "$.exceeded_budgets" isCollection
jsonpath "$.exceeded_budgets" count == 0
jsonpath "$.warning_budgets" isCollection
jsonpath "$.estimated_cost_usd" == 1.0

# Check budget with team context (should match team budget)
POST {{app_url}}/admin/budgets/check
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "api_key_id": "unknown-key",
    "team_id": "team-alpha",
    "estimated_cost_usd": 5.0
}
HTTP 200
[Asserts]
jsonpath "$.allowed" == true

# Check budget for a request (different api key, no team)
POST {{app_url}}/admin/budgets/check
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "api_key_id": "unknown-key",
    "estimated_cost_usd": 10.0
}
HTTP 200
[Asserts]
jsonpath "$.allowed" == true

# Reset budget period
POST {{app_url}}/admin/budgets/test-monthly-budget/reset
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.id" == "test-monthly-budget"
jsonpath "$.current_usage_usd" == 0.0
jsonpath "$.status" == "active"

# Disable a budget
PUT {{app_url}}/admin/budgets/test-daily-budget
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "enabled": false
}
HTTP 200
[Asserts]
jsonpath "$.enabled" == false

# Delete budgets
DELETE {{app_url}}/admin/budgets/test-daily-budget
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.deleted" == true
jsonpath "$.id" == "test-daily-budget"

DELETE {{app_url}}/admin/budgets/test-weekly-budget
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.deleted" == true

DELETE {{app_url}}/admin/budgets/test-lifetime-budget
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.deleted" == true

DELETE {{app_url}}/admin/budgets/test-monthly-budget
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.deleted" == true

DELETE {{app_url}}/admin/budgets/test-team-budget
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.deleted" == true

DELETE {{app_url}}/admin/budgets/test-mixed-budget
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.deleted" == true

# Verify deletion
GET {{app_url}}/admin/budgets/test-monthly-budget
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 404
[Asserts]
jsonpath "$.error.message" exists

# List budgets should be empty
GET {{app_url}}/admin/budgets
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.budgets" isCollection
jsonpath "$.budgets" count == 0

# Get non-existent budget
GET {{app_url}}/admin/budgets/non-existent-budget
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 404
[Asserts]
jsonpath "$.error.message" exists

# Test validation: invalid budget period
POST {{app_url}}/admin/budgets
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "id": "invalid-budget",
    "name": "Invalid Budget",
    "period": "invalid-period",
    "hard_limit_usd": 100.0
}
HTTP 400
[Asserts]
jsonpath "$.error.message" contains "Invalid budget period"

# Test validation: missing hard limit (defaults to 0 which is invalid)
POST {{app_url}}/admin/budgets
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "id": "no-limit-budget",
    "name": "No Limit Budget",
    "period": "monthly",
    "hard_limit_usd": 0
}
HTTP 400

# Test validation: soft limit greater than hard limit
POST {{app_url}}/admin/budgets
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "id": "bad-limits-budget",
    "name": "Bad Limits Budget",
    "period": "monthly",
    "hard_limit_usd": 50.0,
    "soft_limit_usd": 100.0
}
HTTP 400
[Asserts]
jsonpath "$.error.message" contains "Soft limit must be less than hard limit"

# Test authentication required for budget endpoints
GET {{app_url}}/admin/budgets
HTTP 401
[Asserts]
jsonpath "$.error.type" exists

POST {{app_url}}/admin/budgets
Content-Type: application/json
{
    "id": "unauth-budget",
    "name": "Unauthorized Budget",
    "period": "monthly",
    "hard_limit_usd": 100.0
}
HTTP 401
[Asserts]
jsonpath "$.error.type" exists

POST {{app_url}}/admin/budgets/check
Content-Type: application/json
{
    "api_key_id": "test-key",
    "estimated_cost_usd": 1.0
}
HTTP 401
[Asserts]
jsonpath "$.error.type" exists
