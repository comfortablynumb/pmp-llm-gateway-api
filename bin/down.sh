#!/bin/bash

# Default to "full" profile if none specified
PROFILE=${1:-full}

docker compose --profile "$PROFILE" rm -f -s -v
