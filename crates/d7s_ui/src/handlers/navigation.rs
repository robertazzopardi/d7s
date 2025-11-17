use crossterm::event::KeyCode;
use d7s_db::{Column, Schema, Table, TableData};

use crate::widgets::{
    sql_executor::SqlExecutor,
    table::{DataTable, TableDataWidget},
};

/// Helper for table navigation operations
pub struct TableNavigationHandler;

impl TableNavigationHandler {
    /// Clamps the selection for a `DataTable` to valid bounds
    pub fn clamp_data_table_selection<T: TableData + Clone>(
        table: &mut DataTable<T>,
    ) {
        if let Some(selected) = table.table_state.selected()
            && selected >= table.items.len()
        {
            if table.items.is_empty() {
                table.table_state.select(None);
            } else {
                table.table_state.select(Some(table.items.len() - 1));
            }
        }
    }

    /// Clamps the selection for a `TableDataWidget` to valid bounds
    pub fn clamp_table_data_selection(table_data: &mut TableDataWidget) {
        if let Some(selected) = table_data.table_state.selected()
            && selected >= table_data.items.len()
        {
            if table_data.items.is_empty() {
                table_data.table_state.select(None);
            } else {
                table_data
                    .table_state
                    .select(Some(table_data.items.len() - 1));
            }
        }
    }

    /// Clamps the column selection and offset for a `DataTable` to valid bounds
    pub fn clamp_data_table_columns<T: TableData + Clone>(
        table: &mut DataTable<T>,
    ) {
        let num_columns = table
            .items
            .first()
            .map_or(0, d7s_db::TableData::num_columns);

        // Clamp selected column
        if let Some(selected_col) = table.table_state.selected_column()
            && selected_col >= num_columns
        {
            if num_columns == 0 {
                table.table_state.select_column(None);
            } else {
                table
                    .table_state
                    .select_column(Some(num_columns.saturating_sub(1)));
            }
        }

        // Clamp column offset
        if num_columns == 0 {
            table.column_offset = 0;
        } else if table.column_offset >= num_columns {
            table.column_offset = num_columns.saturating_sub(1);
        }
    }

    /// Clamps the column selection and offset for a `TableDataWidget` to valid bounds
    pub fn clamp_table_data_columns(table_data: &mut TableDataWidget) {
        let num_columns = table_data.column_names.len();

        // Clamp selected column
        if let Some(selected_col) = table_data.table_state.selected_column()
            && selected_col >= num_columns
        {
            if num_columns == 0 {
                table_data.table_state.select_column(None);
            } else {
                table_data
                    .table_state
                    .select_column(Some(num_columns.saturating_sub(1)));
            }
        }

        // Clamp column offset
        if num_columns == 0 {
            table_data.column_offset = 0;
        } else if table_data.column_offset >= num_columns {
            table_data.column_offset = num_columns.saturating_sub(1);
        }
    }

    /// Handles navigation for schema table
    pub fn handle_schema_table_navigation(
        schema_table: &mut Option<DataTable<Schema>>,
        key: KeyCode,
    ) {
        if let Some(schema_table) = schema_table {
            Self::handle_data_table_navigation(schema_table, key);
        }
    }

    /// Handles navigation for table table
    pub fn handle_table_table_navigation(
        table_table: &mut Option<DataTable<Table>>,
        key: KeyCode,
    ) {
        if let Some(table_table) = table_table {
            Self::handle_data_table_navigation(table_table, key);
        }
    }

    /// Handles navigation for column table
    pub fn handle_column_table_navigation(
        column_table: &mut Option<DataTable<Column>>,
        key: KeyCode,
    ) {
        if let Some(column_table) = column_table {
            Self::handle_data_table_navigation(column_table, key);
        }
    }

    /// Handles navigation for table data widget
    pub fn handle_table_data_navigation(
        table_data: &mut Option<TableDataWidget>,
        key: KeyCode,
    ) {
        if let Some(table_data) = table_data {
            match key {
                KeyCode::Char('j') | KeyCode::Down => {
                    table_data.table_state.select_next();
                    Self::clamp_table_data_selection(table_data);
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    table_data.table_state.select_previous();
                    Self::clamp_table_data_selection(table_data);
                }
                KeyCode::Char('h' | 'b') | KeyCode::Left => {
                    // If no column is selected, start with the last column, otherwise navigate
                    if table_data.table_state.selected_column().is_none() {
                        let num_cols = table_data.column_names.len();
                        if num_cols > 0 {
                            table_data.table_state.select_column(Some(
                                num_cols.saturating_sub(1),
                            ));
                        }
                    } else {
                        table_data.table_state.select_previous_column();
                    }
                    Self::clamp_table_data_columns(table_data);
                    // Adjust offset to ensure selected column is visible
                    if let Some(selected_col) =
                        table_data.table_state.selected_column()
                    {
                        // Use a reasonable default area width (will be refined in render)
                        table_data.adjust_offset_for_selected_column(
                            selected_col,
                            80,
                        );
                    }
                }
                KeyCode::Char('l' | 'w') | KeyCode::Right => {
                    // If no column is selected, start with the first column, otherwise navigate
                    if table_data.table_state.selected_column().is_none() {
                        table_data.table_state.select_column(Some(0));
                    } else {
                        table_data.table_state.select_next_column();
                    }
                    Self::clamp_table_data_columns(table_data);
                    // Adjust offset to ensure selected column is visible
                    if let Some(selected_col) =
                        table_data.table_state.selected_column()
                    {
                        // Use a reasonable default area width (will be refined in render)
                        table_data.adjust_offset_for_selected_column(
                            selected_col,
                            80,
                        );
                    }
                }
                KeyCode::Char('g') => {
                    table_data.table_state.select(Some(0));
                    Self::clamp_table_data_selection(table_data);
                    // Reset offset when going to first row
                    table_data.column_offset = 0;
                }
                KeyCode::Char('G') => {
                    if !table_data.items.is_empty() {
                        table_data
                            .table_state
                            .select(Some(table_data.items.len() - 1));
                    }
                }
                _ => {}
            }
        }
    }

