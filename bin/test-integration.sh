#!/bin/bash
# Run integration tests using Docker Compose

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_DIR"

echo "Building and starting test services..."
docker compose --profile test up --build -d mock-openai mock-anthropic mock-azure-openai app

echo "Waiting for services to be healthy..."
sleep 5

echo "Running integration tests..."
docker compose --profile test run --rm hurl

EXIT_CODE=$?

echo "Stopping test services..."
docker compose --profile test down

exit $EXIT_CODE
