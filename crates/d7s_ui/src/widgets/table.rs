use std::sync::Arc;

use d7s_db::TableData;
use ratatui::{
    layout::{Constraint, Rect},
    style::{Modifier, Style},
    text::Text,
    widgets::{Cell, HighlightSpacing, Row, StatefulWidget, Table, TableState},
};

use crate::widgets::constraint_len_calculator;

/// A wrapper type for raw table data with dynamic column names
#[derive(Clone, Debug, Default)]
pub struct RawTableRow {
    pub values: Vec<String>,
    pub column_names: Arc<Vec<String>>,
}

impl TableData for RawTableRow {
    fn title() -> &'static str {
        "Table Data"
    }

    fn ref_array(&self) -> Vec<String> {
        self.values.clone()
    }

    fn num_columns(&self) -> usize {
        self.values.len()
    }

    fn cols() -> Vec<&'static str> {
        // This is a limitation - we can't return dynamic column names from a static method
        // We'll handle this specially in DataTable's render method
        vec![]
    }
}

/// Model (data) for the table view
#[derive(Clone, Debug, Default)]
pub struct TableModel<T: TableData + Clone> {
    pub items: Vec<T>,
    pub longest_item_lens: Vec<u16>,
    // For RawTableRow, we store column names here
    pub dynamic_column_names: Option<Arc<Vec<String>>>,
}

/// View state for the table (UI state like selection, scrolling)
#[derive(Clone, Debug, Default)]
pub struct TableViewState {
    pub state: TableState,
    pub column_offset: usize,
}

/// Table widget wrapper that holds the model and state
#[derive(Clone, Debug, Default)]
pub struct DataTable<T: TableData + Clone> {
    pub model: TableModel<T>,
    pub view: TableViewState,
}

impl DataTable<RawTableRow> {
    pub fn reset(&mut self, items: Vec<Vec<String>>, column_names: &[String]) {
        let column_names_arc = Arc::new(column_names.to_owned());
        let raw_rows: Vec<RawTableRow> = items
            .into_iter()
            .map(|values| RawTableRow {
                values,
                column_names: Arc::clone(&column_names_arc),
            })
            .collect();
        let longest_item_lens =
            constraint_len_calculator_for_raw_data(&raw_rows, column_names);

        self.model.items = raw_rows;
        self.model.longest_item_lens = longest_item_lens;
        self.model.dynamic_column_names = Some(column_names_arc);
        self.view.state.select(Some(0));
        self.view.column_offset = 0;
    }
}

impl<T: TableData + Clone> DataTable<T> {
    #[must_use]
    pub fn new(items: Vec<T>) -> Self {
        let longest_item_lens = constraint_len_calculator(&items);
        Self {
            model: TableModel {
                items,
                longest_item_lens,
                dynamic_column_names: None,
            },
            view: TableViewState {
                state: TableState::default().with_selected(0),
                column_offset: 0,
            },
        }
    }

    #[must_use]
    pub fn filter(&self, query: &str) -> Vec<T> {
        if query.is_empty() {
            return self.model.items.clone();
        }

        let query_lower = query.to_lowercase();
        self.model
            .items
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

    /// Adjusts `column_offset` to ensure the selected column is visible
    pub fn adjust_offset_for_selected_column(
        &mut self,
        selected_col: usize,
        area_width: u16,
    ) {
        if self.model.longest_item_lens.is_empty() {
            return;
        }

        // Calculate cumulative widths to determine visible range
        let mut cumulative_width = 0u16;
        let mut visible_end = self.view.column_offset;
        for (idx, &len) in self
            .model
            .longest_item_lens
            .iter()
            .enumerate()
            .skip(self.view.column_offset)
        {
            let col_width = len + 1; // Add 1 for padding
            if cumulative_width + col_width > area_width {
                break;
            }
            cumulative_width += col_width;
            visible_end = idx + 1;
        }

        // Adjust offset if selected column is not visible
        if selected_col < self.view.column_offset {
            self.view.column_offset = selected_col;
        } else if selected_col >= visible_end {
            // Scroll to show selected column - try to show it at the start
            self.view.column_offset = selected_col;
        }

        // Clamp offset to valid range
        if self.view.column_offset >= self.model.longest_item_lens.len() {
            self.view.column_offset =
                self.model.longest_item_lens.len().saturating_sub(1);
        }
    }
}

/// Helper function to calculate visible columns for DataTable
fn calculate_visible_columns_for_table(
    longest_item_lens: &[u16],
    column_offset: usize,
    selected_col_opt: Option<usize>,
    area_width: u16,
) -> (Vec<usize>, Option<usize>) {
    if let Some(selected_col) = selected_col_opt {
        // Adjust offset locally to ensure selected column is visible
        let mut eff_offset = column_offset;
        if selected_col < eff_offset {
            eff_offset = selected_col;
        } else {
            // Calculate if selected column is visible with current offset
            let mut cumulative_width = 0u16;
            let mut visible_end = eff_offset;
            for (idx, &len) in
                longest_item_lens.iter().enumerate().skip(eff_offset)
            {
                let col_width = len + 1;
                if cumulative_width + col_width > area_width {
                    break;
                }
                cumulative_width += col_width;
                visible_end = idx + 1;
            }
            if selected_col >= visible_end {
                eff_offset = selected_col;
            }
        }

        // Clamp effective offset
        if eff_offset >= longest_item_lens.len() {
            eff_offset = longest_item_lens.len().saturating_sub(1);
        }

        // Calculate visible columns with effective offset
        let mut vis_cols = Vec::new();
        let mut cumulative_width = 0u16;
        for (idx, &len) in longest_item_lens.iter().enumerate().skip(eff_offset)
        {
            let col_width = len + 1;
            if cumulative_width + col_width > area_width {
                break;
            }
            cumulative_width += col_width;
            vis_cols.push(idx);
        }

        if vis_cols.is_empty() {
            vis_cols.push(eff_offset.min(longest_item_lens.len() - 1));
        }

        // Find relative position of selected column in visible columns
        let rel_selected_col = vis_cols
            .iter()
            .position(|&idx| idx == selected_col)
            .unwrap_or(0);

        (vis_cols, Some(rel_selected_col))
    } else {
        // No column selected - just calculate visible columns from current offset
        let mut vis_cols = Vec::new();
        let mut cumulative_width = 0u16;
        for (idx, &len) in
            longest_item_lens.iter().enumerate().skip(column_offset)
        {
            let col_width = len + 1;
            if cumulative_width + col_width > area_width {
                break;
            }
            cumulative_width += col_width;
            vis_cols.push(idx);
        }

        if vis_cols.is_empty() {
            vis_cols.push(column_offset.min(longest_item_lens.len() - 1));
        }

        (vis_cols, None)
    }
}

// pub struct TableWidget;

impl<T: TableData + std::fmt::Debug + Clone> StatefulWidget for DataTable<T> {
    type State = TableViewState;

