#!/bin/bash
# Load test runner script
# Usage: bin/load-test.sh [health|chat|all] [base_url] [api_key]

set -e

TEST="${1:-health}"
BASE_URL="${2:-http://localhost:8080}"
API_KEY="${3:-test-api-key}"

echo ""
echo "=== LLM Gateway Load Tests ==="
echo ""
echo "Test: $TEST"
echo "Base URL: $BASE_URL"
echo ""

run_health() {
    echo "Running health endpoint load tests..."
    docker run --rm -i --network=host \
        -e BASE_URL="$BASE_URL" \
        grafana/k6 run - < tests/load/health.js
}

run_chat() {
    echo "Running chat completions load tests..."
    docker run --rm -i --network=host \
        -e BASE_URL="$BASE_URL" \
        -e API_KEY="$API_KEY" \
        grafana/k6 run - < tests/load/chat.js
}

case "$TEST" in
    health)
        run_health
        ;;
    chat)
        run_chat
        ;;
    all)
        echo "Running all load tests..."
        echo ""
        echo "--- Health Tests ---"
        run_health
        echo ""
        echo "--- Chat Tests ---"
        run_chat
        ;;
    *)
        echo "Usage: bin/load-test.sh [health|chat|all] [base_url] [api_key]"
        echo ""
        echo "Examples:"
        echo "  bin/load-test.sh health"
        echo "  bin/load-test.sh chat http://localhost:8080 my-api-key"
        echo "  bin/load-test.sh all http://localhost:3000"
        exit 1
        ;;
esac
