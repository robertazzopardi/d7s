use ratatui::{
    prelude::*,
    widgets::{Paragraph, Wrap},
};

use crate::widgets::table::{DataTable, RawTableRow};

#[derive(Debug, Clone, Default)]
pub struct SqlExecutor {
    pub sql_input: String,
    pub cursor_position: usize,
    pub results: Option<Vec<Vec<String>>>,
    pub column_names: Vec<String>,
    pub error_message: Option<String>,
    pub is_active: bool,
    pub table_widget: Option<DataTable<RawTableRow>>,
}

impl SqlExecutor {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub const fn activate(&mut self) {
        self.is_active = true;
    }

    pub const fn deactivate(&mut self) {
        self.is_active = false;
    }

    pub fn add_char(&mut self, ch: char) {
        self.sql_input.insert(self.cursor_position, ch);
        self.cursor_position += 1;
        // Clear results when user starts typing a new query
        self.clear_results();
    }

    pub fn delete_char(&mut self) {
        if self.cursor_position > 0 {
            self.sql_input.remove(self.cursor_position - 1);
            self.cursor_position -= 1;
        }
    }

    pub const fn move_cursor_left(&mut self) {
        if self.cursor_position > 0 {
            self.cursor_position -= 1;
        }
    }

    pub const fn move_cursor_right(&mut self) {
        if self.cursor_position < self.sql_input.len() {
            self.cursor_position += 1;
        }
    }

    pub const fn move_cursor_to_start(&mut self) {
        self.cursor_position = 0;
    }

    pub const fn move_cursor_to_end(&mut self) {
        self.cursor_position = self.sql_input.len();
    }

    pub fn clear(&mut self) {
        self.sql_input.clear();
        self.cursor_position = 0;
        // Clear results when clearing input
        self.clear_results();
    }

    pub fn set_results(
        &mut self,
        results: Vec<Vec<String>>,
        column_names: Vec<String>,
    ) {
        self.results = Some(results.clone());
        self.column_names.clone_from(&column_names);
        self.error_message = None;
        self.table_widget =
            Some(DataTable::from_raw_data(results, column_names));
    }

    pub fn set_error(&mut self, error: String) {
        self.error_message = Some(error);
        self.results = None;
    }

    pub fn clear_results(&mut self) {
        self.results = None;
        self.column_names.clear();
        self.error_message = None;
        self.table_widget = None;
    }
}

impl Widget for SqlExecutor {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // If we have results or an error, show them
        if self.results.is_some() || self.error_message.is_some() {
            if let Some(error) = &self.error_message {
                // Render error message
                let error_paragraph = Paragraph::new(error.clone())
                    .style(Style::default().fg(Color::Red))
                    .wrap(Wrap { trim: true });
                error_paragraph.render(area, buf);
            } else if let Some(results) = &self.results {
                if results.is_empty() {
                    let empty_paragraph = Paragraph::new("No results")
                        .style(Style::default().fg(Color::Gray));
                    empty_paragraph.render(area, buf);
                } else if let Some(table_widget) = &self.table_widget {
                    // Render results using the table widget
                    table_widget.clone().render(
                        area,
                        buf,
                        &mut table_widget.table_state.clone(),
                    );
                }
            }
        } else {
            // No results yet, show full SQL input area
            let input_text = if self.is_active {
                format!("{}|", self.sql_input)
            } else {
                self.sql_input.clone()
            };

            let input_paragraph = Paragraph::new(input_text).style(
                Style::default().fg(if self.is_active {
                    Color::White
                } else {
                    Color::Gray
                }),
            );

            input_paragraph.render(area, buf);
        }
    }
}
