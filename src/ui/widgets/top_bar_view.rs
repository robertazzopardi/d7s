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

pub const DATABASE_HOTKEYS: [Hotkey; 4] = [
    Hotkey::new('s', "SQL Executor"),
    Hotkey::new('t', "Toggle View"),
    Hotkey::new('/', "Search"),
    Hotkey::new('y', "Copy value"),
];

const COLUMN_CONSTRAINTS: [Constraint; 3] = [
    Constraint::Percentage(30),
    Constraint::Percentage(40),
    Constraint::Percentage(30),
];
const ROW_CONSTRAINTS: [Constraint; 1] = [Constraint::Fill(1)];

pub struct TopBarView<'a> {
    pub current_connection: &'a Connection,
    pub hotkeys: &'a [Hotkey<'a>],
    pub app_name: &'a str,
    pub build_info: Option<String>,
}

impl Widget for TopBarView<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let horizontal = Layout::horizontal(COLUMN_CONSTRAINTS).spacing(1);
        let vertical = Layout::vertical(ROW_CONSTRAINTS).spacing(1);

        let rows = vertical.split(area);
        let cells = rows
            .iter()
            .flat_map(|&row| horizontal.split(row).to_vec())
            .collect::<Vec<_>>();

        // Display build info if provided, otherwise show connection details
        let left_content = if let Some(build_info) = self.build_info {
            build_info
        } else {
            self.current_connection.to_string()
        };

        let app_info_cell = *cells.first().unwrap_or(&Rect::ZERO);
        let hotkey_cell = *cells.get(1).unwrap_or(&Rect::ZERO);
        let app_logo_cell = *cells.get(2).unwrap_or(&Rect::ZERO);

        Paragraph::new(left_content).render(app_info_cell, buf);
        HotkeyView::new(self.hotkeys).render(hotkey_cell, buf);

        let app_name_lines = self.app_name.trim().lines();
        let app_name_width =
            app_name_lines.clone().map(str::len).max().unwrap_or(0);
        let padding =
            (app_logo_cell.width as usize).saturating_sub(app_name_width);
        let padded = app_name_lines
            .map(|line| {
                format!("{:>width$}", line, width = line.len() + padding)
            })
            .collect::<Vec<_>>()
            .join("\n");
        Paragraph::new(padded).render(app_logo_cell, buf);
    }
}
