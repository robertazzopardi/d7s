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
pub struct DataTable<T: TableData> {
    pub items: Vec<T>,
    pub longest_item_lens: Vec<u16>, // order is (name, address, email)
    pub table_state: TableState,
}

impl<T: TableData> DataTable<T> {
    pub fn new(items: Vec<T>) -> Self {
        let longest_item_lens = constraint_len_calculator(&items);
        Self {
            items,
            longest_item_lens,
            table_state: TableState::default().with_selected(0),
        }
    }
}

impl<T: TableData + std::fmt::Debug> StatefulWidget for DataTable<T> {
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

        let rows = self.items.iter().enumerate().map(|(i, data)| {
            let item = data.ref_array();
            item.into_iter()
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
