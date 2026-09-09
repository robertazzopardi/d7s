use k9tui::theme;
use ratatui::{
    prelude::*,
    widgets::{Paragraph, Widget},
};

use crate::app_state::AppState;

/// Footer status line (ratatui `Paragraph`).
#[derive(Clone, Debug, Default)]
pub struct StatusLine {
    message: String,
    idle_hint: String,
}

impl StatusLine {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            message: String::new(),
            idle_hint: String::new(),
        }
    }

    pub fn set_message(&mut self, message: impl Into<String>) {
        self.message = message.into();
    }

    pub fn set_idle_hint(&mut self, hint: impl Into<String>) {
        self.idle_hint = hint.into();
    }

    pub fn clear(&mut self) {
        self.message.clear();
    }
}

impl Widget for StatusLine {
    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        if area.height == 0 || area.width == 0 {
            return;
        }

        let (text, style) = if self.message.is_empty() {
            (self.idle_hint.as_str(), theme::status_idle())
        } else {
            (self.message.as_str(), theme::status_message())
        };

        Paragraph::new(text)
            .style(style)
            .wrap(ratatui::widgets::Wrap { trim: true })
            .render(area, buf);
    }
}

#[must_use]
pub fn default_idle_hint(app_state: AppState) -> String {
    match app_state {
        AppState::ConnectionList => "? help · n new · q quit".to_string(),
        AppState::DatabaseConnected => "? help · Esc back · q quit".to_string(),
    }
}
