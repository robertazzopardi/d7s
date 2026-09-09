use std::collections::BTreeSet;

use ratatui::{
    layout::{Constraint, Rect},
    style::{Modifier, Style},
    text::Text,
    widgets::{Cell, HighlightSpacing, Row, StatefulWidget, Table, TableState},
};

use crate::theme;

pub trait TableData {
    #[allow(dead_code)]
    fn title() -> &'static str;
    fn ref_array(&self) -> Vec<String>;
    fn num_columns(&self) -> usize;
    fn cols() -> Vec<&'static str>;

    fn col(&self, column: usize) -> String {
        self.ref_array().get(column).cloned().unwrap_or_default()
    }

    /// UI draft rows (e.g. pending `INSERT`) use this for styling.
    fn is_draft_row(&self) -> bool {
        false
    }

    /// Optional per-cell styling (e.g. environment badge, help keys).
    fn cell_style(&self, _column: usize) -> Option<Style> {
        None
    }

    /// Section header row in help-like tables.
    fn is_section_header(&self) -> bool {
        false
    }
}

/// Model (data) for the table view
#[derive(Clone, Debug, Default)]
pub struct TableModel<T: TableData + Clone> {
    pub items: Vec<T>,
    pub longest_item_lens: Vec<usize>,
    // For RawTableRow, we store column names here
    pub dynamic_column_names: Option<std::sync::Arc<Vec<String>>>,
}

/// View state for the table (UI state like selection, scrolling)
#[derive(Clone, Debug, Default)]
pub struct TableViewState {
    pub state: TableState,
    pub column_offset: usize,
}

/// Combined state that holds both model and view state
#[derive(Clone, Debug, Default)]
pub struct TableDataState<T: TableData + Clone> {
    pub model: TableModel<T>,
    pub view: TableViewState,
    /// Row indices toggled with Space (batch operations, e.g. delete). Table data / `RawTableRow`.
    pub multi_row_selection: BTreeSet<usize>,
}

/// Pure stateless table widget - all state is managed externally
#[derive(Clone, Debug)]
pub struct DataTable<T: TableData + Clone>(std::marker::PhantomData<T>);

impl<T: TableData + Clone> Default for DataTable<T> {
    fn default() -> Self {
        Self(std::marker::PhantomData)
    }
}

impl<T: TableData + Clone> TableDataState<T> {
    /// Create a new table state from items
    #[must_use]
    pub fn new(items: Vec<T>) -> Self {
        let longest_item_lens = super::constraint_len_calculator(&items);
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
            multi_row_selection: BTreeSet::new(),
        }
    }

    /// Filter items based on query
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
}

const fn col_width(len: usize) -> usize {
    len + 1
}

/// First column index to show so that:
/// - if the full table fits, `0` (all columns visible);
/// - otherwise the window contains `selected` and fits as many columns as possible in `area_width`;
/// - on ties, prefers `start` closest to `scroll_hint` (stable scrolling).
fn horizontal_window_start(
    longest_item_lens: &[usize],
    area_width: usize,
    selected: usize,
    scroll_hint: usize,
) -> usize {
    let n = longest_item_lens.len();
    if n == 0 {
        return 0;
    }

    let total: usize = longest_item_lens.iter().map(|&l| col_width(l)).sum();
    if total <= area_width {
        return 0;
    }

    let Some(&sel_len) = longest_item_lens.get(selected) else {
        return 0;
    };
    if col_width(sel_len) > area_width {
        return selected;
    }

    let mut best_start = selected;
    let mut best_count = 0usize;
    let mut best_dist = usize::MAX;

    for start in 0..=selected {
        let mut w = 0usize;
        let mut last = start.saturating_sub(1);
        for (i, &len) in longest_item_lens.iter().enumerate().skip(start) {
            let cw = col_width(len);
            if w + cw > area_width {
                break;
            }
            w += cw;
            last = i;
        }

        if last < selected {
            continue;
        }

        let count = last - start + 1;
        let dist = start.abs_diff(scroll_hint);

        if count > best_count {
            best_count = count;
            best_start = start;
            best_dist = dist;
        } else if count == best_count
            && (dist < best_dist || (dist == best_dist && start < best_start))
        {
            best_start = start;
            best_dist = dist;
        }
    }

    best_start.min(n.saturating_sub(1))
}

/// Visible column indices and optional relative selection index for ratatui's subset table.
fn visible_columns_packed(
    longest_item_lens: &[usize],
    start: usize,
    area_width: usize,
) -> Vec<usize> {
    let mut vis_cols = Vec::new();
    let mut cumulative_width = 0usize;

    for (idx, &len) in longest_item_lens.iter().enumerate().skip(start) {
        let cw = col_width(len);
        if cumulative_width + cw > area_width {
            break;
        }
        cumulative_width += cw;
        vis_cols.push(idx);
    }

    if vis_cols.is_empty() && !longest_item_lens.is_empty() {
        vis_cols.push(start.min(longest_item_lens.len() - 1));
    }

    vis_cols
}

