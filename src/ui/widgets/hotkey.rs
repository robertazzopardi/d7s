use std::{borrow::Cow, fmt::Display};

use crossterm::event::KeyCode;
use unicode_width::UnicodeWidthStr;

#[derive(Debug, Clone)]
pub enum HotkeyDescription {
    Static(&'static str),
    /// Digit hotkey to reopen a recently viewed table (`schema.table` in the bar).
    RecentTable {
        schema: String,
        table: String,
    },
}

impl HotkeyDescription {
    #[must_use]
    pub fn display_suffix(&self) -> Cow<'_, str> {
        match self {
            Self::Static(s) => Cow::Borrowed(*s),
            Self::RecentTable { schema, table } => {
                Cow::Owned(format!("{schema}.{table}"))
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Hotkey {
    pub keycode: KeyCode,
    pub description: HotkeyDescription,
}

impl Display for Hotkey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.keycode)
    }
}

impl Hotkey {
    #[must_use]
    pub const fn new(c: char, description: &'static str) -> Self {
        Self {
            keycode: KeyCode::Char(c),
            description: HotkeyDescription::Static(description),
        }
    }

    #[must_use]
    #[allow(dead_code)]
    pub fn length(&self) -> u16 {
        let key_len =
            u16::try_from(self.keycode.to_string().len()).unwrap_or(1);
        let desc_len = u16::try_from(UnicodeWidthStr::width(
            self.description.display_suffix().as_ref(),
        ))
        .unwrap_or(1);

        key_len + desc_len + 3
    }
}
