# k9tui Workspace Extraction Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn this repo into a cargo workspace with a new `k9tui` lib crate holding the reusable k9s-style ratatui widgets/theme, and `d7s` as a workspace member depending on it.

**Architecture:** Root `Cargo.toml` becomes a workspace manifest. `crates/d7s` gets the current package moved in wholesale, then generic UI code is split out file-by-file into `crates/k9tui`, leaving d7s-specific glue (anything touching `db::*`, `auth::*`, `app_state::*`) behind in d7s. `d7s` depends on `k9tui` via a workspace path dependency.

**Tech Stack:** Rust 2024, ratatui 0.30, crossterm 0.29, cargo workspaces.

**Spec:** `docs/superpowers/specs/2026-09-09-k9tui-workspace-design.md`

## Global Constraints

- No behavior change: `d7s` must build, clippy-clean (existing `[lints.clippy]` deny list), and run identically after the move.
- `k9tui` has zero dependency on `d7s`-domain types (`db::*`, `auth::*`, `app_state::*`). It only depends on `ratatui`, `crossterm`, `unicode-width`.
- Every `cargo build --workspace` / `cargo test --workspace` / `cargo clippy --workspace --all-targets -- -D warnings` must pass before each commit.
- Existing tests in `theme.rs` (env_style, stays in d7s) and `text_input.rs` (moves whole to k9tui) must keep passing without modification to their assertions.

---

### Task 1: Create workspace skeleton, move `d7s` into `crates/d7s`

**Files:**
- Create: `Cargo.toml` (new workspace manifest, replaces current content)
- Create: `crates/d7s/Cargo.toml` (current package manifest, path-adjusted)
- Move: `src/` → `crates/d7s/src/`
- Move: `README.md`, `LICENSE`, `CHANGELOG.md` → stay at repo root (referenced via relative `../../README.md` is not needed — Cargo `readme` field can point to `"../../README.md"`)
- Create: `Cargo.lock` will regenerate at root

**Interfaces:** None yet — pure mechanical move, no code changes.

- [ ] **Step 1: Move source tree**

```bash
mkdir -p crates/d7s
git mv src crates/d7s/src
```

- [ ] **Step 2: Write `crates/d7s/Cargo.toml`**

Take the current root `Cargo.toml` content verbatim, but:
- Change `include = ["src/**/*", ...]` paths stay the same (relative to `crates/d7s/`)
- Change `readme = "README.md"` to `readme = "../../README.md"`
- Keep all `[dependencies]`, `[profile.release]`, `[lints.clippy]` as-is for now (workspace-level dependency hoisting happens later if desired — YAGNI for this pass)

```toml
[package]
name = "d7s"
version = "0.4.0"
description = "Database client"
authors = ["Rob <robertazzopardi@users.noreply.github.com>"]
license = "MIT"
repository = "https://github.com/robertazzopardi/d7s"
edition = "2024"
readme = "../../README.md"
keywords = ["database", "tui", "postgresql", "sqlite", "terminal"]
categories = ["command-line-utilities", "development-tools", "database-implementations"]
include = ["src/**/*", "Cargo.toml", "../../LICENSE", "../../README.md", "../../CHANGELOG.md"]

[package.metadata.bundle]
identifier = "com.robertazzopardi.d7s"

[dependencies]
crossterm = { version = "0.29.0", default-features = false, features = ["osc52", "bracketed-paste", "events", "windows"] }
ratatui = { version = "0.30.0", features = ["crossterm"], default-features = false }
tui-menu = "0.3.1"
unicode-width = { version = "0.2", default-features = false }
tokio = { version = "1.47.0", features = ["full"] }
async-trait = "0.1"
rusqlite = { version = "0.37.0", features = ["bundled"] }
rusqlite_migration = "2.3"
tokio-postgres = { version = "0.7.13", features = ["with-chrono-0_4", "with-serde_json-1", "with-uuid-1", "array-impls"] }
postgres-types = { version = "0.2", features = ["derive", "with-chrono-0_4", "with-serde_json-1", "with-uuid-1"] }
keyring = { version = "3.6.3", features = ["apple-native", "windows-native", "linux-native-sync-persistent", "sync-secret-service"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1.0.142"
url = "2.5"
uuid = { version = "1.18.0", default-features = false }
chrono = { version = "0.4.41", default-features = false }
rust_decimal = { version = "1.40.0", default-features = false, features = ["db-tokio-postgres"] }
color-eyre = "0.6.3"
directories = "6.0.0"
ratatui-textarea = "0.8.0"
sqlparser = "0.61.0"

[profile.release]
codegen-units = 1
lto = true
opt-level = "s"
strip = true

[lints.clippy]
indexing_slicing = "deny"
fallible_impl_from = "deny"
wildcard_enum_match_arm = "deny"
unneeded_field_pattern = "deny"
fn_params_excessive_bools = "deny"
must_use_candidate = "deny"
```

