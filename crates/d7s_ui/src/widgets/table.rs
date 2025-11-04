use d7s_db::TableData;
use ratatui::{
    layout::{Constraint, Rect},
    style::{Modifier, Style},
    text::Text,
    widgets::{Cell, HighlightSpacing, Row, StatefulWidget, Table, TableState},
};

use crate::widgets::constraint_len_calculator;

/// A ratatui widget for displaying tabular data with selection and styling
#[derive(Clone, Debug)]
pub struct DataTable<T: TableData + Clone> {
    pub items: Vec<T>,
    pub longest_item_lens: Vec<u16>, // order is (name, address, email)
    pub table_state: TableState,
    pub column_offset: usize,
}

/// A ratatui widget for displaying table data with dynamic column headers
#[derive(Clone, Debug)]
pub struct TableDataWidget {
    pub items: Vec<Vec<String>>,
    pub column_names: Vec<String>,
    pub longest_item_lens: Vec<u16>,
    pub table_state: TableState,
    pub column_offset: usize,
}

impl Default for TableDataWidget {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            column_names: Vec::new(),
            longest_item_lens: Vec::new(),
            table_state: TableState::default().with_selected(0),
            column_offset: 0,
        }
    }
}

impl TableDataWidget {
    #[must_use]
    pub fn new(items: Vec<Vec<String>>, column_names: Vec<String>) -> Self {
        let longest_item_lens =
            constraint_len_calculator_for_data(&items, &column_names);
        Self {
            items,
            column_names,
            longest_item_lens,
            table_state: TableState::default().with_selected(0),
            column_offset: 0,
        }
    }