/// Helper function to calculate visible columns for `DataTable`
fn calculate_visible_columns_for_table(
    longest_item_lens: &[usize],
    column_offset: usize,
    selected_col_opt: Option<usize>,
    area_width: u16,
) -> (Vec<usize>, Option<usize>, usize) {
    let area_width = area_width as usize;
    let n = longest_item_lens.len();

    let start = selected_col_opt.map_or_else(
        || column_offset.min(n.saturating_sub(1)),
        |selected_col| {
            horizontal_window_start(
                longest_item_lens,
                area_width,
                selected_col,
                column_offset,
            )
        },
    );

    let vis_cols = visible_columns_packed(longest_item_lens, start, area_width);
    let rel = selected_col_opt.map(|selected_col| {
        vis_cols
            .iter()
            .position(|&idx| idx == selected_col)
            .unwrap_or(0)
    });

    (vis_cols, rel, start)
}

impl<T: TableData + std::fmt::Debug + Clone> StatefulWidget for DataTable<T> {
    type State = TableDataState<T>;

    #[allow(clippy::too_many_lines)]
    fn render(
        self,
        area: Rect,
        buf: &mut ratatui::buffer::Buffer,
        state: &mut Self::State,
    ) {
        if state.model.longest_item_lens.is_empty() {
            return;
        }

        let selected_col_opt = state.view.state.selected_column();
        let (visible_cols, relative_selected_col, scroll_start) =
            calculate_visible_columns_for_table(
                &state.model.longest_item_lens,
                state.view.column_offset,
                selected_col_opt,
                area.width,
            );
        state.view.column_offset = scroll_start;

        let original_col = state.view.state.selected_column();
        state.view.state.select_column(relative_selected_col);

        let (
            selected_row_style,
            selected_col_style,
            selected_cell_style,
            highlight_symbol,
            highlight_spacing,
        ) = create_table_styles();

        // `row_highlight_style` overrides each cell's row style, so a focused row that is also
        // multi-selected would lose the blue background. Patch the default row highlight with blue
        // so the cursor row still reads as "in the multi set" while keeping the same emphasis
        // (reversed, bold) as the normal selection row.
        let row_highlight_style = match state.view.state.selected() {
            Some(i) if state.multi_row_selection.contains(&i) => {
                let style = selected_row_style
                    .patch(Style::new().bg(theme::multi_select_bg()));
                if state
                    .model
                    .items
                    .get(i)
                    .is_some_and(TableData::is_draft_row)
                {
                    style.patch(theme::draft_row())
                } else {
                    style
                }
            }
            _ => selected_row_style,
        };

        // Use dynamic column names if available (for RawTableRow), otherwise use static cols()
        let header = state.model.dynamic_column_names.as_ref().map_or_else(
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
                    .style(theme::header_row())
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
                    .style(theme::header_row())
            },
        );

        let rows =
            state.model.items.iter().enumerate().map(|(row_idx, data)| {
                let row_data = data.ref_array();
                let mut row_style = Style::new();
                if data.is_section_header() {
                    row_style = theme::accent().add_modifier(Modifier::BOLD);
                } else if data.is_draft_row() {
                    row_style = theme::draft_row();
                }
                if state.multi_row_selection.contains(&row_idx) {
                    row_style = row_style.bg(theme::multi_select_bg());
                }
                visible_cols
                    .iter()
                    .enumerate()
                    .map(|(vis_idx, &idx)| {
                        let value =
                            row_data.get(idx).cloned().unwrap_or_default();
                        let (mut text, cell_style) =
                            format_display_cell(data, idx, value);
                        if data.is_draft_row() && vis_idx == 0 {
                            text = format!("~{text}");
                        }
                        let mut cell = Cell::from(text);
                        if let Some(style) = cell_style {
                            cell = cell.style(style);
                        }
                        cell
                    })
                    .collect::<Row>()
                    .style(row_style)
                    .height(1)
            });

        let constraints = visible_cols
            .iter()
            .map(|&idx| {
                let width =
                    state.model.longest_item_lens.get(idx).unwrap_or(&0) + 1;
                Constraint::Length(u16::try_from(width).unwrap_or(u16::MAX))
            })
            .collect::<Vec<_>>();

        let t = Table::new(rows, constraints)
            .header(header)
            .row_highlight_style(row_highlight_style)
            .column_highlight_style(selected_col_style)
            .cell_highlight_style(selected_cell_style)
            .highlight_symbol(highlight_symbol)
            .highlight_spacing(highlight_spacing);

        StatefulWidget::render(t, area, buf, &mut state.view.state);
        state.view.state.select_column(original_col);
    }
}

// Helper function to create table styles
fn create_table_styles()
-> (Style, Style, Style, Text<'static>, HighlightSpacing) {
    let selected_row_style = theme::selection_row();
    let selected_col_style = theme::selection_col();
    let selected_cell_style = theme::selection_cell();
    // k9s highlights the cursor row as a plain full-width bar — no marker glyph.
    let highlight_symbol = Text::from("");
    (
        selected_row_style,
        selected_col_style,
        selected_cell_style,
        highlight_symbol,
        HighlightSpacing::Never,
    )
}

fn format_display_cell<T: TableData>(
    data: &T,
    column: usize,
    value: String,
) -> (String, Option<Style>) {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        // Blank optional fields (e.g. Auth) — don't paint a null placeholder.
        return (String::new(), None);
    }
    if trimmed.eq_ignore_ascii_case("null") {
        return ("·".to_string(), Some(theme::null_cell()));
    }
    (value, data.cell_style(column))
}
