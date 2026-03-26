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
    #[must_use]
    pub const fn new(c: char, description: &'a str) -> Self {
        Self {
            keycode: KeyCode::Char(c),
            description,
        }
    }

    #[must_use]
    pub fn length(&self) -> u16 {
        let key_len =
            u16::try_from(self.keycode.to_string().len()).unwrap_or(1);
        let desc_len = u16::try_from(self.description.len()).unwrap_or(1);

        key_len + desc_len + 3
    }
}
