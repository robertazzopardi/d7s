use k9tui::theme;
use ratatui::{
    prelude::{Buffer, Constraint, Layout, Rect, Widget},
    text::{Line, Span},
    widgets::Paragraph,
};

use super::{hotkey::Hotkey, hotkey_view::HotkeyView};
use crate::db::connection::Connection;

pub const CONNECTION_HOTKEYS: [Hotkey; 5] = [
    Hotkey::new('n', "New Connection"),
    Hotkey::new('e', "Edit Connection"),
    Hotkey::new('d', "Delete Connection"),
    Hotkey::new('o', "Open Connection"),
    Hotkey::new('/', "Search"),
];

pub const DATABASE_HOTKEYS: [Hotkey; 5] = [
    Hotkey::new('e', "SQL Editor"),
    Hotkey::new('t', "Table structure"),
    Hotkey::new('E', "Run SQL"),
    Hotkey::new('/', "Search"),
    Hotkey::new('y', "Copy value"),
];

/// Shown in addition to [`DATABASE_HOTKEYS`] while viewing table row data.
pub const TABLE_DATA_VIEW_HOTKEYS: [Hotkey; 5] = [
    Hotkey::new('r', "Refresh"),
    Hotkey::new('a', "New row"),
    Hotkey::new('c', "Duplicate row"),
    Hotkey::new('s', "Commit row"),
    Hotkey::new('d', "Delete row"),
];

/// Flex weights for the four middle segments (connection / MRU / primary / global hotkeys).
const MAIN_COLUMN_FILLS: [Constraint; 4] = [
    Constraint::Fill(24),
    Constraint::Fill(20),
    Constraint::Fill(36),
    Constraint::Fill(12),
];
// Second row is a blank spacer before the box below — no rule drawn into it.
const ROW_CONSTRAINTS: [Constraint; 2] =
    [Constraint::Fill(1), Constraint::Length(1)];
const MIN_APP_LABEL_WIDTH: u16 = 8;
const APP_LABEL_RIGHT_MARGIN: u16 = 1;

pub struct TopBarView<'a> {
    pub current_connection: &'a Connection,
    pub recent_hotkeys: &'a [Hotkey],
    pub hotkeys: &'a [Hotkey],
    pub global_hotkeys: &'a [Hotkey],
    pub app_name: &'a str,
    pub build_info: Option<String>,
}

impl Widget for TopBarView<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let row = Layout::vertical(ROW_CONSTRAINTS)
            .spacing(0)
            .split(area)
            .first()
            .copied()
            .unwrap_or(area);

        let app_name_lines = self.app_name.trim().lines();
        let app_name_width =
            app_name_lines.clone().map(str::len).max().unwrap_or(0);
        let app_label_width = u16::try_from(app_name_width.max(1))
            .unwrap_or(u16::MAX)
            .max(MIN_APP_LABEL_WIDTH);

        let [main_area, app_logo_cell] = Layout::horizontal([
            Constraint::Fill(1),
            Constraint::Length(
                app_label_width.saturating_add(APP_LABEL_RIGHT_MARGIN),
            ),
        ])
        .spacing(1)
        .areas(row);

        let [app_info_cell, recent_cell, hotkey_cell, global_cell] =
            Layout::horizontal(MAIN_COLUMN_FILLS)
                .spacing(1)
                .areas(main_area);

        if let Some(build_info) = &self.build_info {
            render_info_stack(build_info, app_info_cell, buf);
        } else {
            render_info_stack(
                &self.current_connection.summary_stack(),
                app_info_cell,
                buf,
            );
        }

        HotkeyView::new(self.recent_hotkeys).render(recent_cell, buf);
        HotkeyView::new(self.hotkeys).render(hotkey_cell, buf);
        HotkeyView::new(self.global_hotkeys).render(global_cell, buf);

        let label_align_width =
            (app_logo_cell.width.saturating_sub(APP_LABEL_RIGHT_MARGIN))
                as usize;
        let padding = label_align_width.saturating_sub(app_name_width);
        let padded = app_name_lines
            .map(|line| {
                format!("{:>width$}", line, width = line.len() + padding)
            })
            .collect::<Vec<_>>()
            .join("\n");
        Paragraph::new(padded)
            .style(theme::border())
            .render(app_logo_cell, buf);
    }
}

fn render_info_stack(text: &str, area: Rect, buf: &mut Buffer) {
    let max_val = area.width.saturating_sub(12) as usize;
    let lines: Vec<Line> = text
        .lines()
        .map(|line| {
            if let Some((label, value)) = line.split_once(':') {
                let val =
                    crate::db::connection::shorten_home_path(value.trim());
                let val = if max_val > 1 && val.chars().count() > max_val {
                    let keep = max_val.saturating_sub(1);
                    format!(
                        "…{}",
                        val.chars()
                            .skip(val.chars().count().saturating_sub(keep))
                            .collect::<String>()
                    )
                } else {
                    val
                };
                Line::from(vec![
                    Span::styled(
                        format!("{}:", label.trim()),
                        theme::info_label(),
                    ),
                    Span::raw(" "),
                    Span::styled(val, theme::info_value()),
                ])
            } else {
                Line::from(Span::styled(line.to_string(), theme::muted()))
            }
        })
        .collect();
    Paragraph::new(lines).render(area, buf);
}
