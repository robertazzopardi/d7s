use ratatui::style::{Color, Modifier, Style};

use crate::db::connection::Environment;

pub fn border() -> Style {
    Style::default().fg(Color::DarkGray)
}

pub fn title() -> Style {
    Style::default()
        .fg(Color::White)
        .add_modifier(Modifier::BOLD)
}

pub fn muted() -> Style {
    Style::default().fg(Color::DarkGray)
}

// No hue here by design — color is reserved for state (env tags, errors,
// the cursor row), not decoration. Weight carries emphasis instead.
pub fn accent() -> Style {
    Style::default()
        .fg(Color::White)
        .add_modifier(Modifier::BOLD)
}

pub fn info_label() -> Style {
    Style::default().fg(Color::DarkGray)
}

pub fn info_value() -> Style {
    Style::default().fg(Color::White)
}

pub fn hotkey_key() -> Style {
    Style::default().fg(Color::White)
}

pub fn focus_field() -> Style {
    Style::default().fg(Color::Yellow).bg(Color::DarkGray)
}

pub fn focus_cursor() -> Style {
    Style::default().bg(Color::Yellow).fg(Color::Black)
}

pub fn selection_row() -> Style {
    Style::default()
        .fg(Color::Black)
        .bg(Color::Cyan)
        .add_modifier(Modifier::BOLD)
}

pub fn selection_col() -> Style {
    // No UNDERLINED here — ratatui applies column_highlight_style to every
    // row in the column, so underline would run down the whole column.
    Style::default().fg(Color::Cyan)
}

pub fn selection_cell() -> Style {
    Style::default()
        .add_modifier(Modifier::BOLD)
        .fg(Color::White)
        .bg(Color::DarkGray)
}

pub fn draft_row() -> Style {
    Style::default().fg(Color::Green)
}

pub const fn multi_select_bg() -> Color {
    Color::Blue
}

pub fn status_idle() -> Style {
    Style::default().fg(Color::DarkGray)
}

pub fn status_message() -> Style {
    Style::default().fg(Color::Cyan)
}

pub fn error() -> Style {
    Style::default().fg(Color::Red)
}

pub fn success() -> Style {
    Style::default().fg(Color::Green)
}

pub fn modal_default_border() -> Style {
    Style::default().fg(Color::Cyan)
}

pub fn modal_danger_border() -> Style {
    Style::default().fg(Color::Red)
}

pub fn modal_confirm_border() -> Style {
    Style::default().fg(Color::Yellow)
}

pub fn modal_connection_border() -> Style {
    Style::default().fg(Color::Blue)
}

pub fn header_row() -> Style {
    // Plain, unbold — no background bar, no weight.
    Style::default().fg(Color::White)
}

pub fn null_cell() -> Style {
    Style::default().fg(Color::DarkGray)
}

#[must_use]
pub const fn env_style(env: Environment) -> Style {
    Style::new().fg(match env {
        Environment::Dev => Color::Green,
        Environment::Staging => Color::Yellow,
        Environment::Prod => Color::Red,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_styles_differ() {
        assert_ne!(
            env_style(Environment::Dev).fg,
            env_style(Environment::Prod).fg
        );
    }
}
