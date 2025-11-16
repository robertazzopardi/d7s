use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use d7s_db::TableData;

use super::navigation::TableNavigationHandler;
use crate::widgets::{
    search_filter::SearchFilter, sql_executor::SqlExecutor, table::DataTable,
};

/// Handles search filter key events
pub fn handle_search_filter_input(
    key: KeyEvent,
    search_filter: &mut SearchFilter,
    on_filter_change: &mut dyn FnMut(),
) -> bool {
    match (key.modifiers, key.code) {
        (_, KeyCode::Esc | KeyCode::Enter) => {
            search_filter.deactivate();
            true
        }
        (_, KeyCode::Char(ch)) if !ch.is_control() => {
            search_filter.add_char(ch);
            on_filter_change();
            true
        }
        (_, KeyCode::Backspace) => {
            search_filter.delete_char();
            on_filter_change();
            true
        }
        (_, KeyCode::Left) => {
            search_filter.move_cursor_left();
            true
        }
        (_, KeyCode::Right) => {
            search_filter.move_cursor_right();
            true
        }
        (KeyModifiers::CONTROL, KeyCode::Char('a')) => {
            search_filter.move_cursor_to_start();
            true
        }
        (KeyModifiers::CONTROL, KeyCode::Char('e')) => {
            search_filter.move_cursor_to_end();
            true
        }
        (KeyModifiers::CONTROL, KeyCode::Char('u')) => {
            search_filter.clear();
            on_filter_change();
            true
        }
        _ => false,
    }
}

/// Handles SQL executor key events
pub fn handle_sql_executor_input(
    key: KeyEvent,
    sql_executor: &mut SqlExecutor,
) -> bool {
    match (key.modifiers, key.code) {
        (_, KeyCode::Char(ch)) if !ch.is_control() => {
            sql_executor.add_char(ch);
            true
        }
        (_, KeyCode::Backspace) => {
            sql_executor.delete_char();
            true
        }
        (_, KeyCode::Left) => {
            sql_executor.move_cursor_left();
            true
        }
        (_, KeyCode::Right) => {
            sql_executor.move_cursor_right();
            true
        }
        (KeyModifiers::CONTROL, KeyCode::Char('a')) => {
            sql_executor.move_cursor_to_start();
            true
        }
        (KeyModifiers::CONTROL, KeyCode::Char('e')) => {
            sql_executor.move_cursor_to_end();
            true
        }
        (KeyModifiers::CONTROL, KeyCode::Char('u')) => {
            sql_executor.clear();
            true
        }
        _ => false,
    }
}

/// Handles connection list navigation keys
pub fn handle_connection_list_navigation<T: TableData + Clone>(
    key: KeyCode,
    table_widget: &mut DataTable<T>,
) {
    match key {
        KeyCode::Char('j') | KeyCode::Down => {
            table_widget.table_state.select_next();
            TableNavigationHandler::clamp_data_table_selection(table_widget);
        }
        KeyCode::Char('k') | KeyCode::Up => {
            table_widget.table_state.select_previous();
            TableNavigationHandler::clamp_data_table_selection(table_widget);
        }
        KeyCode::Char('h' | 'b') | KeyCode::Left => {
            // If no column is selected, start with the last column, otherwise navigate
            if table_widget.table_state.selected_column().is_none() {
                let num_cols = table_widget
                    .items
                    .first()
                    .map_or(0, d7s_db::TableData::num_columns);
                if num_cols > 0 {
                    table_widget
                        .table_state
                        .select_column(Some(num_cols.saturating_sub(1)));
                }
            } else {
                table_widget.table_state.select_previous_column();
            }
            TableNavigationHandler::clamp_data_table_columns(table_widget);
            // Adjust offset to ensure selected column is visible
            if let Some(selected_col) =
                table_widget.table_state.selected_column()
            {
                // Use a reasonable default area width (will be refined in render)
                table_widget
                    .adjust_offset_for_selected_column(selected_col, 80);
            }
        }
        KeyCode::Char('l' | 'w') | KeyCode::Right => {
            // If no column is selected, start with the first column, otherwise navigate
            if table_widget.table_state.selected_column().is_none() {
                table_widget.table_state.select_column(Some(0));
            } else {
                table_widget.table_state.select_next_column();
            }
            TableNavigationHandler::clamp_data_table_columns(table_widget);
            // Adjust offset to ensure selected column is visible
            if let Some(selected_col) =
                table_widget.table_state.selected_column()
            {
                // Use a reasonable default area width (will be refined in render)
                table_widget
                    .adjust_offset_for_selected_column(selected_col, 80);
            }
        }
        KeyCode::Char('0') => {
            table_widget.table_state.select_column(Some(0));
            TableNavigationHandler::clamp_data_table_columns(table_widget);
            // Reset offset when going to first column
            table_widget.column_offset = 0;
        }
        KeyCode::Char('$') => {
            if let Some(num_cols) = table_widget
                .items
                .first()
                .map(d7s_db::TableData::num_columns)
            {
                let last_col = num_cols.saturating_sub(1);
                table_widget.table_state.select_column(Some(last_col));
                TableNavigationHandler::clamp_data_table_columns(table_widget);
                // Adjust offset to ensure last column is visible
                table_widget.adjust_offset_for_selected_column(last_col, 80);
            }
        }
        KeyCode::Char('g') => {
            table_widget.table_state.select(Some(1));
            TableNavigationHandler::clamp_data_table_selection(table_widget);
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
