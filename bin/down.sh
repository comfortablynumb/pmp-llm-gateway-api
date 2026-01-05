#!/bin/bash

PROFILE=${1:-}

if [ -n "$PROFILE" ]; then
    docker compose --profile "$PROFILE" rm -f -s -v
else
    docker compose rm -f -s -v
fi
