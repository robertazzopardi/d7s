use ratatui::{
    prelude::*,
    widgets::{Paragraph, StatefulWidget, Wrap},
};

use crate::widgets::{
    table::{DataTable, RawTableRow, TableDataState},
    text_input::TextInput,
};

/// State for the SQL executor widget
#[derive(Debug, Clone, Default)]
pub struct SqlExecutorState {
    input: TextInput,
    pub results: Option<Vec<Vec<String>>>,
    pub column_names: Vec<String>,
    pub error_message: Option<String>,
    pub is_active: bool,
    pub table_state: TableDataState<RawTableRow>,
}

impl SqlExecutorState {
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
        self.input.add_char(ch);
        // Clear results when user starts typing a new query
        self.clear_results();
    }

    pub fn delete_char(&mut self) {
        self.input.delete_char();
    }

    pub fn move_cursor_left(&mut self) {
        self.input.move_cursor_left();
    }

    pub fn move_cursor_right(&mut self) {
        self.input.move_cursor_right();
    }

    pub const fn move_cursor_to_start(&mut self) {
        self.input.move_cursor_to_start();
    }

    pub fn move_cursor_to_end(&mut self) {
        self.input.move_cursor_to_end();
    }

    pub fn clear(&mut self) {
        self.input.clear();
        // Clear results when clearing input
        self.clear_results();
    }

    pub fn set_results(
        &mut self,
        results: Vec<Vec<String>>,
        column_names: &[String],
    ) {
        self.results = Some(results.clone());
        self.column_names.clone_from(&column_names.to_vec());
        self.error_message = None;
        self.table_state.reset(results, column_names);
    }

    pub fn set_error(&mut self, error: String) {
        self.error_message = Some(error);
        self.results = None;
    }

    pub fn clear_results(&mut self) {
        self.results = None;
        self.column_names.clear();
        self.error_message = None;
        self.table_state.reset(vec![], &[]);
    }

    /// Get the SQL input text
    #[must_use]
    pub fn sql_input(&self) -> &str {
        self.input.text()
    }

    /// Get the cursor position
    #[must_use]
    pub const fn cursor_position(&self) -> usize {
        self.input.cursor_position()
    }
}

/// SQL executor widget, which is stateless, only handles rendering
pub struct SqlExecutor;

impl StatefulWidget for SqlExecutor {
    type State = SqlExecutorState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        // If we have results or an error, show them
        if state.results.is_some() || state.error_message.is_some() {
            if let Some(error) = &state.error_message {
                // Render error message
                let error_paragraph = Paragraph::new(error.clone())
                    .style(Style::default().fg(Color::Red))
                    .wrap(Wrap { trim: true });
                error_paragraph.render(area, buf);
            } else if let Some(results) = &state.results {
                if results.is_empty() {
                    let empty_paragraph = Paragraph::new("No results")
                        .style(Style::default().fg(Color::Gray));
                    empty_paragraph.render(area, buf);
                } else {
                    // Render results using the table widget
                    DataTable::<RawTableRow>::default().render(
                        area,
                        buf,
                        &mut state.table_state,
                    );
                }
            }
        } else {
            // No results yet, show full SQL input area
            let input_paragraph = Paragraph::new(state.sql_input()).style(
                Style::default().fg(if state.is_active {
                    Color::White
                } else {
                    Color::Gray
                }),
            );

            input_paragraph.render(area, buf);
        }
    }
}
