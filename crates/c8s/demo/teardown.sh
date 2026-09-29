#!/bin/sh
set -eu
ROOT="$(CDPATH= cd -- "$(dirname "$0")" && pwd)"
docker compose -f "$ROOT/docker-compose.yml" down
echo "demo-web and demo-cache removed"
