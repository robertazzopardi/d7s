# d7s project commands
# Run `just` or `just --list` to see all recipes

default:
    @just --list

# Build the project
build:
    cargo build

# Build release
build-release:
    cargo build --release

# Run the application
run:
    cargo run

# Run tests
test:
    cargo test --all-features --all-targets

# Format with nightly rustfmt (rustfmt.toml uses nightly-only options: group_imports, imports_granularity).
# Uses nightly then returns to stable — use RUST_NIGHTLY_BIN in nix shell, or rustup run nightly otherwise.
fmt *ARGS:
    #!/usr/bin/env bash
    set -e
    if [ -n "${RUST_NIGHTLY_BIN:-}" ]; then
        PATH="${RUST_NIGHTLY_BIN}:$PATH" cargo fmt -- {{ARGS}}
    else
        rustup run nightly cargo fmt -- {{ARGS}}
    fi

# Check formatting (nightly rustfmt, no write)
fmt-check *ARGS:
    #!/usr/bin/env bash
    set -e
    if [ -n "${RUST_NIGHTLY_BIN:-}" ]; then
        PATH="${RUST_NIGHTLY_BIN}:$PATH" cargo fmt -- --check {{ARGS}}
    else
        rustup run nightly cargo fmt -- --check {{ARGS}}
    fi

# Clippy with pedantic, nursery, cargo and all lints enabled
clippy:
    cargo clippy --workspace --all-features --all-targets -- -D warnings -W clippy::all -W clippy::pedantic -W clippy::nursery # -W clippy::cargo

# Clippy and apply fixes where possible
clippy-fix:
    cargo clippy --workspace --all-features --all-targets --fix --allow-dirty -- -D warnings -W clippy::all -W clippy::pedantic -W clippy::nursery # -W clippy::cargo

# Code coverage via llvm-cov (requires cargo-llvm-cov and llvm-tools)
cov:
    cargo llvm-cov --all-features --all-targets

# Coverage report (terminal)
cov-report:
    cargo llvm-cov report --all-features --all-targets

# Coverage as HTML (opens in browser or inspect lcov-report/)
cov-html:
    cargo llvm-cov html --all-features --all-targets
    @echo "Open target/llvm-cov/html/index.html"

# LCOV report for CI / tooling
cov-lcov:
    cargo llvm-cov lcov --all-features --all-targets --output-path lcov.info

# Full check: format, clippy, test
check: fmt-check clippy test
    @echo "All checks passed"

# Docker: start database services
docker-up:
    docker compose up -d

# Docker: stop services
docker-down:
    docker compose down

# Docker: view logs
docker-logs:
    docker compose logs -f

# Tag and push a release (e.g. `just release 0.1.0`)
# This pushes a `v`-prefixed tag, which triggers the GitHub Actions release workflow.
release VERSION:
    git tag v{{VERSION}}
    git push origin v{{VERSION}}
