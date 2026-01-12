#!/bin/bash
# Run Playwright E2E tests against the running application

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
E2E_DIR="$PROJECT_ROOT/resources/e2e"

# Default values
BASE_URL="${BASE_URL:-http://localhost:8080}"
HEADED="${HEADED:-false}"

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --headed)
            HEADED=true
            shift
            ;;
        --url)
            BASE_URL="$2"
            shift 2
            ;;
        *)
            echo "Unknown option: $1"
            echo "Usage: $0 [--headed] [--url <base_url>]"
            exit 1
            ;;
    esac
done

cd "$E2E_DIR"

# Install dependencies if node_modules doesn't exist
if [ ! -d "node_modules" ]; then
    echo "Installing dependencies..."
    npm install
    npx playwright install chromium
fi

# Run tests
echo "Running E2E tests against $BASE_URL..."
export BASE_URL

if [ "$HEADED" = true ]; then
    npm run test:headed
else
    npm test
fi
