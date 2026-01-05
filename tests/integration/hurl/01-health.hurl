# Health Check Endpoints
# Test that the health endpoint returns OK

# Health check - basic endpoint (returns JSON)
GET {{app_url}}/health
HTTP 200
[Asserts]
header "content-type" contains "application/json"
jsonpath "$.status" == "healthy"
jsonpath "$.version" exists

# Health check - ready endpoint (returns 200 with empty body)
GET {{app_url}}/ready
HTTP 200

# Health check - live endpoint (returns 200 with empty body)
GET {{app_url}}/live
HTTP 200