    fn render(
        self,
        area: Rect,
        buf: &mut ratatui::buffer::Buffer,
        state: &mut Self::State,
    ) {
        if self.model.longest_item_lens.is_empty() {
            return;
        }

        let selected_col_opt = state.state.selected_column();
        let (visible_cols, relative_selected_col) =
            calculate_visible_columns_for_table(
                &self.model.longest_item_lens,
                state.column_offset,
                selected_col_opt,
                area.width,
            );

        let original_col = state.state.selected_column();
        state.state.select_column(relative_selected_col);

        let (
            selected_row_style,
            selected_col_style,
            selected_cell_style,
            highlight_symbol,
            highlight_spacing,
        ) = create_table_styles();

        // Use dynamic column names if available (for RawTableRow), otherwise use static cols()
        let header = self.model.dynamic_column_names.as_ref().map_or_else(
            || {
                let all_cols = T::cols();
                visible_cols
                    .iter()
                    .map(|&idx| {
                        let col_name =
                            all_cols.get(idx).copied().unwrap_or_default();
                        Cell::from(col_name)
                    })
                    .collect::<Row>()
                    .height(1)
            },
            |dyn_cols| {
                visible_cols
                    .iter()
                    .map(|&idx| {
                        let col_name =
                            dyn_cols.get(idx).cloned().unwrap_or_default();
                        Cell::from(col_name)
                    })
                    .collect::<Row>()
                    .height(1)
            },
        );

        let rows = self.model.items.iter().map(|data| {
            let row_data = data.ref_array();
            visible_cols
                .iter()
                .map(|&idx| {
                    let value = row_data.get(idx).cloned().unwrap_or_default();
                    Cell::from(value)
                })
                .collect::<Row>()
                .style(Style::new())
                .height(1)
        });

        let constraints = visible_cols
            .iter()
            .map(|&idx| {
                Constraint::Length(self.model.longest_item_lens[idx] + 1)
            })
            .collect::<Vec<_>>();

        let t = Table::new(rows, constraints)
            .header(header)
            .row_highlight_style(selected_row_style)
            .column_highlight_style(selected_col_style)
            .cell_highlight_style(selected_cell_style)
            .highlight_symbol(highlight_symbol)
            .highlight_spacing(highlight_spacing);

        StatefulWidget::render(t, area, buf, &mut state.state);
        state.state.select_column(original_col);
    }
}

// Helper function to calculate constraints for raw table data
fn constraint_len_calculator_for_raw_data(
    items: &[RawTableRow],
    column_names: &[String],
) -> Vec<u16> {
    use unicode_width::UnicodeWidthStr;

    let mut longest_lens = column_names
        .iter()
        .map(|name| {
            u16::try_from(UnicodeWidthStr::width(name.as_str())).unwrap_or(0)
        })
        .collect::<Vec<u16>>();

    for item in items {
        for (i, value) in item.values.iter().enumerate() {
            if i < longest_lens.len() {
                let max_width = value
                    .lines()
                    .map(UnicodeWidthStr::width)
                    .max()
                    .unwrap_or(0);
                if let Ok(len) = u16::try_from(max_width) {
                    longest_lens[i] = longest_lens[i].max(len);
                }
            }
        }
    }

    longest_lens
}

// Helper function to create table styles
fn create_table_styles()
-> (Style, Style, Style, Text<'static>, HighlightSpacing) {
    let selected_row_style = Style::default()
        .add_modifier(Modifier::REVERSED | Modifier::BOLD)
        .fg(ratatui::style::Color::Black)
        .bg(ratatui::style::Color::Yellow);
    let selected_col_style = Style::default().fg(ratatui::style::Color::Cyan);
    let selected_cell_style = Style::default()
        .add_modifier(Modifier::REVERSED)
        .fg(ratatui::style::Color::Magenta);
    let bar: &'static str = " █ ";
    let highlight_symbol =
        Text::from(vec!["".into(), bar.into(), bar.into(), "".into()]);
    (
        selected_row_style,
        selected_col_style,
        selected_cell_style,
        highlight_symbol,
        HighlightSpacing::Always,
    )
}
