# d7s

A TUI database client for PostgreSQL and SQLite, built in Rust with [Ratatui](https://ratatui.rs) and inspired by [k9s](https://k9scli.io/).

## Why

After discovering k9s, I thought it had the perfect format for a database client and I wanted something simpler than the established solutions.

## Features

- **Multi-db Support** — currently supports PostgreSQL and SQLite, with more to come! 
- **Connection management** — save, edit, and delete named connections.
- **Credential storage** — passwords are stored in the platform keyring (macOS Keychain, Windows Credential Manager, Linux Secret Service), or never saved and prompted everytime.
- **Database traversal** — navigate databases, schemas, tables, columns, and row data with keyboard-driven menus, supports vim.
- **SQL executor** — run arbitrary SQL queries and view results as a table.
- **Environment tagging** — label each connection as dev, staging, or prod.

## Install

### crates.io

Requires Rust stable (1.91.0 or later).

```sh
cargo install d7s
```

The `d7s` binary will be placed in `$CARGO_HOME/bin` (usually `~/.cargo/bin`), which should already be on your `PATH`.

### Building from source

Requires Rust stable (1.91.0 or later).

```sh
cargo build --release
```

The binary will be at `target/release/d7s`.

### Nix

A `flake.nix` is provided. Enter the development shell:

```sh
nix develop
```

Then use `just` for common tasks (`just --list`).

## Usage

If installed via `cargo install`, just run:

```sh
d7s
```

Or, if built from source:

```sh
cargo run --release
# or, after building:
./target/release/d7s
```
