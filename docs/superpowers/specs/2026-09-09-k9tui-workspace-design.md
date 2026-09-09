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

## Addendum: top bar + modal generalization

k9s's chrome — top bar, modal dialogs — is not d7s-specific. This
addendum extracts the remaining generic UI shells identified after the
initial extraction.

### TopBarView

`crates/d7s/src/ui/widgets/top_bar_view.rs` couples only through
`&Connection` (for `summary_stack()`) and `crate::db::connection::
shorten_home_path`. Change `TopBarView.current_connection: &Connection`
to `summary: &str` (caller passes `connection.summary_stack()`), and
inline `shorten_home_path`'s path-shortening into the widget itself
(it's a generic "shorten a long value for a labeled line" concern, not
DB-specific). Move the whole widget to `k9tui::widgets::top_bar`.

### Modal (`crates/d7s/src/ui/widgets/modal.rs`, 2706 lines)

Audited every modal type in the file. Three are already generic modulo
naming/dead fields; the rest are genuinely d7s-specific (Connection
forms, SQL text, `DbRowId` cell edits):

**Move to k9tui, generalized:**

1. `ModalField` (labeled `TextArea` input, focus/blur styling, dropdown
   options) — already fully generic. Move as-is to
   `k9tui::widgets::modal::ModalField`.
2. `ConfirmDialog` — new type replacing the near-duplicate
   `ConfirmationModal` and `SqlExecutionConfirmationModal` (identical
   render/key-handling, differ only in title text, border style, and an
   unused `connection: Option<Connection>` field on `ConfirmationModal`
   that nothing reads). Fields: `is_open`, `selected_button`, `message`,
   `title: String`, `border_style: Style`. d7s call sites pass
   `theme::modal_danger_border()` / `modal_confirm_border()` and the
   appropriate title string.
3. `TextPromptModal` — new type generalizing `JumpToRowModal` (already
   fully generic: prompt-less numeric text input + OK/Cancel) and
   `PasswordModal` minus its `connection: Option<Connection>` field
   (carried only so callers could recover which connection triggered
   the prompt — replace with a generic `on_submit`-style caller-side
   correlation, i.e. d7s keeps a `pending_connection: Option<Connection>`
   next to its `TextPromptModal` instance instead of inside it). Fields:
   `is_open`, `input: TextArea`, `prompt: Option<String>`, `masked:
   bool`, `title`, `selected_button`. d7s uses one instance configured
   with `masked: true` for passwords, another with `masked: false` and
   numeric-only parsing left to the caller (`row_number()` becomes a
   d7s-side helper reading `input_value()`).

**Stays in d7s:**

- `Modal` / `ModalManager` (giant struct holding every modal's state +
  `ModalType`/`Mode` enums + the `Connection`-form-specific field-list
  building, validation, and the 830-line `impl Modal` blocks at
  319/1029/1155) — this is d7s's screen-flow orchestration, not a
  reusable widget.
- `ConnectionModalWidget`, `CellValueModal`, `CellValueApply`,
  `PasswordStorageType` — all directly typed on `Connection`/`DbRowId`.
- `SqlQuerySelectionModal` — deferred. It's a generic "pick one of N
  strings" list dialog and a reasonable future k9tui candidate, but
  nothing else in this pass needs it and its `submitted` flag pattern
  differs enough from the other two dialogs to want its own review; not
  worth stalling this pass on.

### Non-goals

- No change to `Modal`/`ModalManager`'s orchestration structure.
- No behavior change: rendering, key handling, and layout stay pixel-
  identical; only code location and, for `ConfirmDialog`/
  `TextPromptModal`, the dedup of two copy-pasted widgets into one.
