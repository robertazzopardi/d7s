use ratatui::{
    prelude::{Buffer, Rect, Widget},
    style::{Color, Style},
    text::{Line, Span},
};

pub struct Buttons<'a> {
    pub buttons: Vec<&'a str>,
    pub selected: usize,
}

impl Widget for Buttons<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut button_spans = vec![];
        for (i, button) in self.buttons.iter().enumerate() {
            if i == self.selected {
                button_spans.push(Span::styled(
                    format!(" {button} "),
                    Style::default().fg(Color::White).bg(Color::Blue),
                ));
            } else {
                button_spans.push(Span::styled(
                    format!(" {button} "),
                    Style::default().fg(Color::White).bg(Color::DarkGray),
                ));
            }
            button_spans.push(Span::raw(" "));
        }
        Line::from(button_spans).centered().render(area, buf);
    }
}
