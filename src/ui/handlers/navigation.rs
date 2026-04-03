use crossterm::event::KeyCode;
use ratatui::widgets::TableState;

use crate::{
    db::TableData,
    ui::{
        table::{TableModel, TableViewState},
        widgets::table::TableDataState,
    },
};

/// Helper for table navigation operations
pub struct TableNavigationHandler;

impl TableNavigationHandler {
    /// Wraps the selection for a `TableState` - going past the end wraps to the beginning and vice versa
    pub const fn wrap_rows<T: TableData>(state: &mut TableState, items: &[T]) {
        if let Some(selected) = state.selected() {
            if items.is_empty() {
                state.select(None);
            } else if selected == items.len() {
                // Past the end - wrap to beginning
                state.select_first();
            } else if selected > items.len() {
                // Underflow (wrapped from 0) - wrap to end
                state.select_last();
            }
        }
    }

    /// Wraps the column selection for a `TableState` - going past the end wraps to the beginning and vice versa
    #[allow(dead_code)]
    pub fn wrap_columns<T: TableData>(
        state: &mut TableState,
        items: &[T],
        column_offset: &mut usize,
    ) {
        let num_columns =
            items.first().map_or_else(|| 0, TableData::num_columns);

        if num_columns == 0 {
            state.select_column(None);
            *column_offset = 0;
            return;
        }

        // Wrap selected column
        if let Some(selected_col) = state.selected_column()
            && selected_col >= num_columns
        {
            // Past the end or underflow - wrap to beginning
            state.select_column(Some(0));
        }

        // Clamp column offset (offset doesn't wrap, just clamps)
        if *column_offset >= num_columns {
            *column_offset = num_columns.saturating_sub(1);
        }
    }

    /// Generic table navigation handler for any `TableState`
    #[allow(clippy::wildcard_enum_match_arm)]
    pub fn navigate_table<T: TableData + Clone>(
        model: &TableModel<T>,
        view: &mut TableViewState,
        key: KeyCode,
    ) {
        match key {
            KeyCode::Char('j') | KeyCode::Down => {
                if let Some(selected) = view.state.selected()
                    && !model.items.is_empty()
                {
                    if selected >= model.items.len() - 1 {
                        view.state.select(Some(0));
                    } else {
                        view.state.select_next();
                    }
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if let Some(selected) = view.state.selected()
                    && !model.items.is_empty()
                {
                    if selected == 0 {
                        view.state.select(Some(model.items.len() - 1));
                    } else {
                        view.state.select_previous();
                    }
                }
            }
            KeyCode::Char('h' | 'b') | KeyCode::Left => {
                let num_cols =
                    model.items.first().map_or(0, TableData::num_columns);
                if num_cols == 0 {
                    return;
                }

                if let Some(selected_col) = view.state.selected_column() {
                    if selected_col == 0 {
                        view.state.select_column(Some(num_cols - 1));
                    } else {
                        view.state.select_previous_column();
                    }
                } else {
                    view.state.select_column(Some(num_cols - 1));
                }

                if let Some(selected_col) = view.state.selected_column() {
                    crate::ui::widgets::table::adjust_offset_for_selected_column(
                        &mut view.column_offset,
                        &model.longest_item_lens,
                        selected_col,
                        80,
                    );
                }
            }
            KeyCode::Char('l' | 'w') | KeyCode::Right => {
                let num_cols =
                    model.items.first().map_or(0, TableData::num_columns);
                if num_cols == 0 {
                    return;
                }

                if let Some(selected_col) = view.state.selected_column() {
                    if selected_col + 1 >= num_cols {
                        view.state.select_column(Some(0));
                    } else {
                        view.state.select_next_column();
                    }
                } else {
                    view.state.select_column(Some(0));
                }

                if let Some(selected_col) = view.state.selected_column() {
                    crate::ui::widgets::table::adjust_offset_for_selected_column(
                        &mut view.column_offset,
                        &model.longest_item_lens,
                        selected_col,
                        80,
                    );
                }
            }
            KeyCode::Char('g') => {
                view.state.select(Some(0));
                Self::wrap_rows(&mut view.state, &model.items);
                view.column_offset = 0;
            }
            KeyCode::Char('G') => {
                if !model.items.is_empty() {
                    view.state.select(Some(model.items.len() - 1));
                }
            }
            _ => {}
        }
    }

    /// Handles navigation for table data widget
    #[allow(dead_code)]
    pub fn navigate<T: TableData + Clone>(
        table_data: &mut Option<TableDataState<T>>,
        key: KeyCode,
    ) {
        if let Some(table) = table_data {
            Self::navigate_table(&table.model, &mut table.view, key);
        }
    }
}
