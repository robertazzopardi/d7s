# k9tui Chrome Generalization Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move d7s's remaining generic k9s-style chrome — the top bar and three reusable modal-dialog shapes — into the `k9tui` crate, deduplicating two copy-pasted dialogs along the way.

**Architecture:** `TopBarView` moves to `k9tui::widgets::top_bar` with its one `Connection` coupling point replaced by a plain `&str` summary. A new `k9tui::widgets::modal` module gains `ModalField` (moved as-is), and two new generalized dialog widgets — `ConfirmDialog` (replaces the near-duplicate `ConfirmationModal`/`SqlExecutionConfirmationModal`) and `TextPromptModal` (replaces `PasswordModal`/`JumpToRowModal`, minus their `connection: Option<Connection>` correlation field, which moves to `ModalManager` in d7s). `d7s::ui::widgets::modal` shrinks to `Modal`/`ModalManager`/`ConnectionModalWidget`/`CellValueModal`/`CellValueApply`/`PasswordStorageType` and adapts its four call-site files to the new types.

**Tech Stack:** Rust, ratatui 0.30, ratatui-textarea 0.8, crossterm 0.29.

**Spec:** `docs/superpowers/specs/2026-09-09-k9tui-workspace-design.md` (see "Addendum: top bar + modal generalization")

## Global Constraints

- No behavior change to rendering, key handling, or layout, EXCEPT: `TextPromptModal`'s generic key handling gives the jump-to-row dialog the same Left/Right arrow-to-button navigation the password dialog already has (originally jump-to-row routed all non-Enter/Tab/BackTab/Esc keys straight into the input, including arrows). This is a deliberate, ruled-acceptable harmonization from the dedup — call it out in the Task 4 commit message, do not treat it as a regression.
- `d7s` depends on `k9tui` via the existing workspace path dependency (`crates/d7s/Cargo.toml` already has `k9tui = { path = "../k9tui", version = "0.1.0" }` — no change needed there).
- `k9tui` gains two new dependencies in this plan: `ratatui-textarea = "0.8.0"` and `crossterm`'s `bracketed-paste` feature is NOT needed in k9tui (paste handling stays a d7s-level concern; k9tui only exposes `handle_paste(&mut self, text: &str)` methods that insert into a `TextArea`, no crossterm paste-event types involved).
- Every crate keeps `[lints] workspace = true`; new k9tui code must satisfy the workspace clippy deny list (`indexing_slicing`, `fallible_impl_from`, `wildcard_enum_match_arm`, `unneeded_field_pattern`, `fn_params_excessive_bools`, `must_use_candidate` — all deny). Add `#[must_use]` to every new public fn/method that returns a value and isn't `fn new(...) -> Self` used purely for side effects.
- `cargo build --workspace`, `cargo clippy --workspace`, and `cargo test --workspace` must all pass after every task.

---

### Task 1: Generalize and move `TopBarView` to k9tui

**Files:**
- Create: `crates/k9tui/src/widgets/top_bar.rs`
- Modify: `crates/k9tui/src/widgets/mod.rs` (add `pub mod top_bar;`)
- Delete: `crates/d7s/src/ui/widgets/top_bar_view.rs`
- Modify: `crates/d7s/src/ui/widgets/mod.rs` (remove `pub mod top_bar_view;`)
- Modify: `crates/d7s/src/rendering.rs` (update import + call site)

**Interfaces:**
- Produces: `k9tui::widgets::top_bar::TopBarView<'a>` with fields `summary: &'a str` (was `current_connection: &'a Connection`), `recent_hotkeys: &'a [Hotkey]`, `hotkeys: &'a [Hotkey]`, `global_hotkeys: &'a [Hotkey]`, `app_name: &'a str`, `build_info: Option<String>`. Implements `ratatui::widgets::Widget`.

- [ ] **Step 1: Find the current call site**

Run: `grep -n "TopBarView" crates/d7s/src/rendering.rs`

Expect one construction site passing `current_connection: &Connection`. Note the exact field values used for `build_info`/`current_connection` — you'll swap `current_connection` for a `summary` string built the same way the old widget built it internally.

- [ ] **Step 2: Create `crates/k9tui/src/widgets/top_bar.rs`**

