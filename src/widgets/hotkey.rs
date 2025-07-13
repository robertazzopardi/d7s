use std::fmt::Display;

use crossterm::event::KeyCode;

pub struct Hotkey<'a> {
    pub keycode: KeyCode,
    pub description: &'a str,
}

impl<'a> Display for Hotkey<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.keycode)
    }
}