    /// Handles navigation for SQL executor results
    pub fn handle_sql_results_navigation(
        sql_executor: &mut SqlExecutor,
        key: KeyCode,
    ) {
        if let Some(table_widget) = &mut sql_executor.table_widget {
            match key {
                KeyCode::Char('j') | KeyCode::Down => {
                    table_widget.table_state.select_next();
                    Self::clamp_table_data_selection(table_widget);
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    table_widget.table_state.select_previous();
                    Self::clamp_table_data_selection(table_widget);
                }
                KeyCode::Char('h' | 'b') | KeyCode::Left => {
                    // If no column is selected, start with the last column, otherwise navigate
                    if table_widget.table_state.selected_column().is_none() {
                        let num_cols = table_widget.column_names.len();
                        if num_cols > 0 {
                            table_widget.table_state.select_column(Some(
                                num_cols.saturating_sub(1),
                            ));
                        }
                    } else {
                        table_widget.table_state.select_previous_column();
                    }
                    Self::clamp_table_data_columns(table_widget);
                    // Adjust offset to ensure selected column is visible
                    if let Some(selected_col) =
                        table_widget.table_state.selected_column()
                    {
                        // Use a reasonable default area width (will be refined in render)
                        table_widget.adjust_offset_for_selected_column(
                            selected_col,
                            80,
                        );
                    }
                }
                KeyCode::Char('l' | 'w') | KeyCode::Right => {
                    // If no column is selected, start with the first column, otherwise navigate
                    if table_widget.table_state.selected_column().is_none() {
                        table_widget.table_state.select_column(Some(0));
                    } else {
                        table_widget.table_state.select_next_column();
                    }
                    Self::clamp_table_data_columns(table_widget);
                    // Adjust offset to ensure selected column is visible
                    if let Some(selected_col) =
                        table_widget.table_state.selected_column()
                    {
                        // Use a reasonable default area width (will be refined in render)
                        table_widget.adjust_offset_for_selected_column(
                            selected_col,
                            80,
                        );
                    }
                }
                KeyCode::Char('g') => {
                    table_widget.table_state.select(Some(0));
                    Self::clamp_table_data_selection(table_widget);
                    // Reset offset when going to first row
                    table_widget.column_offset = 0;
                }
                KeyCode::Char('G') => {
                    if !table_widget.items.is_empty() {
                        table_widget
                            .table_state
                            .select(Some(table_widget.items.len() - 1));
                    }
                }
                _ => {}
            }
        }
    }

    /// Generic handler for `DataTable` navigation
    fn handle_data_table_navigation<T: TableData + Clone>(
        table: &mut DataTable<T>,
        key: KeyCode,
    ) {
        match key {
            KeyCode::Char('j') | KeyCode::Down => {
                table.table_state.select_next();
                Self::clamp_data_table_selection(table);
            }
            KeyCode::Char('k') | KeyCode::Up => {
                table.table_state.select_previous();
                Self::clamp_data_table_selection(table);
            }
            KeyCode::Char('h' | 'b') | KeyCode::Left => {
                // If no column is selected, start with the last column, otherwise navigate
                if table.table_state.selected_column().is_none() {
                    let num_cols = table
                        .items
                        .first()
                        .map_or(0, d7s_db::TableData::num_columns);
                    if num_cols > 0 {
                        table
                            .table_state
                            .select_column(Some(num_cols.saturating_sub(1)));
                    }
                } else {
                    table.table_state.select_previous_column();
                }
                Self::clamp_data_table_columns(table);
                // Adjust offset to ensure selected column is visible
                if let Some(selected_col) = table.table_state.selected_column()
                {
                    // Use a reasonable default area width (will be refined in render)
                    table.adjust_offset_for_selected_column(selected_col, 80);
                }
            }
            KeyCode::Char('l' | 'w') | KeyCode::Right => {
                // If no column is selected, start with the first column, otherwise navigate
                if table.table_state.selected_column().is_none() {
                    table.table_state.select_column(Some(0));
                } else {
                    table.table_state.select_next_column();
                }
                Self::clamp_data_table_columns(table);
                // Adjust offset to ensure selected column is visible
                if let Some(selected_col) = table.table_state.selected_column()
                {
                    // Use a reasonable default area width (will be refined in render)
                    table.adjust_offset_for_selected_column(selected_col, 80);
                }
            }
            KeyCode::Char('g') => {
                table.table_state.select(Some(0));
                Self::clamp_data_table_selection(table);
                // Reset offset when going to first row
                table.column_offset = 0;
            }
            KeyCode::Char('G') => {
                if !table.items.is_empty() {
                    table.table_state.select(Some(table.items.len() - 1));
                }
            }
            _ => {}
        }
    }
}