This is `crates/d7s/src/ui/widgets/top_bar_view.rs` with two changes: `current_connection: &'a Connection` becomes `summary: &'a str`, and the `shorten_home_path` call inside `render_info_stack` is inlined as a private free function (it's a generic "shorten a long value by keeping its tail" helper — nothing home-directory-specific once inlined; drop the `home_dir` shortening semantics only if they were DB-specific, otherwise keep the exact same trimming logic, just renamed and un-exported).

First read the current implementation to copy exactly:

Run: `cat crates/d7s/src/ui/widgets/top_bar_view.rs` and `grep -n "fn shorten_home_path" -A 15 crates/d7s/src/db/connection.rs`

Write `crates/k9tui/src/widgets/top_bar.rs`:

```rust
use crate::{
    theme,
    widgets::hotkey::Hotkey,
    widgets::hotkey_view::HotkeyView,
};
use ratatui::{
    prelude::{Buffer, Constraint, Layout, Rect, Widget},
    text::{Line, Span},
    widgets::Paragraph,
};

/// Flex weights for the four middle segments (info / recent / primary / global hotkeys).
const MAIN_COLUMN_FILLS: [Constraint; 4] = [
    Constraint::Fill(24),
    Constraint::Fill(20),
    Constraint::Fill(36),
    Constraint::Fill(12),
];
// Second row is a blank spacer before the box below — no rule drawn into it.
const ROW_CONSTRAINTS: [Constraint; 2] =
    [Constraint::Fill(1), Constraint::Length(1)];
const MIN_APP_LABEL_WIDTH: u16 = 8;
const APP_LABEL_RIGHT_MARGIN: u16 = 1;

pub struct TopBarView<'a> {
    pub summary: &'a str,
    pub recent_hotkeys: &'a [Hotkey],
    pub hotkeys: &'a [Hotkey],
    pub global_hotkeys: &'a [Hotkey],
    pub app_name: &'a str,
    pub build_info: Option<String>,
}

impl Widget for TopBarView<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let row = Layout::vertical(ROW_CONSTRAINTS)
            .spacing(0)
            .split(area)
            .first()
            .copied()
            .unwrap_or(area);

        let app_name_lines = self.app_name.trim().lines();
        let app_name_width =
            app_name_lines.clone().map(str::len).max().unwrap_or(0);
        let app_label_width = u16::try_from(app_name_width.max(1))
            .unwrap_or(u16::MAX)
            .max(MIN_APP_LABEL_WIDTH);

        let [main_area, app_logo_cell] = Layout::horizontal([
            Constraint::Fill(1),
            Constraint::Length(
                app_label_width.saturating_add(APP_LABEL_RIGHT_MARGIN),
            ),
        ])
        .spacing(1)
        .areas(row);

        let [app_info_cell, recent_cell, hotkey_cell, global_cell] =
            Layout::horizontal(MAIN_COLUMN_FILLS)
                .spacing(1)
                .areas(main_area);

        if let Some(build_info) = &self.build_info {
            render_info_stack(build_info, app_info_cell, buf);
        } else {
            render_info_stack(self.summary, app_info_cell, buf);
        }

        HotkeyView::new(self.recent_hotkeys).render(recent_cell, buf);
        HotkeyView::new(self.hotkeys).render(hotkey_cell, buf);
        HotkeyView::new(self.global_hotkeys).render(global_cell, buf);

        let label_align_width =
            (app_logo_cell.width.saturating_sub(APP_LABEL_RIGHT_MARGIN))
                as usize;
        let padding = label_align_width.saturating_sub(app_name_width);
        let padded = app_name_lines
            .map(|line| {
                format!("{:>width$}", line, width = line.len() + padding)
            })
            .collect::<Vec<_>>()
            .join("\n");
        Paragraph::new(padded)
            .style(theme::border())
            .render(app_logo_cell, buf);
    }
}

/// Shorten a value by keeping its tail, prefixed with `…`, when it exceeds `max_val` chars.
fn shorten_value(val: &str, max_val: usize) -> String {
    if max_val > 1 && val.chars().count() > max_val {
        let keep = max_val.saturating_sub(1);
        format!(
            "…{}",
            val.chars()
                .skip(val.chars().count().saturating_sub(keep))
                .collect::<String>()
        )
    } else {
        val.to_string()
    }
}

fn render_info_stack(text: &str, area: Rect, buf: &mut Buffer) {
    let max_val = area.width.saturating_sub(12) as usize;
    let lines: Vec<Line> = text
        .lines()
        .map(|line| {
            if let Some((label, value)) = line.split_once(':') {
                let val = shorten_value(value.trim(), max_val);
                Line::from(vec![
                    Span::styled(
                        format!("{}:", label.trim()),
                        theme::info_label(),
                    ),
                    Span::raw(" "),
                    Span::styled(val, theme::info_value()),
                ])
            } else {
                Line::from(Span::styled(line.to_string(), theme::muted()))
            }
        })
        .collect();
    Paragraph::new(lines).render(area, buf);
}
```

Note: the original `shorten_home_path` in `crate::db::connection` did tilde/home-relative shortening on the raw value *before* truncation; check its actual body with the grep above. If it does more than tail-truncate (e.g. replaces `/Users/rob` with `~`), keep that as a d7s-side concern: `Connection::summary_stack()` should apply `shorten_home_path` to path-like fields itself when building the summary string in Task 1 Step 4, and `top_bar.rs`'s `shorten_value` here only does the generic tail truncation for width — do not port home-path logic into k9tui.

- [ ] **Step 3: Register the module**

Add `pub mod top_bar;` to `crates/k9tui/src/widgets/mod.rs` (alongside the existing `pub mod` lines, alphabetically after `text_input`).

- [ ] **Step 4: Update the call site in `crates/d7s/src/rendering.rs`**

Change the import from `crate::ui::widgets::top_bar_view::TopBarView` (or wherever it's currently imported from) to `k9tui::widgets::top_bar::TopBarView`. Change the construction site's `current_connection: &self.current_connection` (or equivalent) to `summary: &self.current_connection.summary_stack()` — build the `String` first into a local `let summary = ...;` binding so the borrow lives long enough for the widget construction, then pass `summary: &summary`. If `shorten_home_path` needs folding into `Connection::summary_stack()` per the Step 2 note, do that here in `crates/d7s/src/db/connection.rs`.

- [ ] **Step 5: Delete the old file and its module registration**

```bash
rm crates/d7s/src/ui/widgets/top_bar_view.rs
```

Remove the `pub mod top_bar_view;` line from `crates/d7s/src/ui/widgets/mod.rs`.

- [ ] **Step 6: Build and test**

Run: `cargo build --workspace 2>&1 | tail -50`
Expected: clean build, no errors about missing `top_bar_view` or `Connection` type mismatches.

Run: `cargo clippy --workspace 2>&1 | tail -50`
Expected: no new warnings.

Run: `cargo test --workspace 2>&1 | tail -30`
Expected: all existing tests still pass (this task changes no test files).

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "refactor: move TopBarView to k9tui, decouple from Connection"
```

---

### Task 2: Move `ModalField` to k9tui as-is

**Files:**
- Create: `crates/k9tui/src/widgets/modal.rs`
- Modify: `crates/k9tui/src/widgets/mod.rs` (add `pub mod modal;`)
- Modify: `crates/k9tui/Cargo.toml` (add `ratatui-textarea` dependency)
- Modify: `crates/d7s/src/ui/widgets/modal.rs` (remove `ModalField` definition, import from k9tui instead)

**Interfaces:**
- Consumes: nothing from earlier tasks.
- Produces: `k9tui::widgets::modal::ModalField` — a labeled `TextArea<'static>` wrapper with focus styling and optional dropdown-options constraint. Public API (unchanged from current d7s version): `ModalField::new(label: &'static str) -> Self`, `.value() -> &str`, `.set_value(impl Into<String>)`, `.set_focus(bool)`, `.input_key(KeyEvent)`, `.set_masked()`, `.set_options(Vec<&'static str>)`, `.clamp_to_options()`, `.is_dropdown() -> bool`. Public fields: `label: &'static str`, `input: TextArea<'static>`, `is_focused: bool`, `options: Option<Vec<&'static str>>`.

- [ ] **Step 1: Add the dependency**

In `crates/k9tui/Cargo.toml`, add under `[dependencies]`:

```toml
ratatui-textarea = "0.8.0"
```

Also add `crossterm`'s existing `events` feature already covers `KeyEvent`/`KeyCode` — no crossterm change needed.

- [ ] **Step 2: Read the current `ModalField` definition**

Run: `sed -n '1,161p' crates/d7s/src/ui/widgets/modal.rs`

Copy lines defining `use std::{fmt::Display, str::FromStr};` (only the parts `ModalField` actually needs), `use crossterm::event::{KeyCode, KeyEvent};`, `use k9tui::theme;` (becomes `use crate::theme;` inside k9tui), `ratatui::style::{Color, Style}`, `ratatui_textarea::TextArea`, and the full `ModalField` struct + `impl ModalField` block (originally lines 65–161, i.e. everything from `#[derive(Debug, Clone)] pub struct ModalField` through the closing brace of `impl ModalField`).

- [ ] **Step 3: Create `crates/k9tui/src/widgets/modal.rs`**

```rust
use crate::theme;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::style::{Color, Style};
use ratatui_textarea::TextArea;

#[derive(Debug, Clone)]
pub struct ModalField {
    pub label: &'static str,
    pub input: TextArea<'static>,
    pub is_focused: bool,
    /// When set, this field is a dropdown; value must be one of these options.
    pub options: Option<Vec<&'static str>>,
}

impl ModalField {
    fn make_input(text: &str) -> TextArea<'static> {
        let mut input = TextArea::new(vec![text.to_string()]);
        // Disable cursor line highlight (not needed for single-line form fields)
        input.set_cursor_line_style(Style::default());
        // Hide cursor until focused
        input.set_cursor_style(Style::default());
        // No undo/redo needed for form fields
        input.set_max_histories(0);
        input
    }

    #[must_use]
    pub fn new(label: &'static str) -> Self {
        Self {
            label,
            input: Self::make_input(""),
            is_focused: false,
            options: None,
        }
    }

    /// Get the current text value of this field.
    #[must_use]
    pub fn value(&self) -> &str {
        self.input.lines().first().map_or("", |s| s.as_str())
    }

    /// Set the text value of this field, replacing all content.
    pub fn set_value(&mut self, text: impl Into<String>) {
        let text = text.into();
        self.input = Self::make_input(&text);
        // Move cursor to end of the pre-filled text
        self.input.move_cursor(ratatui_textarea::CursorMove::End);
        // Restore cursor style based on current focus state
        if self.is_focused {
            self.input.set_cursor_style(theme::focus_cursor());
        }
    }

    pub fn set_focus(&mut self, focused: bool) {
        self.is_focused = focused;
        if focused {
            self.input.set_style(theme::focus_field());
            self.input.set_cursor_style(theme::focus_cursor());
        } else {
            self.input.set_style(Style::default().fg(Color::White));
            self.input.set_cursor_style(Style::default());
        }
    }

    pub fn input_key(&mut self, key: KeyEvent) {
        if self.options.is_none() {
            self.input.input(key);
        }
    }

    /// Enable character masking (for password fields).
    pub fn set_masked(&mut self) {
        self.input.set_mask_char('•');
    }

    /// Set dropdown options. If value is empty, sets value to first option.
    pub fn set_options(&mut self, options: Vec<&'static str>) {
        if !options.is_empty() {
            let first = options[0].to_string();
            self.options = Some(options);
            if self.value().is_empty() {
                self.set_value(first);
            }
        }
    }

    /// Ensure value is one of the options. Sets to first option if invalid.
    pub fn clamp_to_options(&mut self) {
        if let Some(ref opts) = self.options
            && !opts.is_empty()
            && !opts.iter().any(|o| *o == self.value())
        {
            self.set_value(opts[0]);
        }
    }

    #[must_use]
    pub const fn is_dropdown(&self) -> bool {
        self.options.is_some()
    }
}
```

This file needs `#![allow(clippy::indexing_slicing)]` at its top only if `opts[0]` trips the workspace lint (the original file has this allow at the file level because other code in it indexes; here the only indexing is `opts[0]` inside a branch already guarded by `!opts.is_empty()`, so clippy's `indexing_slicing` will still flag it since it can't see the guard). Add `#[allow(clippy::indexing_slicing)]` directly above the `set_options` method as a narrow, justified suppression instead of a file-wide one.

- [ ] **Step 4: Register the module**

Add `pub mod modal;` to `crates/k9tui/src/widgets/mod.rs`.

- [ ] **Step 5: Remove `ModalField` from d7s and import from k9tui**

In `crates/d7s/src/ui/widgets/modal.rs`:
- Delete the `ModalField` struct and its `impl` block (the code you copied in Step 2).
- Add `ModalField` to the existing `use k9tui::{...}` import block: `use k9tui::{theme, widgets::{buttons::Buttons, modal::ModalField}};` (adjust to match whatever the existing k9tui import list looks like — just add `modal::ModalField` to the `widgets::{...}` path list).
- Remove now-unused imports from the top of the file if `ModalField`'s removal leaves `std::fmt::Display`/`std::str::FromStr` still used elsewhere (`PasswordStorageType` uses both — check before removing; they likely stay).

- [ ] **Step 6: Build and test**

Run: `cargo build --workspace 2>&1 | tail -50`
Expected: clean build. All `ModalField::new(...)` call sites throughout `modal.rs`'s `impl Modal` blocks resolve unchanged since the type name and API are identical.

Run: `cargo clippy --workspace 2>&1 | tail -50`
Expected: no new warnings.

Run: `cargo test --workspace 2>&1 | tail -30`
Expected: all tests pass.

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "refactor: move ModalField to k9tui"
```

---

### Task 3: Add `ConfirmDialog` and `TextPromptModal` to k9tui

**Files:**
- Modify: `crates/k9tui/src/widgets/modal.rs` (append new types)

**Interfaces:**
- Consumes: `k9tui::widgets::buttons::Buttons`, `k9tui::theme::*` (already available).
- Produces:
  - `k9tui::widgets::modal::DialogAction` enum: `{ None, Submit, Cancel }` — generic outcome of a key event on either dialog.
  - `k9tui::widgets::modal::ConfirmDialog` — replaces d7s's `ConfirmationModal` + `SqlExecutionConfirmationModal`. Constructor: `ConfirmDialog::new(title: impl Into<String>, message: impl Into<String>, border_style: Style, default_button: usize) -> Self`. Methods: `.close()`, `.is_confirmed() -> bool` (true when `selected_button == 0`), `.handle_key_events(KeyEvent) -> DialogAction`. Public fields: `is_open: bool`, `message: String` (kept public so callers can read/update it, matching current `ConfirmationModal.message` visibility). Implements `Widget` (by value, matching the current `impl Widget for ConfirmationModal`).
  - `k9tui::widgets::modal::PromptValidation` enum: `{ NonEmpty, PositiveInteger }`.
  - `k9tui::widgets::modal::TextPromptModal` — replaces d7s's `PasswordModal` + `JumpToRowModal`. Constructor: `TextPromptModal::new(title: impl Into<String>, width: u16, height: u16) -> Self` (defaults: no prompt line, unmasked, `PromptValidation::NonEmpty`, buttons `["OK", "Cancel"]`). Builder methods (consume `self`, return `Self`): `.with_prompt(impl Into<String>)`, `.masked(self)`, `.with_placeholder(&'static str)`, `.with_validation(PromptValidation)`, `.with_buttons(&'static str, &'static str)`. Methods: `.close()`, `.input_value() -> String`, `.parsed_positive_int() -> Option<u64>`, `.clear_input()`, `.handle_key_events(KeyEvent) -> DialogAction`, `.handle_paste(&str)`. Public fields: `is_open: bool`, `submitted: bool`. Implements `Widget`.

- [ ] **Step 1: Read the four originals to confirm exact widths/heights/titles**

Run: `sed -n '1410,1670p' crates/d7s/src/ui/widgets/modal.rs` (ConfirmationModal, SqlExecutionConfirmationModal, their impls and Widget renders — already captured in this conversation, but re-read to be exact about the `CONFIRMATION_MODAL_WIDTH`/`HEIGHT` constants at the top of the file, which are `50`/`8`).

Run: `sed -n '1940,2219p' crates/d7s/src/ui/widgets/modal.rs` (PasswordModal, JumpToRowModal — `PASSWORD_MODAL_WIDTH`/`HEIGHT` are `50`/`8`; `JUMP_TO_ROW_MODAL_WIDTH`/`HEIGHT` are `36`/`7`).

- [ ] **Step 2: Append `DialogAction` and `ConfirmDialog` to `crates/k9tui/src/widgets/modal.rs`**

Add these imports at the top of the file (merge with Task 2's imports):

```rust
use crate::widgets::buttons::Buttons;
use ratatui::{
    prelude::{
        Alignment, Buffer, Constraint, Direction, Layout, Rect, Widget,
    },
    widgets::{Block, Borders, Clear, Paragraph},
};
```

Append:

```rust
/// Outcome of a key event handled by a dialog widget.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogAction {
    None,
    Submit,
    Cancel,
}

const CONFIRM_DIALOG_WIDTH: u16 = 50;
const CONFIRM_DIALOG_HEIGHT: u16 = 8;

/// A yes/no confirmation dialog with a title, message, and configurable border style.
#[derive(Debug, Clone)]
pub struct ConfirmDialog {
    pub is_open: bool,
    pub message: String,
    title: String,
    border_style: Style,
    selected_button: usize,
}

impl ConfirmDialog {
    #[must_use]
    pub fn new(
        title: impl Into<String>,
        message: impl Into<String>,
        border_style: Style,
        default_button: usize,
    ) -> Self {
        Self {
            is_open: true,
            message: message.into(),
            title: title.into(),
            border_style,
            selected_button: default_button,
        }
    }

    pub const fn close(&mut self) {
        self.is_open = false;
    }

    const fn next_button(&mut self) {
        self.selected_button = (self.selected_button + 1) % 2;
    }

    const fn prev_button(&mut self) {
        self.selected_button = (self.selected_button + 1) % 2;
    }

    #[must_use]
    pub const fn is_confirmed(&self) -> bool {
        self.selected_button == 0
    }

    pub fn handle_key_events(&mut self, key: KeyEvent) -> DialogAction {
        match (key.modifiers, key.code) {
            (_, KeyCode::Esc | KeyCode::Enter) => {
                let action = if self.is_confirmed() {
                    DialogAction::Submit
                } else {
                    DialogAction::Cancel
                };
                self.close();
                if key.code == KeyCode::Esc {
                    DialogAction::Cancel
                } else {
                    action
                }
            }
            (_, KeyCode::Left) => {
                self.prev_button();
                DialogAction::None
            }
            (_, KeyCode::Right) => {
                self.next_button();
                DialogAction::None
            }
            _ => DialogAction::None,
        }
    }
}

impl Widget for ConfirmDialog {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if !self.is_open {
            return;
        }

        let x = area.x + (area.width.saturating_sub(CONFIRM_DIALOG_WIDTH)) / 2;
        let y =
            area.y + (area.height.saturating_sub(CONFIRM_DIALOG_HEIGHT)) / 2;
        let modal_area =
            Rect::new(x, y, CONFIRM_DIALOG_WIDTH, CONFIRM_DIALOG_HEIGHT);

        let block = Block::default()
            .title(self.title.clone())
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_style(self.border_style)
            .style(Style::default().bg(Color::Black));
        Clear.render(modal_area, buf);
        block.render(modal_area, buf);

        let inner_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Length(1)])
            .margin(1)
            .split(modal_area);

        let content_layout = *inner_layout.first().unwrap_or(&Rect::ZERO);
        Paragraph::new(self.message)
            .style(Style::default().fg(Color::White))
            .alignment(Alignment::Center)
            .render(content_layout, buf);

        let buttons = Buttons {
            buttons: vec!["Yes", "No"],
            selected: self.selected_button,
        };
        let button_layout = *inner_layout.get(1).unwrap_or(&Rect::ZERO);
        buttons.render(button_layout, buf);
    }
}
```

Note on the `handle_key_events` `Esc`/`Enter` branch above: the original two modals both treated `Esc` and `Enter` identically (just close, then the caller reads `.confirm()` separately) — they never distinguished "confirmed via Enter" from "closed via Esc leaving selected_button wherever it was". Preserve that exactly: **do not** special-case `Esc` to force `DialogAction::Cancel`. Use this simpler, behavior-matching version instead of the branch shown above:

```rust
    pub fn handle_key_events(&mut self, key: KeyEvent) -> DialogAction {
        match (key.modifiers, key.code) {
            (_, KeyCode::Esc | KeyCode::Enter) => {
                let action = if self.is_confirmed() {
                    DialogAction::Submit
                } else {
                    DialogAction::Cancel
                };
                self.close();
                action
            }
            (_, KeyCode::Left) => {
                self.prev_button();
                DialogAction::None
            }
            (_, KeyCode::Right) => {
                self.next_button();
                DialogAction::None
            }
            _ => DialogAction::None,
        }
    }
```

This matches the original semantics: `ConfirmationModal` defaulted `selected_button = 0` (Yes) so bare Esc/Enter without arrow keys confirmed; `SqlExecutionConfirmationModal` defaulted `selected_button = 1` (No) so bare Esc/Enter cancelled. `default_button` in the constructor reproduces both.

- [ ] **Step 3: Append `PromptValidation` and `TextPromptModal` to the same file**

```rust
/// What counts as a submittable value in a [`TextPromptModal`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptValidation {
    NonEmpty,
    PositiveInteger,
}

/// A single-line text-input dialog: optional prompt line, input, OK/Cancel buttons.
#[derive(Debug, Clone)]
pub struct TextPromptModal {
    pub is_open: bool,
    pub submitted: bool,
    input: TextArea<'static>,
    prompt: Option<String>,
    title: String,
    width: u16,
    height: u16,
    validation: PromptValidation,
    buttons: [&'static str; 2],
    selected_button: usize,
}

impl TextPromptModal {
    fn make_input() -> TextArea<'static> {
        let mut input = TextArea::default();
        input.set_cursor_line_style(Style::default());
        input.set_cursor_style(theme::focus_cursor());
        input.set_max_histories(0);
        input
    }

    #[must_use]
    pub fn new(title: impl Into<String>, width: u16, height: u16) -> Self {
        Self {
            is_open: true,
            submitted: false,
            input: Self::make_input(),
            prompt: None,
            title: title.into(),
            width,
            height,
            validation: PromptValidation::NonEmpty,
            buttons: ["OK", "Cancel"],
            selected_button: 0,
        }
    }

    #[must_use]
    pub fn with_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.prompt = Some(prompt.into());
        self
    }

    #[must_use]
    pub fn masked(mut self) -> Self {
        self.input.set_mask_char('•');
        self
    }

    #[must_use]
    pub fn with_placeholder(mut self, placeholder: &'static str) -> Self {
        self.input.set_placeholder_text(placeholder);
        self
    }

    #[must_use]
    pub const fn with_validation(mut self, validation: PromptValidation) -> Self {
        self.validation = validation;
        self
    }

    #[must_use]
    pub const fn with_buttons(
        mut self,
        ok: &'static str,
        cancel: &'static str,
    ) -> Self {
        self.buttons = [ok, cancel];
        self
    }

    /// Get the current text value of the input.
    #[must_use]
    pub fn input_value(&self) -> String {
        self.input.lines().first().cloned().unwrap_or_default()
    }

    /// Parse the current input as a positive integer, if valid.
    #[must_use]
    pub fn parsed_positive_int(&self) -> Option<u64> {
        self.input_value()
            .trim()
            .parse::<u64>()
            .ok()
            .filter(|&n| n > 0)
    }

    pub const fn close(&mut self) {
        self.is_open = false;
    }

    /// Clear the input field back to empty.
    pub fn clear_input(&mut self) {
        self.input = Self::make_input();
    }

    fn can_submit(&self) -> bool {
        match self.validation {
            PromptValidation::NonEmpty => !self.input_value().is_empty(),
            PromptValidation::PositiveInteger => {
                self.parsed_positive_int().is_some()
            }
        }
    }

    pub fn handle_key_events(&mut self, key: KeyEvent) -> DialogAction {
        match (key.modifiers, key.code) {
            (_, KeyCode::Esc) => {
                self.submitted = false;
                self.close();
                DialogAction::Cancel
            }
            (_, KeyCode::Tab | KeyCode::Down) => {
                if self.selected_button == 0 {
                    self.selected_button = 1;
                }
                DialogAction::None
            }
            (_, KeyCode::BackTab | KeyCode::Up) => {
                if self.selected_button == 1 {
                    self.selected_button = 0;
                }
                DialogAction::None
            }
            (_, KeyCode::Left) => {
                if self.selected_button == 1 {
                    self.selected_button = 0;
                } else {
                    self.input.input(key);
                }
                DialogAction::None
            }
            (_, KeyCode::Right) => {
                if self.selected_button == 0 {
                    let line = self.input_value();
                    let (_, col) = self.input.cursor();
                    if col >= line.len() {
                        self.selected_button = 1;
                    } else {
                        self.input.input(key);
                    }
                }
                DialogAction::None
            }
            (_, KeyCode::Enter) => match self.selected_button {
                0 if self.can_submit() => {
                    self.submitted = true;
                    self.close();
                    DialogAction::Submit
                }
                1 => {
                    self.submitted = false;
                    self.close();
                    DialogAction::Cancel
                }
                _ => DialogAction::None,
            },
            _ if self.selected_button == 0 => {
                self.input.input(key);
                DialogAction::None
            }
            _ => DialogAction::None,
        }
    }

    pub fn handle_paste(&mut self, text: &str) {
        if self.selected_button == 0 {
            self.input.insert_str(text);
        }
    }
}

impl Widget for TextPromptModal {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if !self.is_open {
            return;
        }

        let x = area.x + (area.width.saturating_sub(self.width)) / 2;
        let y = area.y + (area.height.saturating_sub(self.height)) / 2;
        let modal_area = Rect::new(x, y, self.width, self.height);

        let block = Block::default()
            .title(self.title.clone())
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_style(theme::modal_confirm_border())
            .style(Style::default().bg(Color::Black));
        Clear.render(modal_area, buf);
        let inner = block.inner(modal_area);
        block.render(modal_area, buf);

        if let Some(prompt) = &self.prompt {
            let [prompt_area, input_area, button_area] = Layout::vertical([
                Constraint::Length(2),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .margin(1)
            .areas(inner);

            Paragraph::new(prompt.clone())
                .style(Style::default().fg(Color::White))
                .alignment(Alignment::Left)
                .render(prompt_area, buf);
            Widget::render(&self.input, input_area, buf);
            Buttons {
                buttons: vec![self.buttons[0], self.buttons[1]],
                selected: self.selected_button,
            }
            .render(button_area, buf);
        } else {
            let [input_area, button_area] = Layout::vertical([
                Constraint::Length(3),
                Constraint::Length(1),
            ])
            .areas(inner);

            Widget::render(&self.input, input_area, buf);
            Buttons {
                buttons: vec![self.buttons[0], self.buttons[1]],
                selected: self.selected_button,
            }
            .render(button_area, buf);
        }
    }
}
```

Note the `PasswordModal` original used `.margin(1)` on its 3-row layout but `JumpToRowModal` used `block.inner(modal_area)` with no extra margin on its 2-row layout — check this against the Step 1 re-read and adjust the `prompt.is_some()` branch's margin to `1` (matching Password) and the `else` branch to no margin (matching JumpToRow) exactly as written above; do not unify the margins if the re-read shows they genuinely differ, since that would be a visual behavior change.

- [ ] **Step 4: Build and test**

Run: `cargo build -p k9tui 2>&1 | tail -50`
Expected: clean build.

Run: `cargo clippy -p k9tui 2>&1 | tail -50`
Expected: no warnings. Pay attention to `fn_params_excessive_bools` (none of the new functions take bool params) and `must_use_candidate` (every non-`&mut self` getter needs `#[must_use]`, already applied above).

Run: `cargo test --workspace 2>&1 | tail -30`
Expected: all pass (nothing wired to d7s yet, so d7s behavior is unchanged).

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat: add ConfirmDialog and TextPromptModal to k9tui"
```

---

### Task 4: Migrate d7s's modal.rs and call sites onto the new k9tui types

**Files:**
- Modify: `crates/d7s/src/ui/widgets/modal.rs` (remove `ConfirmationModal`, `SqlExecutionConfirmationModal`, `PasswordModal`, `JumpToRowModal`; update `ModalManager`)
- Modify: `crates/d7s/src/event_handlers.rs`
- Modify: `crates/d7s/src/connection_manager.rs`
- Modify: `crates/d7s/src/table_data_actions.rs`
- Modify: `crates/d7s/src/app.rs` (only if it references removed types directly — check first)
- Modify: `crates/d7s/src/rendering.rs` (only if `render_modals` needs a `.clone()` adjustment — the new types are `Clone`, so likely no change)

**Interfaces:**
- Consumes: `k9tui::widgets::modal::{ConfirmDialog, TextPromptModal, DialogAction, PromptValidation}` from Task 3.
- Produces: `ModalManager` keeps its exact public method signatures (`open_confirmation_modal`, `open_sql_execution_confirmation_modal`, `open_password_modal`, `open_jump_to_row_modal`, `was_confirmation_modal_confirmed`, `was_sql_execution_confirmed`, `was_jump_to_row_confirmed`, `get_confirmation_modal`, `get_sql_execution_confirmation_modal`, `get_password_modal`/`_mut`, `get_jump_to_row_modal`, `handle_key_events_ui`, `handle_paste`, `close_active_modal`, `cleanup_closed_modals`, `is_any_modal_open`) — only their internal field types change, so no call site outside `modal.rs` needs a signature update, only the two spots identified below that read fields no longer present on the dialog structs directly.

- [ ] **Step 1: Read the current `ModalManager` in full**

Run: `sed -n '2219,2706p' crates/d7s/src/ui/widgets/modal.rs`

This is the full `ModalManager` struct + impl, already captured earlier in this conversation. Confirm field names/types match what's described below before editing.

- [ ] **Step 2: Update `ModalManager`'s fields**

Change:

```rust
#[derive(Default, Debug)]
pub struct ModalManager {
    connection_modal: Option<Modal>,
    confirmation_modal: Option<ConfirmationModal>,
    sql_execution_confirmation_modal: Option<SqlExecutionConfirmationModal>,
    sql_query_selection_modal: Option<SqlQuerySelectionModal>,
    cell_value_modal: Option<CellValueModal>,
    cell_value_apply: Option<CellValueApply>,
    password_modal: Option<PasswordModal>,
    jump_to_row_modal: Option<JumpToRowModal>,
    active_modal_type: Option<ModalType>,
}
```

to:

```rust
#[derive(Default, Debug)]
pub struct ModalManager {
    connection_modal: Option<Modal>,
    confirmation_modal: Option<ConfirmDialog>,
    confirmation_connection: Option<Connection>,
    sql_execution_confirmation_modal: Option<ConfirmDialog>,
    sql_execution_statement: Option<String>,
    sql_query_selection_modal: Option<SqlQuerySelectionModal>,
    cell_value_modal: Option<CellValueModal>,
    cell_value_apply: Option<CellValueApply>,
    password_modal: Option<TextPromptModal>,
    password_connection: Option<Connection>,
    jump_to_row_modal: Option<TextPromptModal>,
    active_modal_type: Option<ModalType>,
}
```

`confirmation_connection`/`password_connection` replace the `connection: Option<Connection>` field that used to live inside `ConfirmationModal`/`PasswordModal` directly (per the spec addendum: correlation data moves to the manager, since the generic dialog type has no `Connection` knowledge). `sql_execution_statement` replaces `SqlExecutionConfirmationModal.statement` for the same reason — `ConfirmDialog` only carries `message`, not a separate statement-to-execute field, so the manager stores it alongside.

Add the import at the top of the file: `use k9tui::widgets::modal::{ConfirmDialog, DialogAction, ModalField, PromptValidation, TextPromptModal};` (merge into the existing `use k9tui::{...}` block from Task 2).

- [ ] **Step 3: Delete the four old struct/impl blocks**

Delete from `crates/d7s/src/ui/widgets/modal.rs`:
- `pub struct ConfirmationModal { ... }` (with its `#[derive(Default, Debug, Clone)]`)
- `pub struct PasswordModal { ... }`
- `impl ConfirmationModal { ... }` (the `new`/`close`/`next_button`/`prev_button`/`confirm`/`handle_key_events` block)
- `impl SqlExecutionConfirmationModal { ... }`
- `pub struct SqlExecutionConfirmationModal { ... }`
- `impl PasswordModal { ... }`
- `impl Widget for ConfirmationModal { ... }`
- `impl Widget for SqlExecutionConfirmationModal { ... }`
- `impl Widget for PasswordModal { ... }`
- `pub struct JumpToRowModal { ... }`
- `impl JumpToRowModal { ... }`
- `impl Widget for JumpToRowModal { ... }`
- The now-unused constants: `CONFIRMATION_MODAL_WIDTH`, `CONFIRMATION_MODAL_HEIGHT`, `PASSWORD_MODAL_WIDTH`, `PASSWORD_MODAL_HEIGHT`, `JUMP_TO_ROW_MODAL_WIDTH`, `JUMP_TO_ROW_MODAL_HEIGHT`.

Leave `SqlQuerySelectionModal` and its impls untouched (deferred per spec addendum).

- [ ] **Step 4: Update `ModalManager::new()`**

```rust
    #[must_use]
    pub const fn new() -> Self {
        Self {
            connection_modal: None,
            confirmation_modal: None,
            confirmation_connection: None,
            sql_execution_confirmation_modal: None,
            sql_execution_statement: None,
            sql_query_selection_modal: None,
            cell_value_modal: None,
            cell_value_apply: None,
            password_modal: None,
            password_connection: None,
            jump_to_row_modal: None,
            active_modal_type: None,
        }
    }
```

- [ ] **Step 5: Update `open_confirmation_modal`**

```rust
    /// Open a confirmation modal
    pub fn open_confirmation_modal(
        &mut self,
        message: String,
        connection: Connection,
    ) {
        let modal = ConfirmDialog::new(
            "Confirm Delete",
            message,
            theme::modal_danger_border(),
            0,
        );
        self.confirmation_modal = Some(modal);
        self.confirmation_connection = Some(connection);
        self.active_modal_type = Some(ModalType::Confirmation);
    }
```

- [ ] **Step 6: Update `open_sql_execution_confirmation_modal`**

```rust
    pub fn open_sql_execution_confirmation_modal(&mut self, statement: String) {
        let preview = statement
            .lines()
            .take(3)
            .collect::<Vec<_>>()
            .join("\n")
            .chars()
            .take(180)
            .collect::<String>();
        let message = format!(
            "This statement may modify data.\n\nExecute anyway?\n\n{preview}"
        );
        let modal = ConfirmDialog::new(
            "Confirm SQL Execution",
            message,
            theme::modal_confirm_border(),
            1,
        );
        self.sql_execution_confirmation_modal = Some(modal);
        self.sql_execution_statement = Some(statement);
        self.active_modal_type = Some(ModalType::SqlExecutionConfirmation);
    }
```

- [ ] **Step 7: Update `open_password_modal`**

```rust
    /// Open a password input modal
    pub fn open_password_modal(
        &mut self,
        connection: Connection,
        prompt: String,
    ) {
        let modal = TextPromptModal::new("Enter Password", 50, 8)
            .with_prompt(prompt)
            .masked();
        self.password_modal = Some(modal);
        self.password_connection = Some(connection);
        self.active_modal_type = Some(ModalType::Password);
    }
```

- [ ] **Step 8: Update `open_jump_to_row_modal`**

```rust
    pub fn open_jump_to_row_modal(&mut self) {
        let modal = TextPromptModal::new(" Jump to row ", 36, 7)
            .with_placeholder("Row number")
            .with_validation(PromptValidation::PositiveInteger)
            .with_buttons("Go", "Cancel");
        self.jump_to_row_modal = Some(modal);
        self.active_modal_type = Some(ModalType::JumpToRow);
    }
```

- [ ] **Step 9: Update `was_jump_to_row_confirmed` and `get_jump_to_row_modal`**

```rust
    /// Row number if jump modal closed with Go.
    #[must_use]
    pub fn was_jump_to_row_confirmed(&self) -> Option<u64> {
        if let Some(modal) = &self.jump_to_row_modal
            && !modal.is_open
            && modal.submitted
        {
            return modal.parsed_positive_int();
        }
        None
    }

    #[must_use]
    pub const fn get_jump_to_row_modal(&self) -> Option<&TextPromptModal> {
        self.jump_to_row_modal.as_ref()
    }
```

- [ ] **Step 10: Update `close_active_modal`, `handle_key_events_ui`, `cleanup_closed_modals`, getters — swap type names and route `DialogAction` back to `ModalAction`**

In `handle_key_events_ui`, the `Confirmation` arm becomes:

```rust
            Some(ModalType::Confirmation) => {
                if let Some(modal) = &mut self.confirmation_modal {
                    let action = modal.handle_key_events(key);
                    if !modal.is_open {
                        self.active_modal_type = None;
                    }
                    match action {
                        DialogAction::Submit => ModalAction::Save,
                        DialogAction::Cancel => ModalAction::Cancel,
                        DialogAction::None => ModalAction::None,
                    }
                } else {
                    ModalAction::None
                }
            }
```

The `SqlExecutionConfirmation` arm becomes the same shape, reading `self.sql_execution_confirmation_modal`.

The `Password` arm becomes:

```rust
            Some(ModalType::Password) => {
                if let Some(modal) = &mut self.password_modal {
                    let action = modal.handle_key_events(key);
                    if !modal.is_open {
                        self.active_modal_type = None;
                    }
                    match action {
                        DialogAction::Submit => ModalAction::Save,
                        DialogAction::Cancel => ModalAction::Cancel,
                        DialogAction::None => ModalAction::None,
                    }
                } else {
                    ModalAction::None
                }
            }
```

The `JumpToRow` arm follows the same pattern reading `self.jump_to_row_modal`.

Update `get_confirmation_modal` return type to `Option<&ConfirmDialog>`, `get_sql_execution_confirmation_modal` to `Option<&ConfirmDialog>`, `get_password_modal`/`get_password_modal_mut` to `Option<&TextPromptModal>`/`Option<&mut TextPromptModal>`.

Update `was_confirmation_modal_confirmed`:

```rust
    /// Check if the confirmation modal was just closed and confirmed
    #[must_use]
    pub fn was_confirmation_modal_confirmed(&self) -> Option<Connection> {
        if let Some(modal) = &self.confirmation_modal
            && !modal.is_open
            && modal.is_confirmed()
        {
            return self.confirmation_connection.clone();
        }
        None
    }
```

Update `was_sql_execution_confirmed`:

```rust
    /// Check if SQL execution confirmation modal was just closed and confirmed.
    #[must_use]
    pub fn was_sql_execution_confirmed(&self) -> Option<String> {
        if let Some(modal) = &self.sql_execution_confirmation_modal
            && !modal.is_open
            && modal.is_confirmed()
        {
            return self.sql_execution_statement.clone();
        }
        None
    }
```

`cleanup_closed_modals` needs no logic change beyond the field types already matching (`Option<ConfirmDialog>`/`Option<TextPromptModal>` both still expose `.is_open`) — leave its body as-is, it will just compile against the new types. Also clear the correlation fields alongside: in the `confirmation_modal`/`password_modal` cleanup branches, additionally set `self.confirmation_connection = None;` / `self.password_connection = None;` when the modal is cleared, to avoid stale connections lingering after cleanup.

- [ ] **Step 11: Update `crates/d7s/src/event_handlers.rs`**

Run: `sed -n '600,660p' src/event_handlers.rs` (from `crates/d7s/`) to see the exact current `handle_password_modal_save` body.

Change the two field-read spots:

`password_modal.connection.clone()` → `self.modal_manager.password_connection.clone()`. Since `password_connection` is a private field on `ModalManager`, either (a) make it accessible via a small new getter `ModalManager::password_connection(&self) -> Option<&Connection>`, or (b) inline the read at the point `handle_password_modal_save` already borrows `self.modal_manager` mutably for `get_password_modal_mut()` — do (a), it's cleaner:

Add to `ModalManager`:

```rust
    /// Connection associated with the currently open (or just-closed) password modal.
    #[must_use]
    pub const fn password_connection(&self) -> Option<&Connection> {
        self.password_connection.as_ref()
    }
```

In `event_handlers.rs`, change:

```rust
let Some(connection) = password_modal.connection.clone() else {
```

to:

```rust
let Some(connection) = self.modal_manager.password_connection().cloned() else {
```

(adjust ordering/borrow so this read happens before or independent of the `password_modal` mutable borrow — check the surrounding function body from the Step 11 read to place it correctly, likely right after obtaining `password_modal` but before mutating it, or hoisted above the `let Some(password_modal) = ...` line entirely since it doesn't need the mutable borrow).

Change `password_modal.password()` → `password_modal.input_value()`.

Change `password_modal.clear_password()` → `password_modal.clear_input()`.

- [ ] **Step 12: Check `crates/d7s/src/connection_manager.rs` and `crates/d7s/src/table_data_actions.rs`**

Run: `grep -n "password_modal\|confirmation_modal\|jump_to_row_modal" src/connection_manager.rs src/table_data_actions.rs src/app.rs` (from `crates/d7s/`)

These files only call `.open_password_modal(...)` / `.open_sql_execution_confirmation_modal(...)` / `.open_confirmation_modal(...)` / `.open_jump_to_row_modal()` per the earlier grep in this conversation — all unchanged signatures, so no edits expected here. If the grep surfaces anything reading a field directly (not just calling an `open_*`/`was_*` method), apply the same field-rename pattern as Step 11.

- [ ] **Step 13: Build and test**

Run: `cargo build --workspace 2>&1 | tail -80`
Expected: clean build. Fix any remaining type-mismatch errors by re-checking Steps 2–10 against the actual current file (the exact line ranges may have shifted slightly after Tasks 1–3's edits).

Run: `cargo clippy --workspace 2>&1 | tail -80`
Expected: no new warnings. Watch for `must_use_candidate` on the new `password_connection()` getter (already added).

Run: `cargo test --workspace 2>&1 | tail -30`
Expected: all tests pass.

- [ ] **Step 14: Manual smoke check**

Run: `cargo run -p d7s -- --help 2>&1 | head -5` to confirm the binary still builds and starts (a full TUI interaction check isn't automatable here, but this catches gross link/panic-on-init errors). If a demo/test DB fixture exists for interactive testing, note it for the task reviewer but don't block on it — the automated test suite is the primary gate per Global Constraints.

- [ ] **Step 15: Commit**

```bash
git add -A
git commit -m "refactor: migrate d7s modal.rs onto k9tui ConfirmDialog/TextPromptModal

Jump-to-row dialog gains Left/Right arrow-to-button navigation as a side
effect of sharing TextPromptModal's key handling with the password dialog
(previously it routed all non-Enter/Tab/Esc keys into the input). Ruled
acceptable in the k9tui workspace design addendum."
```

---

## Self-Review Notes

- **Task ordering:** Task 1 (top bar) is independent of Tasks 2–4 and could run in parallel, but keep it first anyway — it's the smallest, fastest task and gives the SDD loop an early clean win before the larger modal surgery.
- **Task 2 before Task 3 before Task 4** is a hard dependency chain: Task 3's `ConfirmDialog`/`TextPromptModal` live in the same file Task 2 creates; Task 4 consumes both.
- **Line numbers throughout are from the pre-Task-1 state of `crates/d7s/src/ui/widgets/modal.rs`** (2706 lines, as read during planning) — each task's dispatch should tell the implementer to re-grep for exact current line numbers rather than trust the numbers literally, since earlier tasks in this same plan shift them. Step instructions already say "re-read" at the relevant points; flagged again here for the controller's awareness when writing task briefs.
- **Behavior-change exception is called out three times** (Global Constraints, Task 3, Task 4 commit message) deliberately — this is the one place this plan knowingly deviates from pixel-identical behavior, and it should survive review, not get "fixed" back to divergent copy-pasted logic.
- **`SqlQuerySelectionModal` is explicitly out of scope** — confirmed no task touches it; it stays in `crates/d7s/src/ui/widgets/modal.rs` unchanged.
