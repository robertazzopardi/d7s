use d7s_db::TableData;
use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    style::{Modifier, Style},
    text::Text,
    widgets::{
        Cell, HighlightSpacing, Row, StatefulWidget, Table, TableState, Widget,
    },
};
use unicode_width::UnicodeWidthStr;

/// A ratatui widget for displaying tabular data with selection and styling
pub struct DataTable<T: TableData> {
    items: Vec<T>,
    longest_item_lens: Vec<u16>, // order is (name, address, email)
}

impl<T: TableData> DataTable<T> {
    pub fn new(items: Vec<T>) -> Self {
        let longest_item_lens = constraint_len_calculator(&items);
        Self {
            items,
            longest_item_lens,
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
        let header_style = Style::default();
        let selected_row_style =
            Style::default().add_modifier(Modifier::REVERSED);
        let selected_col_style = Style::default();
        let selected_cell_style =
            Style::default().add_modifier(Modifier::REVERSED);

        let header = T::cols()
            .into_iter()
            .map(Cell::from)
            .collect::<Row>()
            .style(header_style)
            .height(1);

        let rows = self.items.iter().enumerate().map(|(i, data)| {
            let item = data.ref_array();
            item.into_iter()
                .map(|content| Cell::from(content.to_string()))
                .collect::<Row>()
                .style(Style::new())
                .height(1)
        });

        let bar: &'static str = " █ ";
        let constraints = self
            .longest_item_lens
            .into_iter()
            .map(|len| Constraint::Fill(len));
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

        Widget::render(t, area, buf);
    }
}

/// State management for the table widget
pub struct TableWidgetState {
    pub state: TableState,
}

impl TableWidgetState {
    pub fn new(items_len: usize) -> Self {
        Self {
            state: TableState::default().with_selected(0),
        }
    }

    pub fn next_row(&mut self, items_len: usize) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= items_len - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn previous_row(&mut self, items_len: usize) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    items_len - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn next_column(&mut self) {
        self.state.select_next_column();
    }

    pub fn previous_column(&mut self) {
        self.state.select_previous_column();
    }
}

/// A complete table view that combines the table widget with state management
pub struct TableView<T: TableData + Clone> {
    state: TableWidgetState,
    items: Vec<T>,
}

impl<T: TableData + Clone> TableView<T> {
    pub fn new() -> Self
    where
        T: Default,
    {
        Self {
            state: TableWidgetState::new(0),
            items: Vec::new(),
        }
    }

    pub fn with_items(items: Vec<T>) -> Self {
        Self {
            state: TableWidgetState::new(items.len()),
            items,
        }
    }
}

impl<T: TableData + Clone + std::fmt::Debug> TableView<T> {
    pub fn next_row(&mut self) {
        self.state.next_row(self.items.len());
    }

    pub fn previous_row(&mut self) {
        self.state.previous_row(self.items.len());
    }

    pub fn next_column(&mut self) {
        self.state.next_column();
    }

    pub fn previous_column(&mut self) {
        self.state.previous_column();
    }

    pub fn state(&mut self) -> &mut TableWidgetState {
        &mut self.state
    }

    pub fn items(&self) -> &[T] {
        &self.items
    }

    pub fn items_mut(&mut self) -> &mut Vec<T> {
        &mut self.items
    }

    pub fn draw(&mut self, frame: &mut Frame, area: Rect) {
        let table_widget = DataTable::new(self.items.clone());
        frame.render_stateful_widget(table_widget, area, &mut self.state.state);
    }
}

fn constraint_len_calculator<T: TableData>(items: &[T]) -> Vec<u16> {
    if items.is_empty() {
        return Vec::new();
    }

    let num_columns = items[0].num_columns();

    (0..num_columns)
        .map(|col_idx| {
            items
                .iter()
                .flat_map(|data| data.col(col_idx).lines())
                .map(UnicodeWidthStr::width)
                .max()
                .unwrap_or(0) as u16
        })
        .collect()
}
