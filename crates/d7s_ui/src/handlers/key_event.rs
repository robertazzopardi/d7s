use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use d7s_db::TableData;

use super::navigation::TableNavigationHandler;
use crate::widgets::{
    search_filter::SearchFilter, sql_executor::SqlExecutorState,
    table::TableDataState,
};

/// Default terminal width used for column offset calculations
const DEFAULT_TERMINAL_WIDTH: u16 = 80;

/// Macro to generate text input key handling logic
macro_rules! handle_text_input {
    ($key:expr, $widget:expr, $on_change:block, $on_esc_enter:block, $handle_esc:expr) => {{
        match ($key.modifiers, $key.code) {
            (_, KeyCode::Esc | KeyCode::Enter) if $handle_esc => {
                $on_esc_enter;
                true
            }
            (_, KeyCode::Char(ch)) if !ch.is_control() => {
                $widget.add_char(ch);
                $on_change;
                true
            }
            (_, KeyCode::Backspace) => {
                $widget.delete_char();
                $on_change;
                true
            }
            (_, KeyCode::Left) => {
                $widget.move_cursor_left();
                true
            }
            (_, KeyCode::Right) => {
                $widget.move_cursor_right();
                true
            }
            (KeyModifiers::CONTROL, KeyCode::Char('a')) => {
                $widget.move_cursor_to_start();
                true
            }
            (KeyModifiers::CONTROL, KeyCode::Char('e')) => {
                $widget.move_cursor_to_end();
                true
            }
            (KeyModifiers::CONTROL, KeyCode::Char('u')) => {
                $widget.clear();
                $on_change;
                true
            }
            _ => false,
        }
    }};
}

/// Handles search filter key events
pub fn handle_search_filter_input(
    key: KeyEvent,
    search_filter: &mut SearchFilter,
    on_filter_change: &mut dyn FnMut(),
) -> bool {
    handle_text_input!(
        key,
        search_filter,
        {
            on_filter_change();
        },
        {
            search_filter.deactivate();
        },
        true
    )
}

/// Handles SQL executor key events
pub fn handle_sql_executor_input(
    key: KeyEvent,
    sql_executor: &mut SqlExecutorState,
) -> bool {
    handle_text_input!(key, sql_executor, {}, {}, false)
}

pub fn handle_connection_list_navigation<T: TableData + Clone>(
    key: KeyCode,
    table_state: &mut TableDataState<T>,
) {
    // Handle special column navigation keys
    match key {
        KeyCode::Char('0') => {
            table_state.view.state.select_column(Some(0));
            table_state.view.column_offset = 0;
            TableNavigationHandler::wrap_columns(table_state);
            return;
        }
        KeyCode::Char('$') => {
            if let Some(num_cols) =
                table_state.model.items.first().map(TableData::num_columns)
            {
                let last_col = num_cols.saturating_sub(1);
                table_state.view.state.select_column(Some(last_col));
                table_state.adjust_offset_for_selected_column(
                    last_col,
                    DEFAULT_TERMINAL_WIDTH,
                );
                TableNavigationHandler::wrap_columns(table_state);
            }
            return;
        }
        KeyCode::Char('g') => {
            // Note: Connection list uses index 1 instead of 0 for first row
            table_state.view.state.select(Some(1));
            TableNavigationHandler::wrap_rows(table_state);
            return;
        }
        _ => {}
    }

    // Use standard navigation for common keys (j/k/h/l/G/etc)
    TableNavigationHandler::navigate_table(table_state, key);
}
