use ratatui::{
    prelude::{Buffer, Rect, Widget},
    widgets::Paragraph,
};

use super::hotkey::Hotkey;

pub struct HotkeyView<'a> {
    pub hotkeys: &'a [Hotkey],
}

impl<'a> HotkeyView<'a> {
    #[must_use]
    pub const fn new(hotkeys: &'a [Hotkey]) -> Self {
        Self { hotkeys }
    }
}

impl Widget for HotkeyView<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut y = area.y;
        let mut x = area.x;
        let max_y = area.y + area.height;
        let area_right = area.x.saturating_add(area.width);
        let column_width: u16 = 30;

        for hotkey in self.hotkeys {
            // Check if we need to start a new column
            if y >= max_y {
                let next_x = x.saturating_add(column_width);
                if next_x >= area_right {
                    break;
                }
                x = next_x;
                y = area.y;
            }

            let avail = area_right.saturating_sub(x);
            if avail == 0 {
                break;
            }
            let line_width = column_width.min(avail);
            let hotkey_area = Rect::new(x, y, line_width, 1);

            let line =
                format!("<{}> {}", hotkey, hotkey.description.display_suffix());
            Paragraph::new(line).render(hotkey_area, buf);

            y += 1;
        }
    }
}
