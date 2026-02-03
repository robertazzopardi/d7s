pub mod buttons;
pub mod hotkey;
pub mod hotkey_view;
pub mod modal;
pub mod search_filter;
pub mod sql_executor;
pub mod status_line;
pub mod table;
pub mod text_input;
pub mod top_bar_view;

use d7s_db::TableData;
pub use status_line::StatusLine;
use unicode_width::UnicodeWidthStr;

pub fn constraint_len_calculator<T: TableData>(items: &[T]) -> Vec<usize> {
    if items.is_empty() {
        return Vec::new();
    }

    let num_columns = items[0].num_columns();

    // Initialize with column header widths
    let column_names = T::cols();
    let mut result = column_names
        .iter()
        .map(|name| UnicodeWidthStr::width(*name))
        .collect::<Vec<usize>>();

    // Ensure we have entries for all columns (in case cols() returns fewer than num_columns)
    result.resize(num_columns, 0);

    for (col_idx, max_width) in result.iter_mut().enumerate().take(num_columns)
    {
        for data in items {
            for line in data.col(col_idx).lines() {
                let width = UnicodeWidthStr::width(line);
                if width > *max_width {
                    *max_width = width;
                }
            }
        }
    }

    result
}
