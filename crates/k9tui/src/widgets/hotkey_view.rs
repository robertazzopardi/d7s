use crate::theme;
use ratatui::{
    prelude::{Buffer, Rect, Widget},
    text::{Line, Span},
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

fn hotkey_line(hotkey: &Hotkey) -> Line<'static> {
    Line::from(vec![
        Span::styled("<", theme::muted()),
        Span::styled(hotkey.key_label(), theme::hotkey_key()),
        Span::styled("> ", theme::muted()),
        Span::styled(
            hotkey.description.display_suffix().into_owned(),
            theme::muted(),
        ),
    ])
}

fn overflow_line(hidden: usize) -> Line<'static> {
    Line::from(vec![
        Span::styled("<", theme::muted()),
        Span::styled("?", theme::hotkey_key()),
        Span::styled("> ", theme::muted()),
        Span::styled(format!("+{hidden} more"), theme::muted()),
    ])
}

impl Widget for HotkeyView<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if self.hotkeys.is_empty() || area.height == 0 || area.width == 0 {
            return;
        }

        // Column-major (k9s-style): fill down, then next column.
        let mut y = area.y;
        let mut x = area.x;
        let max_y = area.y + area.height;
        let area_right = area.x.saturating_add(area.width);
        let column_width: u16 = 30;

        let slots_per_col = area.height as usize;
        let cols = ((area.width as usize).saturating_add(29)) / 30;
        let max_slots = slots_per_col * cols.max(1);
        let show_overflow = self.hotkeys.len() > max_slots;
        let visible_count = if show_overflow {
            max_slots.saturating_sub(1)
        } else {
            self.hotkeys.len()
        };
        let hidden = self.hotkeys.len().saturating_sub(visible_count);

        for (i, hotkey) in self.hotkeys.iter().take(visible_count).enumerate() {
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
            hotkey_line(hotkey).render(Rect::new(x, y, line_width, 1), buf);
            y += 1;

            if show_overflow && i + 1 == visible_count && hidden > 0 {
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
                overflow_line(hidden)
                    .render(Rect::new(x, y, line_width, 1), buf);
            }
        }
    }
}
