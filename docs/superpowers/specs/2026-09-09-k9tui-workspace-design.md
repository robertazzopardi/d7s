# k9tui workspace extraction

## Goal

Extract d7s's k9s-style ratatui theme/widgets into a standalone reusable
crate (`k9tui`), and turn this repo into a cargo workspace that can host
d7s plus future TUI apps built on the same widget kit (e.g. a lazydocker-
style container viewer).

## Layout

```
Cargo.toml          # [workspace] only, members = ["crates/*"]
crates/
  k9tui/             # new lib crate: k9s-style ratatui widget kit
  d7s/                # current crate, moved as-is minus extracted bits
```

Root `Cargo.toml` becomes a pure workspace manifest (`[workspace]`,
`resolver = "2"`, `members = ["crates/*"]`, shared `[workspace.dependencies]`
for ratatui/crossterm/etc). `d7s`'s existing package metadata (bundle id,
keywords, categories, include list) moves to `crates/d7s/Cargo.toml`.

## What moves into `k9tui`

- `theme.rs` minus the `env_style`/`Environment` coupling — generic color
  palette, borders, base `Style` helpers.
- `widgets/buttons.rs`, `help_view.rs`, `hotkey.rs`, `hotkey_view.rs`,
  `status_line.rs`, `table.rs`, `text_input.rs`.
- The `TableData` trait (currently `crate::db::TableData`) moves into
  `k9tui::widgets::table` as a generic row trait, along with
  `constraint_len_calculator`. d7s's DB row types implement it.

`k9tui` gets its own `Cargo.toml`, README, MIT license — structured as if
publishable, even though it starts as a path dependency.

## What stays in `d7s`

- `modal.rs` (connection-form modal, tied to `db::connection::Connection`,
  `DbRowId` — not a generic modal widget)
- `sql_executor.rs`, `top_bar_view.rs` (both use `db::connection::Connection`)
- `handlers/*` (use `db`/`auth`)
- `env_style` (env-tag colors are d7s domain logic; built on `k9tui::theme`
  primitives)
- everything outside `src/ui`: `app.rs`, `app_state.rs`, `db/`, `auth/`,
  `sql/`, `services/`, `main.rs`

## Dependency direction

`d7s` depends on `k9tui` via workspace path dependency. `k9tui` has no
knowledge of d7s, database types, or auth — it only knows ratatui/crossterm
and its own generic traits (`TableData`, etc).

## Testing

`cargo build --workspace` and `cargo clippy --workspace` after the move.
No behavior change intended — existing d7s tests should pass unchanged,
just relocated to `crates/d7s`.

## Future apps

Additional binaries (e.g. a lazydocker-style container viewer) get added
later as new `crates/<name>` members depending on `k9tui`, following the
same pattern.
