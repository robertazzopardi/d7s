use ratatui::{
    prelude::{Buffer, Constraint, Direction, Layout, Rect, Widget},
    widgets::Paragraph,
};

use super::hotkey::Hotkey;

pub struct HotkeyView<'a> {
    pub hotkeys: &'a [Hotkey<'a>],
}

impl<'a> Widget for HotkeyView<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut y = area.y;
        let mut x = area.x;
        let max_y = area.y + area.height;
        let column_width = 30; // Width for each column

        for hotkey in self.hotkeys {
            // Check if we need to start a new column
            if y >= max_y {
                x += column_width;
                y = area.y;
            }

            // Create a rectangle for this hotkey row
            let hotkey_area = Rect::new(x, y, column_width, 1);

            // Split the area horizontally for key and description
            let row = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Length(
                        (hotkey.keycode.to_string().len()
                            + hotkey.description.len())
                            as u16
                            + 3,
                    ), // Space for the key
                    Constraint::Fill(1), // Rest for description
                ])
                .split(hotkey_area);

            Paragraph::new(format!("<{}> {}", hotkey, hotkey.description))
                .render(row[0], buf);
            // Paragraph::new(format!("{}", hotkey.description))
            //     .render(row[1], buf);

            y += 1;
        }
    }
}
