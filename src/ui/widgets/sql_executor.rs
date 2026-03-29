use ratatui::{
    prelude::*,
    widgets::{Paragraph, StatefulWidget, Wrap},
};

use crate::ui::widgets::{
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
    pub fn set_sql(&mut self, sql: String) {
        self.input.set_text(sql);
    }

    /// Get the SQL input text
    #[must_use]
    pub fn sql_input(&self) -> &str {
        self.input.text()
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
