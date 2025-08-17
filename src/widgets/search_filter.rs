use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, StatefulWidget, Widget},
};

/// A search filter widget that appears above the main table
#[derive(Debug, Clone)]
pub struct SearchFilter {
    pub query: String,
    pub is_active: bool,
    pub cursor_position: usize,
}

impl Default for SearchFilter {
    fn default() -> Self {
        Self {
            query: String::new(),
            is_active: false,
            cursor_position: 0,
        }
    }
}

impl SearchFilter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn activate(&mut self) {
        self.is_active = true;
        self.cursor_position = self.query.len();
    }

    pub fn deactivate(&mut self) {
        self.is_active = false;
        self.query.clear();
        self.cursor_position = 0;
    }

    pub fn add_char(&mut self, ch: char) {
        self.query.insert(self.cursor_position, ch);
        self.cursor_position += 1;
    }

    pub fn delete_char(&mut self) {
        if self.cursor_position > 0 {
            self.query.remove(self.cursor_position - 1);
            self.cursor_position -= 1;
        }
    }

    pub fn move_cursor_left(&mut self) {
        if self.cursor_position > 0 {
            self.cursor_position -= 1;
        }
    }

    pub fn move_cursor_right(&mut self) {
        if self.cursor_position < self.query.len() {
            self.cursor_position += 1;
        }
    }

    pub fn move_cursor_to_start(&mut self) {
        self.cursor_position = 0;
    }

    pub fn move_cursor_to_end(&mut self) {
        self.cursor_position = self.query.len();
    }

    pub fn clear(&mut self) {
        self.query.clear();
        self.cursor_position = 0;
    }

    pub fn get_filter_query(&self) -> &str {
        self.query.trim()
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

        // Add the query text before cursor
        if self.cursor_position > 0 {
            spans.push(Span::raw(&self.query[..self.cursor_position]));
        }

        // Add cursor
        spans.push(Span::styled("█", Style::default().fg(Color::White)));

        // Add the query text after cursor
        if self.cursor_position < self.query.len() {
            spans.push(Span::raw(&self.query[self.cursor_position..]));
        }

        let line = Line::from(spans);
        let paragraph =
            Paragraph::new(line).style(Style::default().fg(Color::White));

        // Render the paragraph
        Widget::render(paragraph, inner_area, buf);
    }
}