    #[must_use]
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

impl TableDataWidget {
    /// Adjusts column_offset to ensure the selected column is visible
    pub fn adjust_offset_for_selected_column(
        &mut self,
        selected_col: usize,
        area_width: u16,
    ) {
        // Calculate cumulative widths to determine visible range
        let mut cumulative_width = 0u16;
        let mut visible_end = self.column_offset;
        for (idx, &len) in self
            .longest_item_lens
            .iter()
            .enumerate()
            .skip(self.column_offset)
        {
            let col_width = len + 1; // Add 1 for padding
            if cumulative_width + col_width > area_width {
                break;
            }
            cumulative_width += col_width;
            visible_end = idx + 1;
        }

        // Adjust offset if selected column is not visible
        if selected_col < self.column_offset {
            self.column_offset = selected_col;
        } else if selected_col >= visible_end {
            // Scroll to show selected column - try to show it at the start
            self.column_offset = selected_col;
        }

        // Clamp offset to valid range
        if self.column_offset >= self.column_names.len() {
            self.column_offset = self.column_names.len().saturating_sub(1);
        }
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
        // Get absolute selected column index (None means no column selected)
        let selected_col_opt = state.selected_column();

        // If no column is selected, don't adjust offset or highlight columns
        let (visible_cols, relative_selected_col) = if let Some(selected_col) =
            selected_col_opt
        {
            // Adjust offset locally to ensure selected column is visible
            let mut eff_offset = self.column_offset;
            if selected_col < eff_offset {
                eff_offset = selected_col;
            } else {
                // Calculate if selected column is visible with current offset
                let mut cumulative_width = 0u16;
                let mut visible_end = eff_offset;
                for (idx, &len) in
                    self.longest_item_lens.iter().enumerate().skip(eff_offset)
                {
                    let col_width = len + 1;
                    if cumulative_width + col_width > area.width {
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
            if eff_offset >= self.column_names.len() {
                eff_offset = self.column_names.len().saturating_sub(1);
            }

            // Calculate visible columns with effective offset
            let mut vis_cols = Vec::new();
            let mut cumulative_width = 0u16;
            for (idx, &len) in
                self.longest_item_lens.iter().enumerate().skip(eff_offset)
            {
                let col_width = len + 1;
                if cumulative_width + col_width > area.width {
                    break;
                }
                cumulative_width += col_width;
                vis_cols.push(idx);
            }

            if vis_cols.is_empty() && !self.column_names.is_empty() {
                vis_cols.push(eff_offset.min(self.column_names.len() - 1));
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
            for (idx, &len) in self
                .longest_item_lens
                .iter()
                .enumerate()
                .skip(self.column_offset)
            {
                let col_width = len + 1;
                if cumulative_width + col_width > area.width {
                    break;
                }
                cumulative_width += col_width;
                vis_cols.push(idx);
            }

            if vis_cols.is_empty() && !self.column_names.is_empty() {
                vis_cols
                    .push(self.column_offset.min(self.column_names.len() - 1));
            }

            (vis_cols, None)
        };

        // Temporarily set relative column for rendering (None if no column selected)
        let original_col = state.selected_column();
        state.select_column(relative_selected_col);

        let selected_row_style = Style::default()
            .add_modifier(Modifier::REVERSED | Modifier::BOLD)
            .fg(ratatui::style::Color::Black)
            .bg(ratatui::style::Color::Yellow);
        let selected_col_style =
            Style::default().fg(ratatui::style::Color::Cyan);
        let selected_cell_style = Style::default()
            .add_modifier(Modifier::REVERSED)
            .fg(ratatui::style::Color::Magenta);

        // Only render visible columns
        let header = visible_cols
            .iter()
            .map(|&idx| Cell::from(self.column_names[idx].clone()))
            .collect::<Row>()
            .height(1);

        let rows = self.items.iter().map(|values| {
            visible_cols
                .iter()
                .map(|&idx| {
                    let value = values.get(idx).cloned().unwrap_or_default();
                    Cell::from(value)
                })
                .collect::<Row>()
                .style(Style::new())
                .height(1)
        });

        let bar: &'static str = " █ ";
        let constraints = visible_cols
            .iter()
            .map(|&idx| {
                let len = self.longest_item_lens[idx];
                Constraint::Length(len + 1) // Use Length instead of Min to prevent compression
            })
            .collect::<Vec<_>>();

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

        // Restore original column selection (absolute index)
        state.select_column(original_col);
    }
}

impl<T: TableData + Clone> Default for DataTable<T> {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            longest_item_lens: Vec::new(),
            table_state: TableState::default().with_selected(0),
            column_offset: 0,
        }
    }
}

impl<T: TableData + Clone> DataTable<T> {
    #[must_use]
    pub fn new(items: Vec<T>) -> Self {
        let longest_item_lens = constraint_len_calculator(&items);
        Self {
            items,
            longest_item_lens,
            table_state: TableState::default().with_selected(0),
            column_offset: 0,
        }
    }

    #[must_use]
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

impl<T: TableData + Clone> DataTable<T> {
    /// Adjusts column_offset to ensure the selected column is visible
    pub fn adjust_offset_for_selected_column(
        &mut self,
        selected_col: usize,
        area_width: u16,
    ) {
        if self.longest_item_lens.is_empty() {
            return;
        }

        // Calculate cumulative widths to determine visible range
        let mut cumulative_width = 0u16;
        let mut visible_end = self.column_offset;
        for (idx, &len) in self
            .longest_item_lens
            .iter()
            .enumerate()
            .skip(self.column_offset)
        {
            let col_width = len + 1; // Add 1 for padding
            if cumulative_width + col_width > area_width {
                break;
            }
            cumulative_width += col_width;
            visible_end = idx + 1;
        }

        // Adjust offset if selected column is not visible
        if selected_col < self.column_offset {
            self.column_offset = selected_col;
        } else if selected_col >= visible_end {
            // Scroll to show selected column - try to show it at the start
            self.column_offset = selected_col;
        }

        // Clamp offset to valid range
        if self.column_offset >= self.longest_item_lens.len() {
            self.column_offset = self.longest_item_lens.len().saturating_sub(1);
        }
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
        if self.longest_item_lens.is_empty() {
            return;
        }

        // Get absolute selected column index (None means no column selected)
        let selected_col_opt = state.selected_column();

        // If no column is selected, don't adjust offset or highlight columns
        let (visible_cols, relative_selected_col) = if let Some(selected_col) =
            selected_col_opt
        {
            // Adjust offset locally to ensure selected column is visible
            let mut eff_offset = self.column_offset;
            if selected_col < eff_offset {
                eff_offset = selected_col;
            } else {
                // Calculate if selected column is visible with current offset
                let mut cumulative_width = 0u16;
                let mut visible_end = eff_offset;
                for (idx, &len) in
                    self.longest_item_lens.iter().enumerate().skip(eff_offset)
                {
                    let col_width = len + 1;
                    if cumulative_width + col_width > area.width {
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
            if eff_offset >= self.longest_item_lens.len() {
                eff_offset = self.longest_item_lens.len().saturating_sub(1);
            }

            // Calculate visible columns with effective offset
            let mut vis_cols = Vec::new();
            let mut cumulative_width = 0u16;
            for (idx, &len) in
                self.longest_item_lens.iter().enumerate().skip(eff_offset)
            {
                let col_width = len + 1;
                if cumulative_width + col_width > area.width {
                    break;
                }
                cumulative_width += col_width;
                vis_cols.push(idx);
            }

            if vis_cols.is_empty() {
                vis_cols.push(eff_offset.min(self.longest_item_lens.len() - 1));
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
            for (idx, &len) in self
                .longest_item_lens
                .iter()
                .enumerate()
                .skip(self.column_offset)
            {
                let col_width = len + 1;
                if cumulative_width + col_width > area.width {
                    break;
                }
                cumulative_width += col_width;
                vis_cols.push(idx);
            }

            if vis_cols.is_empty() {
                vis_cols.push(
                    self.column_offset.min(self.longest_item_lens.len() - 1),
                );
            }

            (vis_cols, None)
        };

        // Temporarily set relative column for rendering (None if no column selected)
        let original_col = state.selected_column();
        state.select_column(relative_selected_col);

        let selected_row_style = Style::default()
            .add_modifier(Modifier::REVERSED | Modifier::BOLD)
            .fg(ratatui::style::Color::Black)
            .bg(ratatui::style::Color::Yellow);
        let selected_col_style =
            Style::default().fg(ratatui::style::Color::Cyan);
        let selected_cell_style = Style::default()
            .add_modifier(Modifier::REVERSED)
            .fg(ratatui::style::Color::Magenta);

        // Get column names from the trait
        let all_cols = T::cols();

        // Only render visible columns
        let header = visible_cols
            .iter()
            .map(|&idx| {
                let col_name = all_cols.get(idx).cloned().unwrap_or_default();
                Cell::from(col_name)
            })
            .collect::<Row>()
            .height(1);

        let rows = self.items.iter().map(|data| {
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

        let bar: &'static str = " █ ";
        let constraints = visible_cols
            .iter()
            .map(|&idx| {
                let len = self.longest_item_lens[idx];
                Constraint::Length(len + 1) // Use Length instead of Min to prevent compression
            })
            .collect::<Vec<_>>();

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

        // Restore original column selection (absolute index)
        state.select_column(original_col);
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
