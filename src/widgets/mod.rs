pub mod buttons;
pub mod hotkey;
pub mod hotkey_view;
pub mod modal;
pub mod table;
pub mod top_bar_view;

use d7s_db::TableData;
use unicode_width::UnicodeWidthStr;

pub(crate) fn constraint_len_calculator<T: TableData>(items: &[T]) -> Vec<u16> {
    if items.is_empty() {
        return Vec::new();
    }

    let num_columns = items[0].num_columns();

    let mut result = Vec::with_capacity(num_columns);
    for col_idx in 0..num_columns {
        let mut max_width = 0;
        for data in items.iter() {
            for line in data.col(col_idx).lines() {
                let width = UnicodeWidthStr::width(line);
                if width > max_width {
                    max_width = width;
                }
            }
        }
        result.push(max_width as u16);
    }
    result
}
