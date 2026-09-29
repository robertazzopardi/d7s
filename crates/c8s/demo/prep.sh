#!/bin/sh
# Starts fake, clearly-named containers for the demo recording so it never
# shows real local containers. Run `just c8s-demo-teardown` after.
set -eu

docker rm -f demo-web demo-cache >/dev/null 2>&1 || true

docker run -d --name demo-web -p 18080:80 nginx:alpine >/dev/null
docker run -d --name demo-cache redis:alpine >/dev/null

echo "demo-web and demo-cache started"
