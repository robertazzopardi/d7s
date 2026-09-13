# k9tui

Reusable [k9s](https://k9scli.io/)-style ratatui widget kit — theme, tables, modals, top bar and hotkey chrome for building keyboard-driven, row-based TUI apps. Extracted from [d7s](https://github.com/robertazzopardi/d7s), which remains its reference consumer.

## What's in it

- `theme` — shared color/style helpers (focus, borders, success/danger states).
- `widgets::table` — `TableData` trait, `TableModel`/`TableViewState`/`TableDataState`, `DataTable` widget for dynamic-column row tables.
- `widgets::navigation` — `TableNavigationHandler`: vim-style (`hjkl`)/arrow row and column navigation with wrapping.
- `widgets::top_bar` — `TopBarView`: title, connection/status summary, hotkey stack.
- `widgets::hotkey` / `widgets::hotkey_view` — `Hotkey` type and the hotkey-list widget shown in the top bar and help screen.
- `widgets::status_line` — bottom status/idle-hint bar.
- `widgets::modal` — `ConfirmDialog`, `TextPromptModal`, `ModalField`: generic confirm/prompt dialogs and form-field building block.
- `widgets::help_view` — `HelpRow` table-row builder for a help screen.
- `widgets::buttons` — modal button row widget.
- `widgets::text_input` — masked/plain single-line text input.

## Using it

Add as a path dependency inside a Cargo workspace:

```toml
[dependencies]
k9tui = { path = "../k9tui" }
```

App-specific behavior (domain types, orchestration, key-handling policy) stays in the consuming crate; k9tui only holds chrome and interaction patterns with no dependency back on any app.

## Status

Pre-1.0, not yet published to crates.io. API has one real consumer (d7s) — expect breaking changes until a second app exercises it. Publishing (crates.io metadata: `repository`, `readme`, `keywords`, `edition.workspace`) is tracked as future work.
