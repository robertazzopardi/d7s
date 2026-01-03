use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, StatefulWidget, Widget},
};

use crate::widgets::text_input::TextInput;

/// A search filter widget that appears above the main table
#[derive(Debug, Clone, Default)]
pub struct SearchFilter {
    input: TextInput,
    pub is_active: bool,
}

impl SearchFilter {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub const fn activate(&mut self) {
        self.is_active = true;
        self.input.move_cursor_to_end();
    }

    pub fn deactivate(&mut self) {
        self.is_active = false;
        self.input.clear();
    }

    pub fn add_char(&mut self, ch: char) {
        self.input.add_char(ch);
    }

    pub fn delete_char(&mut self) {
        self.input.delete_char();
    }

    pub const fn move_cursor_left(&mut self) {
        self.input.move_cursor_left();
    }

    pub const fn move_cursor_right(&mut self) {
        self.input.move_cursor_right();
    }

    pub const fn move_cursor_to_start(&mut self) {
        self.input.move_cursor_to_start();
    }

    pub const fn move_cursor_to_end(&mut self) {
        self.input.move_cursor_to_end();
    }

    pub fn clear(&mut self) {
        self.input.clear();
    }

    #[must_use]
    pub fn get_filter_query(&self) -> &str {
        self.input.text().trim()
    }

    /// Get the cursor position for rendering
    #[must_use]
    pub const fn cursor_position(&self) -> usize {
        self.input.cursor_position()
    }
}

impl StatefulWidget for SearchFilter {
    type State = ();

    fn render(
        self,
        area: Rect,
        buf: &mut ratatui::buffer::Buffer,
        _state: &mut Self::State,
    ) {
        if !self.is_active {
            return;
        }

        // Create the search input block
        let block = Block::new()
            .borders(Borders::ALL)
            .title(" Search Filter (ESC to cancel) ")
            .title_alignment(ratatui::layout::Alignment::Left);

        let inner_area = block.inner(area);

        // Render the block
        Widget::render(block, area, buf);

        // Create the search input with cursor
        let mut spans = Vec::new();

        let cursor_pos = self.input.cursor_position();
        let query = self.input.text();

        // Add the query text before cursor
        if cursor_pos > 0 {
            spans.push(Span::raw(&query[..cursor_pos]));
        }

        // Add cursor
        spans.push(Span::styled("█", Style::default().fg(Color::White)));

        // Add the query text after cursor
        if cursor_pos < query.len() {
            spans.push(Span::raw(&query[cursor_pos..]));
        }

        let line = Line::from(spans);
        let paragraph =
            Paragraph::new(line).style(Style::default().fg(Color::White));

        // Render the paragraph
        Widget::render(paragraph, inner_area, buf);
    }
}
