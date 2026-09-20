use crossterm::event::KeyCode;
use k9tui::keymap::Keymap;

use crate::app::PKG_NAME;

/// Default bindings for the actions shown in the hotkey bar (see
/// `ui::widgets::hotkeys` and `ui::widgets::global_hotkeys`). Vim-style
/// navigation (h/j/k/l, g/G, 0/$, /), quit, help, and Esc are not
/// remappable — only the actions listed here and documented in the README.
const DEFAULTS: &[(&str, KeyCode)] = &[
    // Connection list
    ("new_connection", KeyCode::Char('n')),
    ("edit_connection", KeyCode::Char('e')),
    ("delete_connection", KeyCode::Char('d')),
    ("open_connection", KeyCode::Char('o')),
    // Database view
    ("sql_editor", KeyCode::Char('e')),
    ("table_structure", KeyCode::Char('t')),
    ("run_sql", KeyCode::Char('E')),
    ("copy_value", KeyCode::Char('y')),
    // Table data view
    ("refresh", KeyCode::Char('r')),
    ("new_row", KeyCode::Char('a')),
    ("duplicate_row", KeyCode::Char('c')),
    ("commit_row", KeyCode::Char('s')),
    ("delete_row", KeyCode::Char('d')),
];

/// Build the keymap with built-in defaults, then apply
/// `~/.config/d7s/keys.yml` overrides if present.
#[must_use]
pub fn load() -> Keymap {
    let mut keymap = Keymap::new(DEFAULTS);
    keymap.load_overrides(PKG_NAME);
    keymap
}
