use ratatui::{
    prelude::*,
    style::Style,
    widgets::{Paragraph, Widget},
};

/// A simple status line widget that displays a message at the bottom of the screen
#[derive(Clone, Debug, Default)]
pub struct StatusLine {
    message: String,
}

impl StatusLine {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            message: String::new(),
        }
    }

    /// Set the status message
    pub fn set_message(&mut self, message: impl Into<String>) {
        self.message = message.into();
    }

    /// Get the current status message
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Clear the status message
    pub fn clear(&mut self) {
        self.message.clear();
    }
}

impl Widget for StatusLine {
    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        if area.height == 0 || area.width == 0 {
            return;
        }

        // Always render the message, even if empty (will show blank line)
        let text = if self.message.is_empty() {
            " ".to_string()
        } else {
            self.message
        };

        let paragraph = Paragraph::new(text.as_str())
            .style(Style::default())
            .wrap(ratatui::widgets::Wrap { trim: true });

        paragraph.render(area, buf);
    }
}
