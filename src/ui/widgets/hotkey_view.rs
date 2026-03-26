use ratatui::{
    prelude::{Buffer, Rect, Widget},
    widgets::Paragraph,
};

use super::hotkey::Hotkey;

pub struct HotkeyView<'a> {
    pub hotkeys: &'a [Hotkey<'a>],
}

impl<'a> HotkeyView<'a> {
    #[must_use]
    pub const fn new(hotkeys: &'a [Hotkey<'a>]) -> Self {
        Self { hotkeys }
    }
}

impl Widget for HotkeyView<'_> {
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

            Paragraph::new(format!("<{}> {}", hotkey, hotkey.description))
                .render(hotkey_area, buf);

            y += 1;
        }
    }
}
