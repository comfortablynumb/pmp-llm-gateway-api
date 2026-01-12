# Admin Teams Endpoints
# Test team management

# List all teams (should have at least administrators team)
GET {{app_url}}/admin/teams
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.teams" isCollection
jsonpath "$.teams" count >= 1
jsonpath "$.total" >= 1

# Get the administrators team
GET {{app_url}}/admin/teams/administrators
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.id" == "administrators"
jsonpath "$.name" == "Administrators"
jsonpath "$.status" == "active"

# Create a new team
POST {{app_url}}/admin/teams
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "id": "test-team",
    "name": "Test Team",
    "description": "A test team for integration tests"
}
HTTP 200
[Asserts]
jsonpath "$.id" == "test-team"
jsonpath "$.name" == "Test Team"
jsonpath "$.description" == "A test team for integration tests"
jsonpath "$.status" == "active"

# Get the created team
GET {{app_url}}/admin/teams/test-team
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.id" == "test-team"
jsonpath "$.name" == "Test Team"
jsonpath "$.status" == "active"

# Update the team
PUT {{app_url}}/admin/teams/test-team
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "name": "Updated Test Team",
    "description": "Updated description"
}
HTTP 200
[Asserts]
jsonpath "$.id" == "test-team"
jsonpath "$.name" == "Updated Test Team"
jsonpath "$.description" == "Updated description"

# Suspend the team
POST {{app_url}}/admin/teams/test-team/suspend
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.status" == "suspended"

# Activate the team
POST {{app_url}}/admin/teams/test-team/activate
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.status" == "active"

# Delete the team
DELETE {{app_url}}/admin/teams/test-team
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 200
[Asserts]
jsonpath "$.deleted" == true

# Verify deletion
GET {{app_url}}/admin/teams/test-team
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 404

# Cannot suspend administrators team
POST {{app_url}}/admin/teams/administrators/suspend
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 400
[Asserts]
jsonpath "$.error.message" contains "Cannot suspend the administrators team"

# Cannot delete administrators team
DELETE {{app_url}}/admin/teams/administrators
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 400
[Asserts]
jsonpath "$.error.message" contains "Cannot delete the administrators team"

# Get non-existent team
GET {{app_url}}/admin/teams/non-existent-team
Authorization: Bearer pk_test_{{admin_api_key}}
HTTP 404
[Asserts]
jsonpath "$.error.message" exists

# Create team with invalid ID
POST {{app_url}}/admin/teams
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "id": "Invalid Team ID!",
    "name": "Test"
}
HTTP 400
[Asserts]
jsonpath "$.error.message" exists

# Create team with empty name
POST {{app_url}}/admin/teams
Authorization: Bearer pk_test_{{admin_api_key}}
Content-Type: application/json
{
    "id": "valid-id",
    "name": ""
}
HTTP 400
[Asserts]
jsonpath "$.error.message" exists
