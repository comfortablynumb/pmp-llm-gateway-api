#!/bin/bash

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Default to "full" profile if none specified
PROFILE=${1:-full}

# Run down first
"$SCRIPT_DIR/down.sh" "$PROFILE"

docker compose --profile "$PROFILE" up
