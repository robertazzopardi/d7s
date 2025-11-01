use crossterm::event::KeyCode;
use d7s_db::connection::Connection;
use ratatui::{
    prelude::{Alignment, Buffer, Constraint, Layout, Rect, Widget},
    widgets::Paragraph,
};

use super::hotkey_view::HotkeyView;
use super::hotkey::Hotkey;

pub const CONNECTION_HOTKEYS: [Hotkey; 4] = [
    Hotkey {
        keycode: KeyCode::Char('n'),
        description: "New Connection",
    },
    Hotkey {
        keycode: KeyCode::Char('e'),
        description: "Edit Connection",
    },
    Hotkey {
        keycode: KeyCode::Char('d'),
        description: "Delete Connection",
    },
    Hotkey {
        keycode: KeyCode::Char('o'),
        description: "Open Connection",
    },
];

pub const DATABASE_HOTKEYS: [Hotkey; 3] = [
    Hotkey {
        keycode: KeyCode::Char('s'),
        description: "SQL Executor",
    },
    Hotkey {
        keycode: KeyCode::Char('t'),
        description: "Toggle View",
    },
    Hotkey {
        keycode: KeyCode::Char('/'),
        description: "Search",
    },
];

pub struct TopBarView<'a> {
    pub current_connection: Connection,
    pub hotkeys: &'a [Hotkey<'a>],
    pub app_name: &'a str,
}

impl Widget for TopBarView<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let col_constraints = [
            Constraint::Percentage(30),
            Constraint::Percentage(40),
            Constraint::Percentage(30),
        ];
        let row_constraints = [Constraint::Fill(1)];

        let horizontal = Layout::horizontal(col_constraints).spacing(1);
        let vertical = Layout::vertical(row_constraints).spacing(1);

        let rows = vertical.split(area);
        let cells = rows
            .iter()
            .flat_map(|&row| horizontal.split(row).to_vec())
            .collect::<Vec<_>>();

        Paragraph::new(format!("{}", self.current_connection))
            .render(cells[0], buf);
        HotkeyView {
            hotkeys: self.hotkeys,
        }
        .render(cells[1], buf);
        Paragraph::new(self.app_name.trim_start())
            .alignment(Alignment::Right)
            .render(cells[2], buf);
    }
}

