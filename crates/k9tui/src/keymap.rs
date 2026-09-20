use std::collections::HashMap;

use crossterm::event::KeyCode;
use serde::Deserialize;

use crate::config_dir::config_dir;

/// Parse a `keys.yml` key spec: a single character (`"s"`, `"?"`) or a
/// named key (`"esc"`, `"enter"`, `"f5"`, `"pageup"`, ...), case-insensitive.
#[must_use]
pub fn parse_key(s: &str) -> Option<KeyCode> {
    let mut chars = s.chars();
    if let (Some(c), None) = (chars.next(), chars.next()) {
        return Some(KeyCode::Char(c));
    }
    Some(match s.to_ascii_lowercase().as_str() {
        "esc" | "escape" => KeyCode::Esc,
        "enter" | "return" => KeyCode::Enter,
        "tab" => KeyCode::Tab,
        "backspace" => KeyCode::Backspace,
        "delete" | "del" => KeyCode::Delete,
        "space" => KeyCode::Char(' '),
        "up" => KeyCode::Up,
        "down" => KeyCode::Down,
        "left" => KeyCode::Left,
        "right" => KeyCode::Right,
        "home" => KeyCode::Home,
        "end" => KeyCode::End,
        "pageup" => KeyCode::PageUp,
        "pagedown" => KeyCode::PageDown,
        "f1" => KeyCode::F(1),
        "f2" => KeyCode::F(2),
        "f3" => KeyCode::F(3),
        "f4" => KeyCode::F(4),
        "f5" => KeyCode::F(5),
        "f6" => KeyCode::F(6),
        "f7" => KeyCode::F(7),
        "f8" => KeyCode::F(8),
        "f9" => KeyCode::F(9),
        "f10" => KeyCode::F(10),
        "f11" => KeyCode::F(11),
        "f12" => KeyCode::F(12),
        _ => return None,
    })
}

#[derive(Debug, Deserialize, Default)]
struct KeymapFile {
    #[serde(flatten)]
    bindings: HashMap<String, String>,
}

/// Action-name -> key bindings, seeded with hardcoded defaults and
/// optionally overridden by `~/.config/<app>/keys.yml` (a flat map of
/// `action: key`). Unknown actions in the file are ignored; unparsable key
/// specs keep the default for that action.
#[derive(Debug, Clone)]
pub struct Keymap {
    bindings: HashMap<&'static str, KeyCode>,
}

impl Keymap {
    #[must_use]
    pub fn new(defaults: &[(&'static str, KeyCode)]) -> Self {
        Self {
            bindings: defaults.iter().copied().collect(),
        }
    }

    /// Load `~/.config/<app>/keys.yml` and apply matching overrides in place.
    /// Does nothing if the config dir or file doesn't exist.
    pub fn load_overrides(&mut self, app: &str) {
        let Some(dir) = config_dir(app) else {
            return;
        };
        let Ok(contents) = std::fs::read_to_string(dir.join("keys.yml")) else {
            return;
        };
        let Ok(file) = serde_yaml::from_str::<KeymapFile>(&contents) else {
            return;
        };
        for (action, key_spec) in file.bindings {
            let Some(slot) = self
                .bindings
                .iter_mut()
                .find(|(name, _)| **name == action)
            else {
                continue;
            };
            if let Some(code) = parse_key(&key_spec) {
                *slot.1 = code;
            }
        }
    }

    /// The key bound to `action`. Panics if `action` isn't one of the
    /// defaults this keymap was built with — a programmer error, not a
    /// runtime/config one.
    #[must_use]
    pub fn get(&self, action: &str) -> KeyCode {
        *self
            .bindings
            .get(action)
            .unwrap_or_else(|| panic!("unknown keymap action: {action}"))
    }

    /// True if `code` is the key currently bound to `action`.
    #[must_use]
    pub fn is(&self, action: &str, code: KeyCode) -> bool {
        self.get(action) == code
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_chars_and_named_keys() {
        assert_eq!(parse_key("s"), Some(KeyCode::Char('s')));
        assert_eq!(parse_key("Esc"), Some(KeyCode::Esc));
        assert_eq!(parse_key("F5"), Some(KeyCode::F(5)));
        assert_eq!(parse_key(""), None);
        assert_eq!(parse_key("nonsense"), None);
    }

    #[test]
    fn defaults_are_used_when_unset() {
        let km = Keymap::new(&[("quit", KeyCode::Char('q'))]);
        assert_eq!(km.get("quit"), KeyCode::Char('q'));
        assert!(km.is("quit", KeyCode::Char('q')));
    }
}
