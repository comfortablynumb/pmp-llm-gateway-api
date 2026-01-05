#!/bin/bash

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROFILE=${1:-}

# Run down first
"$SCRIPT_DIR/down.sh" "$PROFILE"

if [ -n "$PROFILE" ]; then
    docker compose --profile "$PROFILE" up -d
else
    docker compose up -d
fi
