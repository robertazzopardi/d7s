use ratatui::style::{Color, Style};

use crate::db::connection::Environment;

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
