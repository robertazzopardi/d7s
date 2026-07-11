#!/bin/sh
ROOT="$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)"
cd "$ROOT/.demo-data" || exit 1
export D7S_DEMO=1
export D7S_DEMO_SQL="$ROOT/demo/next.sql"
export HOME="$ROOT/.demo-home"
export EDITOR="$ROOT/demo/editor.sh"
export VISUAL="$ROOT/demo/editor.sh"
exec "$ROOT/target/release/d7s"
