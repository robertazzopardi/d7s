use crossterm::event::KeyCode;
use ratatui::{
    prelude::{Buffer, Constraint, Layout, Rect, Widget},
    widgets::Paragraph,
};

use super::{hotkey::Hotkey, hotkey_view::HotkeyView};
use crate::db::connection::Connection;

pub const CONNECTION_HOTKEYS: [Hotkey; 5] = [
    Hotkey::new('n', "New Connection"),
    Hotkey::new('e', "Edit Connection"),
    Hotkey::new('d', "Delete Connection"),
    Hotkey::new('o', "Open Connection"),
    Hotkey::new('y', "Copy value"),
];

pub const DATABASE_HOTKEYS: [Hotkey; 5] = [
    Hotkey::new('e', "SQL Editor"),
    Hotkey::new('t', "Table structure"),
    Hotkey::new('E', "Run SQL"),
    Hotkey::new('/', "Search"),
    Hotkey::new('y', "Copy value"),
];

/// Shown in addition to [`DATABASE_HOTKEYS`] while viewing table row data.
pub const TABLE_DATA_VIEW_HOTKEYS: [Hotkey; 6] = [
    Hotkey::new('r', "Refresh"),
    Hotkey::new('a', "New row"),
    Hotkey::new('c', "Copy row"),
    Hotkey::new('s', "Commit row"),
    Hotkey::new('d', "Delete row"),
    Hotkey {
        keycode: KeyCode::Char(' '),
        description: super::hotkey::HotkeyDescription::Static("Multi"),
    },
];

/// Flex weights for the three middle segments (connection / MRU / primary hotkeys), matching the
/// former 26% / 22% / 38% split of the space left of the app label column.
const MAIN_COLUMN_FILLS: [Constraint; 3] = [
    Constraint::Fill(26),
    Constraint::Fill(22),
    Constraint::Fill(38),
];
const ROW_CONSTRAINTS: [Constraint; 1] = [Constraint::Fill(1)];
const MIN_APP_LABEL_WIDTH: u16 = 8;
/// Empty column between the app label and the terminal edge (or parent rect).
const APP_LABEL_RIGHT_MARGIN: u16 = 1;

pub struct TopBarView<'a> {
    pub current_connection: &'a Connection,
    /// Left column of the hotkey bar: recent tables (`1`–`5`); empty when not connected.
    pub recent_hotkeys: &'a [Hotkey],
    pub hotkeys: &'a [Hotkey],
    pub app_name: &'a str,
    pub build_info: Option<String>,
}

impl Widget for TopBarView<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let vertical = Layout::vertical(ROW_CONSTRAINTS).spacing(1);
        let rows = vertical.split(area);
        let row = rows.first().copied().unwrap_or(area);

        let app_name_lines = self.app_name.trim().lines();
        let app_name_width =
            app_name_lines.clone().map(str::len).max().unwrap_or(0);
        let app_label_width = u16::try_from(app_name_width.max(1))
            .unwrap_or(u16::MAX)
            .max(MIN_APP_LABEL_WIDTH);

        // Reserve the right strip for the app label so it stays on-screen; share the remainder
        // between connection info and hotkey columns.
        let [main_area, app_logo_cell] = Layout::horizontal([
            Constraint::Fill(1),
            Constraint::Length(
                app_label_width.saturating_add(APP_LABEL_RIGHT_MARGIN),
            ),
        ])
        .spacing(1)
        .areas(row);

        let [app_info_cell, recent_cell, hotkey_cell] =
            Layout::horizontal(MAIN_COLUMN_FILLS)
                .spacing(1)
                .areas(main_area);

        // Display build info if provided, otherwise show connection details
        let left_content = if let Some(build_info) = self.build_info {
            build_info
        } else {
            self.current_connection.to_string()
        };

        Paragraph::new(left_content).render(app_info_cell, buf);
        HotkeyView::new(self.recent_hotkeys).render(recent_cell, buf);
        HotkeyView::new(self.hotkeys).render(hotkey_cell, buf);

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
        Paragraph::new(padded).render(app_logo_cell, buf);
    }
}