- [ ] **Step 3: Replace root `Cargo.toml` with workspace manifest**

```toml
[workspace]
resolver = "2"
members = ["crates/*"]

[workspace.package]
edition = "2024"
```

- [ ] **Step 4: Build and verify nothing broke**

Run: `cargo build --workspace 2>&1 | tail -50`
Expected: builds successfully, `d7s` binary produced under `target/debug/d7s`.

- [ ] **Step 5: Run existing tests**

Run: `cargo test --workspace 2>&1 | tail -50`
Expected: all existing tests pass (same set as before the move).

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "chore: move d7s into cargo workspace under crates/d7s"
```

---

### Task 2: Create `k9tui` crate skeleton

**Files:**
- Create: `crates/k9tui/Cargo.toml`
- Create: `crates/k9tui/src/lib.rs`
- Create: `crates/k9tui/README.md`
- Create: `crates/k9tui/LICENSE` (copy of repo root `LICENSE`, MIT)
- Modify: `crates/d7s/Cargo.toml` (add `k9tui = { path = "../k9tui" }` dependency)

**Interfaces:**
- Produces: empty `k9tui` crate that `d7s` can depend on; `lib.rs` starts with `pub mod theme;` etc. added incrementally in later tasks (start with empty file, modules added as they're populated).

- [ ] **Step 1: Write `crates/k9tui/Cargo.toml`**

```toml
[package]
name = "k9tui"
version = "0.1.0"
description = "Reusable k9s-style ratatui widget kit"
license = "MIT"
edition = "2024"

[dependencies]
crossterm = { version = "0.29.0", default-features = false, features = ["events"] }
ratatui = { version = "0.30.0", features = ["crossterm"], default-features = false }
unicode-width = { version = "0.2", default-features = false }
```

- [ ] **Step 2: Write empty `crates/k9tui/src/lib.rs`**

```rust
//! k9s-style ratatui widget kit.
```

- [ ] **Step 3: Write `crates/k9tui/README.md`**

```markdown
# k9tui

