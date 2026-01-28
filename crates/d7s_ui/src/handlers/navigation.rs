use crossterm::event::KeyCode;
use d7s_db::TableData;

use crate::widgets::table::DataTable;

/// Helper for table navigation operations
pub struct TableNavigationHandler;

impl TableNavigationHandler {
    /// Wraps the selection for a `DataTable` - going past the end wraps to the beginning and vice versa
    pub fn wrap_rows<T: TableData + Clone>(table: &mut DataTable<T>) {
        if let Some(selected) = table.view.state.selected() {
            if table.model.items.is_empty() {
                table.view.state.select(None);
            } else if selected == table.model.items.len() {
                // Past the end - wrap to beginning
                table.view.state.select_first();
            } else if selected > table.model.items.len() {
                // Underflow (wrapped from 0) - wrap to end
                table.view.state.select_last();
            }
        }
    }

    /// Wraps the column selection for a `DataTable` - going past the end wraps to the beginning and vice versa
    pub fn wrap_columns<T: TableData + Clone>(table: &mut DataTable<T>) {
        let num_columns = table
            .model
            .items
            .first()
            .map_or_else(|| 0, TableData::num_columns);

        if num_columns == 0 {
            table.view.state.select_column(None);
            table.view.column_offset = 0;
            return;
        }

        // Wrap selected column
        if let Some(selected_col) = table.view.state.selected_column()
            && selected_col >= num_columns
        {
            // Past the end or underflow - wrap to beginning
            table.view.state.select_column(Some(0));
        }

        // Clamp column offset (offset doesn't wrap, just clamps)
        if table.view.column_offset >= num_columns {
            table.view.column_offset = num_columns.saturating_sub(1);
        }
    }

    /// Generic table navigation handler for any `DataTable`
    pub fn navigate_table<T: TableData + Clone>(
        table: &mut DataTable<T>,
        key: KeyCode,
    ) {
        match key {
            KeyCode::Char('j') | KeyCode::Down => {
                if let Some(selected) = table.view.state.selected()
                    && !table.model.items.is_empty()
                {
                    if selected >= table.model.items.len() - 1 {
                        table.view.state.select(Some(0));
                    } else {
                        table.view.state.select_next();
                    }
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if let Some(selected) = table.view.state.selected()
                    && !table.model.items.is_empty()
                {
                    if selected == 0 {
                        table.view.state.select(Some(table.model.items.len() - 1));
                    } else {
                        table.view.state.select_previous();
                    }
                }
            }
            KeyCode::Char('h' | 'b') | KeyCode::Left => {
                let num_cols =
                    table.model.items.first().map_or(0, TableData::num_columns);
                if num_cols == 0 {
                    return;
                }

                if let Some(selected_col) = table.view.state.selected_column() {
                    if selected_col == 0 {
                        table.view.state.select_column(Some(num_cols - 1));
                    } else {
                        table.view.state.select_previous_column();
                    }
                } else {
                    table.view.state.select_column(Some(num_cols - 1));
                }

                if let Some(selected_col) = table.view.state.selected_column() {
                    table.adjust_offset_for_selected_column(selected_col, 80);
                }
            }
            KeyCode::Char('l' | 'w') | KeyCode::Right => {
                let num_cols =
                    table.model.items.first().map_or(0, TableData::num_columns);
                if num_cols == 0 {
                    return;
                }

                if let Some(selected_col) = table.view.state.selected_column() {
                    if selected_col + 1 >= num_cols {
                        table.view.state.select_column(Some(0));
                    } else {
                        table.view.state.select_next_column();
                    }
                } else {
                    table.view.state.select_column(Some(0));
                }

                if let Some(selected_col) = table.view.state.selected_column() {
                    table.adjust_offset_for_selected_column(selected_col, 80);
                }
            }
            KeyCode::Char('g') => {
                table.view.state.select(Some(0));
                Self::wrap_rows(table);
                table.view.column_offset = 0;
            }
            KeyCode::Char('G') => {
                if !table.model.items.is_empty() {
                    table.view.state.select(Some(table.model.items.len() - 1));
                }
            }
            _ => {}
        }
    }

    /// Handles navigation for table data widget
    pub fn navigate<T: TableData + Clone>(
        table_data: &mut Option<DataTable<T>>,
        key: KeyCode,
    ) {
        if let Some(table) = table_data {
            Self::navigate_table(table, key);
        }
    }
}
