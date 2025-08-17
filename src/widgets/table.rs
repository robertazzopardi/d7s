use d7s_db::TableData;
use ratatui::{
    layout::{Constraint, Rect},
    style::{Modifier, Style},
    text::Text,
    widgets::{Cell, HighlightSpacing, Row, StatefulWidget, Table, TableState},
};

use crate::widgets::constraint_len_calculator;

/// A ratatui widget for displaying tabular data with selection and styling
#[derive(Clone, Debug, Default)]
pub struct DataTable<T: TableData + Clone> {
    pub items: Vec<T>,
    pub longest_item_lens: Vec<u16>, // order is (name, address, email)
    pub table_state: TableState,
}

/// A ratatui widget for displaying table data with dynamic column headers
#[derive(Clone, Debug, Default)]
pub struct TableDataWidget {
    pub items: Vec<Vec<String>>,
    pub column_names: Vec<String>,
    pub longest_item_lens: Vec<u16>,
    pub table_state: TableState,
}

impl TableDataWidget {
    pub fn new(items: Vec<Vec<String>>, column_names: Vec<String>) -> Self {
        let longest_item_lens =
            constraint_len_calculator_for_data(&items, &column_names);
        Self {
            items,
            column_names,
            longest_item_lens,
            table_state: TableState::default().with_selected(0),
        }
    }

    pub fn filter(&self, query: &str) -> Vec<Vec<String>> {
        if query.is_empty() {
            return self.items.clone();
        }

        let query_lower = query.to_lowercase();
        self.items
            .iter()
            .filter(|row| {
                // Check if any column contains the query
                row.iter()
                    .any(|cell| cell.to_lowercase().contains(&query_lower))
            })
            .cloned()
            .collect()
    }
}

impl StatefulWidget for TableDataWidget {
    type State = TableState;

    fn render(
        self,
        area: Rect,
        buf: &mut ratatui::buffer::Buffer,
        state: &mut Self::State,
    ) {
        let selected_row_style = Style::default()
            .add_modifier(Modifier::REVERSED | Modifier::BOLD)
            .fg(ratatui::style::Color::Black)
            .bg(ratatui::style::Color::Yellow);
        let selected_col_style =
            Style::default().fg(ratatui::style::Color::Cyan);
        let selected_cell_style = Style::default()
            .add_modifier(Modifier::REVERSED)
            .fg(ratatui::style::Color::Magenta);

        let header = self
            .column_names
            .iter()
            .map(|name| Cell::from(name.clone()))
            .collect::<Row>()
            .height(1);

        let rows = self.items.iter().map(|values| {
            values
                .iter()
                .map(|value| Cell::from(value.clone()))
                .collect::<Row>()
                .style(Style::new())
                .height(1)
        });

        let bar: &'static str = " █ ";
        let constraints = self
            .longest_item_lens
            .into_iter()
            .map(|len| Constraint::Min(len + 1)); // Add 1 for padding
        let t = Table::new(rows, constraints)
            .header(header)
            .row_highlight_style(selected_row_style)
            .column_highlight_style(selected_col_style)
            .cell_highlight_style(selected_cell_style)
            .highlight_symbol(Text::from(vec![
                "".into(),
                bar.into(),
                bar.into(),
                "".into(),
            ]))
            .highlight_spacing(HighlightSpacing::Always);

        StatefulWidget::render(t, area, buf, state);
    }
}

impl<T: TableData + Clone> DataTable<T> {
    pub fn new(items: Vec<T>) -> Self {
        let longest_item_lens = constraint_len_calculator(&items);
        Self {
            items,
            longest_item_lens,
            table_state: TableState::default().with_selected(0),
        }
    }

    pub fn filter(&self, query: &str) -> Vec<T> {
        if query.is_empty() {
            return self.items.clone();
        }

        let query_lower = query.to_lowercase();
        self.items
            .iter()
            .filter(|item| {
                // Check if any column contains the query
                for col_idx in 0..item.num_columns() {
                    let col_value = item.col(col_idx);
                    if col_value.to_lowercase().contains(&query_lower) {
                        return true;
                    }
                }
                false
            })
            .cloned()
            .collect()
    }
}

impl<T: TableData + std::fmt::Debug + Clone> StatefulWidget for DataTable<T> {
    type State = TableState;

    fn render(
        self,
        area: Rect,
        buf: &mut ratatui::buffer::Buffer,
        state: &mut Self::State,
    ) {
        let selected_row_style = Style::default()
            .add_modifier(Modifier::REVERSED | Modifier::BOLD)
            .fg(ratatui::style::Color::Black)
            .bg(ratatui::style::Color::Yellow);
        let selected_col_style =
            Style::default().fg(ratatui::style::Color::Cyan);
        let selected_cell_style = Style::default()
            .add_modifier(Modifier::REVERSED)
            .fg(ratatui::style::Color::Magenta);

        let header = T::cols()
            .into_iter()
            .map(Cell::from)
            .collect::<Row>()
            .height(1);

        let rows = self.items.iter().map(|data| {
            data.ref_array()
                .into_iter()
                .map(Cell::from)
                .collect::<Row>()
                .style(Style::new())
                .height(1)
        });

        let bar: &'static str = " █ ";
        let constraints = self
            .longest_item_lens
            .into_iter()
            .map(|len| Constraint::Min(len + 1)); // Add 1 for padding
        let t = Table::new(rows, constraints)
            .header(header)
            .row_highlight_style(selected_row_style)
            .column_highlight_style(selected_col_style)
            .cell_highlight_style(selected_cell_style)
            .highlight_symbol(Text::from(vec![
                "".into(),
                bar.into(),
                bar.into(),
                "".into(),
            ]))
            .highlight_spacing(HighlightSpacing::Always);

        StatefulWidget::render(t, area, buf, state);
    }
}

// Helper function to calculate constraints for table data
fn constraint_len_calculator_for_data(
    items: &[Vec<String>],
    column_names: &[String],
) -> Vec<u16> {
    let mut longest_lens = column_names
        .iter()
        .map(|name| u16::try_from(name.len()).unwrap_or(0))
        .collect::<Vec<u16>>();

    for item in items {
        for (i, value) in item.iter().enumerate() {
            if i < longest_lens.len()
                && let Ok(len) = u16::try_from(value.len())
            {
                longest_lens[i] = longest_lens[i].max(len);
            }
        }
    }

    longest_lens
}
