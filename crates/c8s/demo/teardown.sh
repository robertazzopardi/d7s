#!/bin/sh
set -eu
docker rm -f demo-web demo-cache >/dev/null 2>&1 || true
echo "demo-web and demo-cache removed"
