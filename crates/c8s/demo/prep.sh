#!/bin/sh
# Starts fake, clearly-named containers for the demo recording so it never
# shows real local containers. Run `just c8s-demo-teardown` after.
set -eu
ROOT="$(CDPATH= cd -- "$(dirname "$0")" && pwd)"
docker compose -f "$ROOT/docker-compose.yml" up -d
echo "demo-web and demo-cache started"
