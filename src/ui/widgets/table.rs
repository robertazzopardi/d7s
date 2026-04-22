use std::{collections::BTreeSet, sync::Arc};

use ratatui::{
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    text::Text,
    widgets::{Cell, HighlightSpacing, Row, StatefulWidget, Table, TableState},
};

use crate::{
    db::{DbRowId, TableData},
    ui::widgets::constraint_len_calculator,
};

/// A wrapper type for raw table data with dynamic column names
#[derive(Clone, Debug, Default)]
pub struct RawTableRow {
    pub values: Vec<String>,
    #[allow(dead_code)]
    pub column_names: Arc<Vec<String>>,
    /// Set when rows are loaded from a concrete DB table (for `UPDATE`).
    pub db_row_id: Option<DbRowId>,
    /// Pending row not yet `INSERT`ed; edited locally until commit (`s`).
    pub is_draft: bool,
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

    fn is_draft_row(&self) -> bool {
        self.is_draft
    }
}

/// Model (data) for the table view
#[derive(Clone, Debug, Default)]
pub struct TableModel<T: TableData + Clone> {
    pub items: Vec<T>,
    pub longest_item_lens: Vec<usize>,
    // For RawTableRow, we store column names here
    pub dynamic_column_names: Option<Arc<Vec<String>>>,
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

impl TableDataState<RawTableRow> {
    /// Reset the table state with new raw data
    pub fn reset(
        &mut self,
        items: Vec<Vec<String>>,
        column_names: &[String],
        row_ids: Option<Vec<Option<DbRowId>>>,
    ) {
        let column_names_arc = Arc::new(column_names.to_owned());
        let row_ids = row_ids.filter(|r| r.len() == items.len());
        let raw_rows: Vec<RawTableRow> = items
            .into_iter()
            .enumerate()
            .map(|(i, values)| RawTableRow {
                values,
                column_names: Arc::clone(&column_names_arc),
                db_row_id: row_ids
                    .as_ref()
                    .and_then(|r| r.get(i))
                    .cloned()
                    .flatten(),
                is_draft: false,
            })
            .collect();
        let longest_item_lens =
            constraint_len_calculator_for_raw_data(&raw_rows, column_names);

        self.model.items = raw_rows;
        self.model.longest_item_lens = longest_item_lens;
        self.model.dynamic_column_names = Some(column_names_arc);
        self.view.state.select(Some(0));
        self.view.column_offset = 0;
        self.multi_row_selection.clear();
    }

    /// Recompute column display widths after cell text changes.
    pub fn recompute_column_widths(&mut self) {
        let Some(names) = self.model.dynamic_column_names.as_deref() else {
            return;
        };
        self.model.longest_item_lens =
            constraint_len_calculator_for_raw_data(&self.model.items, names);
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

// pub struct TableWidget;

impl<T: TableData + std::fmt::Debug + Clone> StatefulWidget for DataTable<T> {
    type State = TableDataState<T>;

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
                let style = selected_row_style.patch(Style::new().bg(Color::Blue));
                if state
                    .model
                    .items
                    .get(i)
                    .is_some_and(TableData::is_draft_row)
                {
                    style.patch(Style::new().fg(Color::LightGreen))
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

        let rows = state.model.items.iter().enumerate().map(|(row_idx, data)| {
            let row_data = data.ref_array();
            let mut row_style = Style::new();
            if data.is_draft_row() {
                row_style = row_style.fg(Color::LightGreen);
            }
            if state.multi_row_selection.contains(&row_idx) {
                row_style = row_style.bg(Color::Blue);
            }
            visible_cols
                .iter()
                .map(|&idx| {
                    let value = row_data.get(idx).cloned().unwrap_or_default();
                    Cell::from(value)
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

// Helper function to calculate constraints for raw table data
fn constraint_len_calculator_for_raw_data(
    items: &[RawTableRow],
    column_names: &[String],
) -> Vec<usize> {
    use unicode_width::UnicodeWidthStr;

    let mut longest_lens = column_names
        .iter()
        .map(|name| UnicodeWidthStr::width(name.as_str()))
        .collect::<Vec<usize>>();

    for item in items {
        for (i, value) in item.values.iter().enumerate() {
            if i < longest_lens.len() {
                let max_width = value
                    .lines()
                    .map(UnicodeWidthStr::width)
                    .max()
                    .unwrap_or(0);

                if let Some(longest_len) = longest_lens.get_mut(i) {
                    *longest_len = (*longest_len).max(max_width);
                }

                // longest_lens[i] = longest_lens[i].max(max_width);
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
        .fg(Color::Black)
        .bg(Color::Yellow);
    let selected_col_style = Style::default().fg(Color::Cyan);
    let selected_cell_style = Style::default()
        .add_modifier(Modifier::REVERSED)
        .fg(Color::Magenta);
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
