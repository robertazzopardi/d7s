use ratatui::{
    prelude::*,
    widgets::{Paragraph, StatefulWidget, Wrap},
};
use ratatui_textarea::TextArea;

use crate::ui::widgets::table::{DataTable, RawTableRow, TableDataState};

/// State for the SQL executor widget
#[derive(Debug, Clone)]
pub struct SqlExecutorState {
    input: TextArea<'static>,
    pub results: Option<Vec<Vec<String>>>,
    pub column_names: Vec<String>,
    pub error_message: Option<String>,
    pub is_active: bool,
    pub table_state: TableDataState<RawTableRow>,
}

impl Default for SqlExecutorState {
    fn default() -> Self {
        let mut input = TextArea::default();
        input.set_cursor_line_style(Style::default());
        input.set_cursor_style(Style::default());
        // SQL is loaded from an external editor, so undo/redo within the widget isn't useful
        input.set_max_histories(0);
        Self {
            input,
            results: None,
            column_names: Vec::new(),
            error_message: None,
            is_active: false,
            table_state: TableDataState::default(),
        }
    }
}

impl SqlExecutorState {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub const fn deactivate(&mut self) {
        self.is_active = false;
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

    #[allow(dead_code)]
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

    /// Replace the SQL input text entirely after loading from external editor
    pub fn set_sql(&mut self, sql: &str) {
        let lines: Vec<String> = sql.lines().map(String::from).collect();
        self.input = TextArea::new(if lines.is_empty() {
            vec![String::new()]
        } else {
            lines
        });
        self.input.set_cursor_line_style(Style::default());
        self.input.set_cursor_style(Style::default());
        self.input.set_max_histories(0);
        self.input.move_cursor(ratatui_textarea::CursorMove::Bottom);
        self.input.move_cursor(ratatui_textarea::CursorMove::End);
    }

    /// Get the SQL input text
    #[must_use]
    pub fn sql_input(&self) -> String {
        self.input.lines().join("\n")
    }
}

/// SQL executor widget, which is stateless, only handles rendering
pub struct SqlExecutor;

impl StatefulWidget for SqlExecutor {
    type State = SqlExecutorState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        if let Some(error) = &state.error_message {
            Paragraph::new(error.clone())
                .style(Style::default().fg(Color::Red))
                .wrap(Wrap { trim: true })
                .render(area, buf);
        } else if let Some(results) = &state.results {
            if results.is_empty() {
                Paragraph::new("No results")
                    .style(Style::default().fg(Color::Gray))
                    .render(area, buf);
            } else {
                DataTable::<RawTableRow>::default().render(
                    area,
                    buf,
                    &mut state.table_state,
                );
            }
        } else {
            Paragraph::new("Press 'e' to open editor")
                .style(Style::default().fg(Color::DarkGray))
                .render(area, buf);
        }
    }
}
