# d7s

A monorepo of k9s-style terminal UIs, built in Rust with [Ratatui](https://ratatui.rs).

## Why

After discovering [k9s](https://k9scli.io/), I liked its format enough to want it for more than Kubernetes — a keyboard-driven, table-first TUI shell reused across a few small tools.

## Apps

- **[`crates/d7s`](crates/d7s)** — a TUI database client for PostgreSQL and SQLite: connection management, keyring-backed credentials, schema/table/row navigation, and a SQL executor. See its [README](crates/d7s/README.md).
- **[`crates/c8s`](crates/c8s)** — a TUI for Docker containers: list, start/stop/restart/remove, live log tail, and exec shell. See its [README](crates/c8s/README.md).

## Shared widget kit

- **[`crates/k9tui`](crates/k9tui)** — the k9s-style ratatui chrome (theme, tables, modals, top bar, hotkeys) both apps build on, with no dependency back on either. See its [README](crates/k9tui/README.md).

## Building

Requires Rust stable (1.91.0 or later).

```sh
cargo build --release --locked
```

Binaries land at `target/release/{d7s,c8s}`. A `flake.nix` is provided (`nix develop`); `just --list` shows common tasks.

## Installing a specific app

```sh
cargo install d7s --locked
```

`c8s` is not yet published to crates.io — build it from source: `cargo build --release --locked -p c8s`.

See each app's README for usage, hotkeys, and demos.
