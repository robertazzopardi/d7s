use crossterm::event::KeyCode;
use ratatui::{
    prelude::{Alignment, Buffer, Constraint, Layout, Rect, Widget},
    widgets::Paragraph,
};

use super::{connection::Connection, hotkey_view::HotkeyView};
use crate::{app::APP_NAME, widgets::hotkey::Hotkey};

const CONNECTION_HOTKEYS: [Hotkey; 5] = [
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
    Hotkey {
        keycode: KeyCode::Char('t'),
        description: "Test Connection",
    },
];

pub struct TopBarView {
    pub current_connection: Connection,
}

impl Widget for TopBarView {
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
            hotkeys: &CONNECTION_HOTKEYS,
        }
        .render(cells[1], buf);
        Paragraph::new(APP_NAME.trim_start())
            .alignment(Alignment::Right)
            .render(cells[2], buf);
    }
}
