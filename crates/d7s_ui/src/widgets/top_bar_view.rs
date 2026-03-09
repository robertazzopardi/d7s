use d7s_db::connection::Connection;
use ratatui::{
    prelude::{Buffer, Constraint, Layout, Rect, Widget},
    widgets::Paragraph,
};

use super::{hotkey::Hotkey, hotkey_view::HotkeyView};

pub const CONNECTION_HOTKEYS: [Hotkey; 5] = [
    Hotkey::new('n', "New Connection"),
    Hotkey::new('e', "Edit Connection"),
    Hotkey::new('d', "Delete Connection"),
    Hotkey::new('o', "Open Connection"),
    Hotkey::new('y', "Copy value"),
];

pub const DATABASE_HOTKEYS: [Hotkey; 3] = [
    Hotkey::new('s', "SQL Executor"),
    Hotkey::new('t', "Toggle View"),
    Hotkey::new('/', "Search"),
];

const COLUMN_CONSTRAINTS: [Constraint; 3] = [
    Constraint::Percentage(30),
    Constraint::Percentage(40),
    Constraint::Percentage(30),
];
const ROW_CONSTRAINTS: [Constraint; 1] = [Constraint::Fill(1)];

pub struct TopBarView<'a> {
    pub current_connection: Connection,
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
            format!("{}", self.current_connection)
        };
        Paragraph::new(left_content).render(cells[0], buf);
        HotkeyView {
            hotkeys: self.hotkeys,
        }
        .render(cells[1], buf);

        let app_name_width = self
            .app_name
            .trim()
            .lines()
            .map(str::len)
            .max()
            .unwrap_or(0);
        let padding = cells[2].width as usize - app_name_width;
        let padded = self
            .app_name
            .lines()
            .map(|line| {
                format!("{:>width$}", line, width = line.len() + padding)
            })
            .collect::<Vec<_>>()
            .join("\n");
        Paragraph::new(padded).render(cells[2], buf);
    }
}
