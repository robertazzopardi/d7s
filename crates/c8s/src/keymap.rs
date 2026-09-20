use crossterm::event::KeyCode;
use k9tui::keymap::Keymap;

use crate::app::PKG_NAME;

/// Default bindings for the actions shown in the hotkey bar. Navigation
/// (j/k/g/G, arrows) and quit/back are not remappable — only the actions
/// listed here and documented in the README.
const DEFAULTS: &[(&str, KeyCode)] = &[
    ("start_stop", KeyCode::Char('s')),
    ("restart", KeyCode::Char('r')),
    ("logs", KeyCode::Char('l')),
    ("exec", KeyCode::Char('e')),
    ("remove", KeyCode::Char('d')),
];

/// Build the keymap with built-in defaults, then apply
/// `~/.config/c8s/keys.yml` overrides if present.
#[must_use]
pub fn load() -> Keymap {
    let mut keymap = Keymap::new(DEFAULTS);
    keymap.load_overrides(PKG_NAME);
    keymap
}
