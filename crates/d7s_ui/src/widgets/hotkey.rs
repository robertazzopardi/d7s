use std::fmt::Display;

use crossterm::event::KeyCode;

#[derive(Debug, Clone)]
pub struct Hotkey<'a> {
    pub keycode: KeyCode,
    pub description: &'a str,
}

impl Display for Hotkey<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.keycode)
    }
}

impl<'a> Hotkey<'a> {
    pub const fn new(c: char, description: &'a str) -> Self {
        Self {
            keycode: KeyCode::Char(c),
            description,
        }
    }
}