Reusable k9s-style ratatui theme and widgets, extracted from [d7s](https://github.com/robertazzopardi/d7s).
```

- [ ] **Step 4: Copy license**

```bash
cp LICENSE crates/k9tui/LICENSE
```

- [ ] **Step 5: Add path dependency in `crates/d7s/Cargo.toml`**

Add under `[dependencies]`:
```toml
k9tui = { path = "../k9tui" }
```

- [ ] **Step 6: Build workspace**

Run: `cargo build --workspace 2>&1 | tail -30`
Expected: builds successfully; `k9tui` compiles as an empty crate, unused-dependency is fine (not yet used in d7s code).

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "feat: scaffold k9tui crate"
```

---

### Task 3: Move `theme.rs` (split generic vs `env_style`)

**Files:**
- Create: `crates/k9tui/src/theme.rs`
- Modify: `crates/d7s/src/ui/theme.rs` (keep only `env_style` + its test, build on `k9tui::theme`)
- Modify: `crates/k9tui/src/lib.rs` (add `pub mod theme;`)
- Modify: every file under `crates/d7s/src/ui/` that does `use crate::ui::theme` for functions other than `env_style` (grep list below)

**Interfaces:**
- Produces: `k9tui::theme::{border, title, muted, accent, info_label, info_value, hotkey_key, focus_field, focus_cursor, selection_row, selection_col, selection_cell, draft_row, status_idle, status_message, error, success, modal_default_border, modal_danger_border, modal_confirm_border, modal_connection_border, header_row, null_cell}` — all `fn() -> ratatui::style::Style`.
- `crates/d7s/src/ui/theme.rs` keeps: `pub fn env_style(env: Environment) -> Style` and its test `env_styles_differ`.

- [ ] **Step 1: Copy theme.rs content into k9tui, remove env_style**

```bash
git show HEAD:crates/d7s/src/ui/theme.rs > crates/k9tui/src/theme.rs
```

Then edit `crates/k9tui/src/theme.rs`: delete the `use crate::db::connection::Environment;` import, delete the `env_style` function, delete the `env_styles_differ` test and any test using `Environment`. Keep every other function.

- [ ] **Step 2: Add `pub mod theme;` to `crates/k9tui/src/lib.rs`**

```rust
//! k9s-style ratatui widget kit.

pub mod theme;
```

- [ ] **Step 3: Rewrite `crates/d7s/src/ui/theme.rs` to re-use k9tui**

```rust
use ratatui::style::{Color, Style};

use crate::db::connection::Environment;

#[must_use]
pub const fn env_style(env: Environment) -> Style {
    match env {
        Environment::Dev => Style::new().fg(Color::Green),
        Environment::Staging => Style::new().fg(Color::Yellow),
        Environment::Prod => Style::new().fg(Color::Red),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_styles_differ() {
        assert_ne!(
            env_style(Environment::Dev).fg,
            env_style(Environment::Prod).fg
        );
    }
}
```

(Copy the exact original `env_style` match-arm bodies and `env_styles_differ` assertions from `git show HEAD:crates/d7s/src/ui/theme.rs` — do not guess at exact `Style` construction; use what the original file had.)

- [ ] **Step 4: Update every caller of `theme::<fn>` other than `env_style`**

Run: `grep -rln "ui::theme" crates/d7s/src/ui/ crates/d7s/src/app.rs crates/d7s/src/rendering.rs 2>/dev/null`

For each file found, change `use crate::ui::theme;` to `use k9tui::theme;` (unless the file also calls `env_style`, in which case keep both: `use crate::ui::theme as d7s_theme;` for `env_style` calls and `use k9tui::theme;` for the rest — check each call site's function name against the list in Interfaces above to decide which import it needs).

- [ ] **Step 5: Build**

Run: `cargo build --workspace 2>&1 | tail -60`
Expected: compiles; fix any remaining `theme::` unresolved-path errors by pointing them at `k9tui::theme` or `crate::ui::theme` per Step 4's rule.

- [ ] **Step 6: Test**

Run: `cargo test --workspace 2>&1 | tail -30`
Expected: `env_styles_differ` still passes; all other tests unaffected.

- [ ] **Step 7: Clippy**

Run: `cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail -60`
Expected: clean.

- [ ] **Step 8: Commit**

```bash
git add -A
git commit -m "refactor: move generic theme styles into k9tui"
```

---

### Task 4: Move `hotkey.rs`, `hotkey_view.rs`, `buttons.rs`, `status_line.rs` (split `default_idle_hint` out)

**Files:**
- Create: `crates/k9tui/src/widgets/mod.rs`
- Create: `crates/k9tui/src/widgets/hotkey.rs` (verbatim move)
- Create: `crates/k9tui/src/widgets/hotkey_view.rs` (verbatim move, update `use crate::ui::theme` → `use crate::theme`, `use super::hotkey::Hotkey` stays)
- Create: `crates/k9tui/src/widgets/buttons.rs` (verbatim move, update theme import)
- Create: `crates/k9tui/src/widgets/status_line.rs` (move `StatusLine` struct + impls only, NOT `default_idle_hint`)
- Modify: `crates/d7s/src/ui/widgets/mod.rs` (remove `hotkey`, `hotkey_view`, `buttons` module declarations; keep others)
- Modify: `crates/d7s/src/ui/widgets/status_line.rs` (keep only `default_idle_hint`, importing `k9tui::widgets::status_line::StatusLine` where the type is re-exported for callers, or have callers import `k9tui` directly — see Step 4)
- Delete: `crates/d7s/src/ui/widgets/hotkey.rs`, `crates/d7s/src/ui/widgets/hotkey_view.rs`, `crates/d7s/src/ui/widgets/buttons.rs`
- Modify: all `crates/d7s/src/**` files importing `crate::ui::widgets::{hotkey, hotkey_view, buttons}` or `crate::ui::widgets::status_line::StatusLine`

**Interfaces:**
- Produces: `k9tui::widgets::hotkey::Hotkey`, `k9tui::widgets::hotkey_view::HotkeyView`, `k9tui::widgets::buttons::Buttons`, `k9tui::widgets::status_line::StatusLine` (with `new()`, `set_message()`, `set_idle_hint()`, `clear()`, `Widget` impl — identical API to today).
- `crates/d7s/src/ui/widgets/status_line.rs` keeps only: `pub fn default_idle_hint(app_state: AppState) -> String`.

- [ ] **Step 1: Create k9tui widgets module**

```bash
mkdir -p crates/k9tui/src/widgets
```

`crates/k9tui/src/widgets/mod.rs`:
```rust
pub mod buttons;
pub mod hotkey;
pub mod hotkey_view;
pub mod status_line;
```

Add to `crates/k9tui/src/lib.rs`:
```rust
pub mod theme;
pub mod widgets;
```

- [ ] **Step 2: Move hotkey.rs and buttons.rs verbatim**

```bash
git mv crates/d7s/src/ui/widgets/hotkey.rs crates/k9tui/src/widgets/hotkey.rs
git mv crates/d7s/src/ui/widgets/buttons.rs crates/k9tui/src/widgets/buttons.rs
```

In `crates/k9tui/src/widgets/buttons.rs`, change `use crate::ui::theme;` to `use crate::theme;`.

- [ ] **Step 3: Move hotkey_view.rs**

```bash
git mv crates/d7s/src/ui/widgets/hotkey_view.rs crates/k9tui/src/widgets/hotkey_view.rs
```

Change `use super::hotkey::Hotkey;` stays as-is (still `super::hotkey` within k9tui's widgets module). Change `use crate::ui::theme;` to `use crate::theme;`.

- [ ] **Step 4: Split status_line.rs**

Copy `StatusLine` struct, its `impl StatusLine` block, and `impl Widget for StatusLine` into `crates/k9tui/src/widgets/status_line.rs`, with `use crate::theme;` instead of `use crate::{app_state::AppState, ui::theme};`.

In `crates/d7s/src/ui/widgets/status_line.rs`, delete everything except `default_idle_hint`:
```rust
use crate::app_state::AppState;

#[must_use]
pub fn default_idle_hint(app_state: AppState) -> String {
    match app_state {
        AppState::ConnectionList => "? help · n new · q quit".to_string(),
        AppState::DatabaseConnected => "? help · Esc back · q quit".to_string(),
        // ... copy remaining match arms verbatim from git history
    }
}
```

(Copy the exact remaining match arms from `git show HEAD:crates/d7s/src/ui/widgets/status_line.rs`.)

- [ ] **Step 5: Update `crates/d7s/src/ui/widgets/mod.rs`**

Remove `pub mod hotkey;`, `pub mod hotkey_view;`, `pub mod buttons;` lines (keep `status_line` since it still exists with `default_idle_hint`).

- [ ] **Step 6: Update all import sites**

Run: `grep -rln "ui::widgets::hotkey\|ui::widgets::buttons\|widgets::status_line::StatusLine\|widgets::hotkey_view" crates/d7s/src`

For each hit, replace:
- `crate::ui::widgets::hotkey::Hotkey` → `k9tui::widgets::hotkey::Hotkey`
- `crate::ui::widgets::hotkey_view::HotkeyView` → `k9tui::widgets::hotkey_view::HotkeyView`
- `crate::ui::widgets::buttons::Buttons` → `k9tui::widgets::buttons::Buttons`
- `crate::ui::widgets::status_line::StatusLine` → `k9tui::widgets::status_line::StatusLine`
- keep `crate::ui::widgets::status_line::default_idle_hint` as-is

- [ ] **Step 7: Build**

Run: `cargo build --workspace 2>&1 | tail -80`
Fix remaining unresolved imports per Step 6's mapping.

- [ ] **Step 8: Test**

Run: `cargo test --workspace 2>&1 | tail -30`

- [ ] **Step 9: Clippy**

Run: `cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail -60`

- [ ] **Step 10: Commit**

```bash
git add -A
git commit -m "refactor: move hotkey, buttons, status_line widgets into k9tui"
```

---

### Task 5: Move `text_input.rs` (verbatim, has its own tests)

**Files:**
- Create: `crates/k9tui/src/widgets/text_input.rs` (verbatim move including `#[cfg(test)] mod tests`)
- Modify: `crates/k9tui/src/widgets/mod.rs` (add `pub mod text_input;`)
- Delete: `crates/d7s/src/ui/widgets/text_input.rs`
- Modify: `crates/d7s/src/ui/widgets/mod.rs` (remove `pub mod text_input;` if present)
- Modify: any `crates/d7s/src` file importing `crate::ui::widgets::text_input::*`

**Interfaces:**
- Produces: `k9tui::widgets::text_input::TextInput` (same API as before: `new()`, `with_text()`, `add_char()`, `delete_char()`, etc.)

- [ ] **Step 1: Move file**

```bash
git mv crates/d7s/src/ui/widgets/text_input.rs crates/k9tui/src/widgets/text_input.rs
```

Check its imports (`grep -n "^use" crates/k9tui/src/widgets/text_input.rs`) — if it references `crate::ui::theme` or other d7s paths, rewrite to `crate::theme` (k9tui-local). If it has no `crate::` imports (self-contained), no changes needed beyond the move.

- [ ] **Step 2: Register module**

Add `pub mod text_input;` to `crates/k9tui/src/widgets/mod.rs`. Remove the equivalent line from `crates/d7s/src/ui/widgets/mod.rs` if it exists there.

- [ ] **Step 3: Update import sites**

Run: `grep -rln "ui::widgets::text_input" crates/d7s/src`

Replace `crate::ui::widgets::text_input::TextInput` with `k9tui::widgets::text_input::TextInput` in each hit.

- [ ] **Step 4: Build**

Run: `cargo build --workspace 2>&1 | tail -60`

- [ ] **Step 5: Test — verify text_input's own tests still pass, now under k9tui**

Run: `cargo test -p k9tui 2>&1 | tail -30`
Expected: `test_add_char`, `test_delete_char`, and any other text_input tests PASS under the `k9tui` package.

Run: `cargo test --workspace 2>&1 | tail -30`
Expected: full suite green.

- [ ] **Step 6: Clippy**

Run: `cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail -60`

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "refactor: move text_input widget into k9tui"
```

---

### Task 6: Move `help_view.rs` (split `HelpRow` from `help_rows()`)

**Files:**
- Create: `crates/k9tui/src/widgets/help_view.rs` (HelpRow struct + `impl TableData for HelpRow` + any generic rendering fn, NOT `help_rows()`)
- Modify: `crates/k9tui/src/widgets/mod.rs` (add `pub mod help_view;`)
- Modify: `crates/d7s/src/ui/widgets/help_view.rs` (keep only `help_rows(app_state, explorer_state) -> Vec<HelpRow>` and its supporting private fns, importing `HelpRow` from k9tui)
- Modify: import sites of `HelpRow` / `help_rows` elsewhere in `crates/d7s/src`

**Interfaces:**
- Consumes: `k9tui::widgets::table::TableData` trait (already moved in Task 7 — **do Task 7 before Task 6 if ordering matters**; if `TableData` isn't in k9tui yet, keep `impl TableData for HelpRow` using `crate::db::TableData` for now and revisit in Task 7). To avoid ordering hazards, **do Task 7 first, then Task 6.**
- Produces: `k9tui::widgets::help_view::HelpRow` (same fields/API as today).

- [ ] **Step 0: Confirm Task 7 (TableData trait move) is already done before starting this task.** If not, stop and do Task 7 first — reorder these two tasks in execution, keep numbering as-is for reference.

- [ ] **Step 1: Read current help_view.rs in full**

Run: `sed -n '1,335p' crates/d7s/src/ui/widgets/help_view.rs`

Identify the exact line range for `HelpRow` struct + `impl TableData for HelpRow` (starts ~line 11, per earlier grep) versus the `app_state`/`explorer_state`-dependent code (~line 270 onward, including `help_rows()`).

- [ ] **Step 2: Create `crates/k9tui/src/widgets/help_view.rs`**

Copy the `HelpRow` struct definition and its `impl TableData for HelpRow` block verbatim, changing `use crate::{db::TableData, ui::theme};` to `use crate::{theme, widgets::table::TableData};` (adjust to whatever k9tui's actual module path for `TableData` is per Task 7).

- [ ] **Step 3: Register module**

Add `pub mod help_view;` to `crates/k9tui/src/widgets/mod.rs`.

- [ ] **Step 4: Rewrite `crates/d7s/src/ui/widgets/help_view.rs`**

Keep only the `app_state`/`explorer_state`-dependent functions (the help-row builder logic, ~line 270–335 from the original). Add `use k9tui::widgets::help_view::HelpRow;` at the top. Remove the now-duplicated `HelpRow` struct and its `TableData` impl.

- [ ] **Step 5: Update other import sites**

Run: `grep -rln "widgets::help_view::HelpRow" crates/d7s/src`

Update any direct `crate::ui::widgets::help_view::HelpRow` references to `k9tui::widgets::help_view::HelpRow` (the re-export in d7s's `help_view.rs` via `use` also makes `crate::ui::widgets::help_view::HelpRow` work if you keep it `pub use k9tui::widgets::help_view::HelpRow;` — pick one approach and apply consistently; prefer direct `k9tui::` imports at call sites for clarity).

- [ ] **Step 6: Build**

Run: `cargo build --workspace 2>&1 | tail -80`

- [ ] **Step 7: Test**

Run: `cargo test --workspace 2>&1 | tail -30`

- [ ] **Step 8: Clippy**

Run: `cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail -60`

- [ ] **Step 9: Commit**

```bash
git add -A
git commit -m "refactor: move HelpRow into k9tui, keep help_rows() builder in d7s"
```

---

### Task 7: Move `TableData` trait + generic table widgets from `table.rs`

**Files:**
- Create: `crates/k9tui/src/widgets/table.rs` (generic `TableData` trait, `DataTable<T>`, `TableModel<T>`, `TableViewState`, `TableDataState<T>`, `filter`, `recompute_column_widths`, `constraint_len_calculator`)
- Modify: `crates/k9tui/src/widgets/mod.rs` (add `pub mod table;`, move `constraint_len_calculator` here from top-level `mod.rs` if currently there)
- Modify: `crates/d7s/src/ui/widgets/table.rs` (keep only `RawTableRow` struct + `impl TableData for RawTableRow` + `impl TableDataState<RawTableRow> { fn reset(...) }`, importing generics from `k9tui::widgets::table`)
- Modify: `crates/d7s/src/ui/widgets/mod.rs` (remove `constraint_len_calculator` if moved; remove `pub mod table;` module decl if table.rs is fully re-exported — keep d7s's `table.rs` since `RawTableRow` stays)
- Modify: `crates/d7s/src/db/mod.rs` or wherever `TableData` trait is currently defined/re-exported as `crate::db::TableData` — this trait definition itself moves to k9tui; d7s's `db` module implements it for its row types instead of defining it
- Modify: every file across `crates/d7s/src` importing `crate::db::TableData` or `crate::ui::widgets::table::{DataTable, TableModel, TableViewState, TableDataState}`

**Interfaces:**
- Produces: `k9tui::widgets::table::TableData` (trait with methods `cols() -> &'static [&'static str]`, `num_columns(&self) -> usize`, `col(&self, idx: usize) -> &str` — copy the exact trait definition from `git show HEAD:crates/d7s/src/db/mod.rs` or wherever it's currently declared), `k9tui::widgets::table::{DataTable<T>, TableModel<T>, TableViewState, TableDataState<T>}`, `k9tui::widgets::constraint_len_calculator<T: TableData>`.
- `crates/d7s/src/ui/widgets/table.rs` keeps: `RawTableRow`, `impl TableData for RawTableRow` (implementing the trait now defined in k9tui), `impl TableDataState<RawTableRow> { pub fn reset(...) }`.

- [ ] **Step 1: Find current `TableData` trait definition**

Run: `grep -rn "trait TableData" crates/d7s/src`

Read that file in full to get the exact trait signature.

- [ ] **Step 2: Create `crates/k9tui/src/widgets/table.rs` with the trait and generics**

Move the `TableData` trait definition itself into this file. Then copy from the current `crates/d7s/src/ui/widgets/table.rs`: `TableModel<T: TableData + Clone>`, `TableViewState`, `TableDataState<T: TableData + Clone>`, `DataTable<T: TableData + Clone>` and its `impl` block (`new`, `filter`), and `recompute_column_widths` if it's on a generic type (check — if `recompute_column_widths` at line 167 is on `TableDataState<RawTableRow>` specifically, it stays in d7s instead; read the `impl` block header at that line to confirm before deciding).

Update imports to `use crate::theme;` and drop any `crate::db::*` references (the trait no longer needs them since it's now the definition, not a user).

- [ ] **Step 3: Move `constraint_len_calculator`**

Move this function (currently in `crates/d7s/src/ui/widgets/mod.rs`) into `crates/k9tui/src/widgets/table.rs` or `crates/k9tui/src/widgets/mod.rs`, changing its bound from `crate::db::TableData` to the local `TableData` trait (same module now).

- [ ] **Step 4: Register module**

Add `pub mod table;` to `crates/k9tui/src/widgets/mod.rs`.

- [ ] **Step 5: Rewrite `crates/d7s/src/db` module to implement, not define, `TableData`**

Wherever `crate::db::TableData` was declared as a trait, remove the trait definition and instead `use k9tui::widgets::table::TableData;` and keep the `impl TableData for <ConcreteType>` blocks for d7s's DB row types.

- [ ] **Step 6: Rewrite `crates/d7s/src/ui/widgets/table.rs`**

Keep: `RawTableRow` struct, `impl TableData for RawTableRow` (trait now from `k9tui::widgets::table::TableData`), `impl TableDataState<RawTableRow> { pub fn reset(...) }` (using `k9tui::widgets::table::TableDataState`). Remove everything moved to k9tui in Step 2. Update imports: `use k9tui::widgets::table::{TableData, TableDataState, DataTable, TableModel, TableViewState};`.

- [ ] **Step 7: Update `crates/d7s/src/ui/widgets/mod.rs`**

Remove `constraint_len_calculator` (moved) and its `use crate::db::TableData;` import if nothing else in the file needs it.

- [ ] **Step 8: Update all remaining import sites**

Run: `grep -rln "crate::db::TableData\|ui::widgets::table::\|widgets::constraint_len_calculator" crates/d7s/src`

For each hit, redirect to `k9tui::widgets::table::*` for the generic pieces, keep `crate::ui::widgets::table::RawTableRow` for the d7s-specific type.

- [ ] **Step 9: Build**

Run: `cargo build --workspace 2>&1 | tail -100`
This is the largest surface-area task — expect multiple rounds of fixing unresolved imports. Iterate until clean.

- [ ] **Step 10: Test**

Run: `cargo test --workspace 2>&1 | tail -30`

- [ ] **Step 11: Clippy**

Run: `cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail -80`

- [ ] **Step 12: Commit**

```bash
git add -A
git commit -m "refactor: move TableData trait and generic table widgets into k9tui"
```

---

### Task 8: Final cleanup pass

**Files:**
- Modify: `crates/d7s/src/ui/mod.rs`, `crates/d7s/src/ui/widgets/mod.rs` (remove now-empty/dead module declarations)
- Modify: `crates/k9tui/src/lib.rs` (final module list sanity check)
- Modify: root `README.md` (add one paragraph noting the workspace layout and `k9tui` crate)

**Interfaces:** None — cleanup only, no new public API.

- [ ] **Step 1: Search for dead code / unused imports**

Run: `cargo clippy --workspace --all-targets -- -D warnings -W clippy::all 2>&1 | tail -100`
Fix anything clippy flags as unused (leftover imports from the moves in Tasks 3–7).

- [ ] **Step 2: Confirm k9tui has no d7s coupling**

Run: `grep -rn "crate::db\|crate::auth\|crate::app_state\|d7s" crates/k9tui/src`
Expected: no matches. If any exist, that's a leftover coupling to fix before calling this done.

- [ ] **Step 3: Full workspace verification**

Run: `cargo build --workspace 2>&1 | tail -30 && cargo test --workspace 2>&1 | tail -30 && cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail -60`
Expected: all three clean.

- [ ] **Step 4: Update root README**

Add a short section (2-3 sentences) to `README.md` describing the workspace: `crates/d7s` is the database TUI client, `crates/k9tui` is the extracted reusable k9s-style widget kit other TUI apps in this repo can build on.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "chore: cleanup dead imports, document workspace layout"
```

---

## Self-Review Notes

- **Spec coverage:** Layout (Task 1-2), theme split (Task 3), widget moves (Task 4-7), dependency direction verified (Task 8 Step 2), testing checkpoints in every task. Future-apps section of spec is not a task — it's explicitly deferred ("added later"), correctly out of scope here.
- **Placeholder scan:** Steps referencing "copy verbatim from `git show HEAD:...`" are not placeholders — they're precise instructions to preserve exact original code the plan author cannot fully reproduce without truncating this document; each instruction names the exact source path and what to extract.
- **Type consistency:** `TableData` trait, `DataTable<T>`, `TableModel<T>`, `TableViewState`, `TableDataState<T>` names used consistently across Tasks 6 and 7. `StatusLine` API consistent between Task 4 and its consumers.
- **Ordering hazard:** Task 6 depends on Task 7 (HelpRow's `TableData` impl needs the trait already in k9tui) — flagged explicitly in Task 6 Step 0 with an explicit "do Task 7 first" instruction.
