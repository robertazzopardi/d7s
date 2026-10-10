# Workspace commands (d7s, c8s, k9tui)
# Run `just` or `just --list` to see all recipes

default:
    @just --list

# Build the project
build:
    cargo build

# Build release
build-release:
    cargo build --release --locked

# Run the application
run:
    cargo run

# Run tests
test:
    cargo test --workspace --locked --all-features --all-targets

# Format with nightly rustfmt (rustfmt.toml uses nightly-only options: group_imports, imports_granularity).
# Uses RUST_NIGHTLY_BIN in nix shell, or the pinned rustup nightly otherwise.
fmt *ARGS:
    #!/usr/bin/env bash
    set -e
    if [ -n "${RUST_NIGHTLY_BIN:-}" ]; then
        PATH="${RUST_NIGHTLY_BIN}:$PATH" cargo fmt -- {{ARGS}}
    else
        rustup run nightly-2026-09-08 cargo fmt -- {{ARGS}}
    fi

# Check formatting (pinned nightly rustfmt, no write)
fmt-check *ARGS: (fmt "--check" ARGS)

# Clippy: same flags as CI. CI uses the latest stable (no rust-toolchain file), so run `rustup update stable` often or new lints will be missed locally.
clippy *ARGS:
    cargo clippy --workspace --all-features --all-targets {{ARGS}} -- -D warnings -W clippy::all -W clippy::pedantic -W clippy::nursery

# Clippy and apply fixes where possible
clippy-fix: (clippy "--fix" "--allow-dirty")

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

# Demo: isolated HOME + sample DBs for vhs (does not touch real connections)
demo-prep:
    #!/usr/bin/env bash
    set -euo pipefail
    cd crates/d7s
    rm -rf .demo-home .demo-data
    mkdir -p ".demo-home/Library/Application Support/d7s" .demo-data
    sqlite3 .demo-data/sample.db < demo/sample.sql
    sqlite3 ".demo-home/Library/Application Support/d7s/d7s.db" < demo/connections.sql
    test "$(sqlite3 ".demo-home/Library/Application Support/d7s/d7s.db" 'SELECT count(*) FROM connections;')" = "2"
    sqlite3 .demo-data/sample.db "SELECT count(*) FROM users;" | grep -qx 4

# Demo: prep, build release binary, record demo.gif
demo: demo-prep build-release
    cd crates/d7s && vhs demo.tape

# c8s demo: start fake containers, build, record demo.gif, tear down
c8s-demo: c8s-demo-prep
    cargo build --release --locked -p c8s
    cd crates/c8s && vhs demo.tape
    just c8s-demo-teardown

# c8s demo: start fake demo-web/demo-cache containers
c8s-demo-prep:
    docker compose -f crates/c8s/demo/docker-compose.yml up -d

# c8s demo: remove fake demo-web/demo-cache containers
c8s-demo-teardown:
    docker compose -f crates/c8s/demo/docker-compose.yml down

# Docker: start database services
docker-up:
    cd crates/d7s && docker compose up -d

# Docker: stop services
docker-down:
    cd crates/d7s && docker compose down

# Docker: view logs
docker-logs:
    cd crates/d7s && docker compose logs -f

# Publish order (d7s and c8s depend on k9tui):
#   just release k9tui 0.1.0   (tag k9tui-v0.1.0, no workflow)
#   just release d7s 0.5.0     (tag v0.5.0, triggers the d7s binary release workflow)
#   just release c8s 0.1.0     (tag c8s-v0.1.0, no workflow)
# Prereq: cargo install cargo-edit, jq
# Release one crate: bump version, commit, tag, push, publish to crates.io
release CRATE VERSION:
    #!/usr/bin/env bash
    set -euo pipefail

    case "{{CRATE}}" in
        d7s) TAG="v{{VERSION}}" ;;
        c8s|k9tui) TAG="{{CRATE}}-v{{VERSION}}" ;;
        *) echo "Error: CRATE must be one of: k9tui, d7s, c8s"; exit 1 ;;
    esac

    # Ensure working tree is clean
    if ! git diff --quiet || ! git diff --cached --quiet; then
        echo "Error: working tree is not clean. Commit or stash changes first."
        exit 1
    fi

    # d7s and c8s need their k9tui dependency already on crates.io
    if [ "{{CRATE}}" != "k9tui" ]; then
        K9=$(cargo metadata --no-deps --format-version 1 | jq -r '.packages[] | select(.name=="k9tui") | .version')
        if ! curl -sf -A "d7s-release" "https://crates.io/api/v1/crates/k9tui/$K9" >/dev/null; then
            echo "Error: k9tui $K9 is not on crates.io yet. Run: just release k9tui $K9"
            exit 1
        fi
    fi

    # Require a changelog entry (k9tui has no changelog file)
    CL=crates/{{CRATE}}/CHANGELOG.md
    if [ -f "$CL" ]; then
        if ! grep -q "\[{{VERSION}}\]" "$CL"; then
            echo "No CHANGELOG.md entry found for [{{VERSION}}]. Opening for editing..."
            ${EDITOR:-vi} "$CL"
            grep -q "\[{{VERSION}}\]" "$CL" || { echo "Error: $CL still has no entry for [{{VERSION}}]. Aborting."; exit 1; }
        fi
    fi

    # Bump only this crate's version
    cargo set-version -p {{CRATE}} {{VERSION}}

    # Commit and tag
    git add crates/{{CRATE}}/Cargo.toml Cargo.lock
    if [ -f "$CL" ]; then git add "$CL"; fi
    git commit -m "chore: release {{CRATE}} {{VERSION}}"
    git tag "$TAG"
    git push origin HEAD --follow-tags

    echo "Pushed $TAG: publishing {{CRATE}}..."
    cargo publish -p {{CRATE}}

    # Wait for the crates.io index so dependents (d7s, c8s) can resolve it
    for _ in $(seq 1 30); do
        if curl -sf -A "d7s-release" "https://crates.io/api/v1/crates/{{CRATE}}/{{VERSION}}" >/dev/null; then break; fi
        sleep 5
    done

    if [ "{{CRATE}}" = "d7s" ]; then
        echo "Released $TAG: GitHub Actions will build and publish d7s binaries."
    else
        echo "Released $TAG (no GitHub workflow runs for this tag)."
    fi
