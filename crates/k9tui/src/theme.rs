use std::{collections::HashMap, sync::OnceLock};

use ratatui::style::{Color, Modifier, Style};
use serde::Deserialize;

use crate::config_dir::config_dir;

/// Named colors that make up a skin. Grouped by role, not by widget — several
/// widgets share a role (e.g. `muted` backs borders, labels, and idle status).
#[derive(Debug, Clone, Copy)]
pub struct Palette {
    pub muted: Color,
    pub text: Color,
    pub focus: Color,
    pub on_focus: Color,
    pub selection_fg: Color,
    pub selection_bg: Color,
    pub bg_alt: Color,
    pub draft: Color,
    pub multi_select: Color,
    pub info: Color,
    pub error: Color,
    pub success: Color,
    pub warning: Color,
    pub link: Color,
}

impl Default for Palette {
    fn default() -> Self {
        Self {
            muted: Color::DarkGray,
            text: Color::White,
            focus: Color::Yellow,
            on_focus: Color::Black,
            selection_fg: Color::Black,
            selection_bg: Color::Cyan,
            bg_alt: Color::DarkGray,
            draft: Color::Green,
            multi_select: Color::Blue,
            info: Color::Cyan,
            error: Color::Red,
            success: Color::Green,
            warning: Color::Yellow,
            link: Color::Blue,
        }
    }
}

static PALETTE: OnceLock<Palette> = OnceLock::new();

/// Install the active palette. Only the first call takes effect; safe to
/// call once at startup before any rendering happens.
pub fn set_palette(palette: Palette) {
    let _ = PALETTE.set(palette);
}

fn palette() -> &'static Palette {
    PALETTE.get_or_init(Palette::default)
}

/// Parse a skin color name: a named ANSI color (case-insensitive, e.g.
/// `"darkgray"`, `"lightred"`) or a `#rrggbb` hex triplet.
#[must_use]
pub fn parse_color(s: &str) -> Option<Color> {
    if let Some(hex) = s.strip_prefix('#') {
        if hex.len() == 6 {
            let r = u8::from_str_radix(hex.get(0..2)?, 16).ok()?;
            let g = u8::from_str_radix(hex.get(2..4)?, 16).ok()?;
            let b = u8::from_str_radix(hex.get(4..6)?, 16).ok()?;
            return Some(Color::Rgb(r, g, b));
        }
        return None;
    }
    Some(match s.to_ascii_lowercase().as_str() {
        "black" => Color::Black,
        "red" => Color::Red,
        "green" => Color::Green,
        "yellow" => Color::Yellow,
        "blue" => Color::Blue,
        "magenta" => Color::Magenta,
        "cyan" => Color::Cyan,
        "gray" | "grey" => Color::Gray,
        "darkgray" | "darkgrey" => Color::DarkGray,
        "lightred" => Color::LightRed,
        "lightgreen" => Color::LightGreen,
        "lightyellow" => Color::LightYellow,
        "lightblue" => Color::LightBlue,
        "lightmagenta" => Color::LightMagenta,
        "lightcyan" => Color::LightCyan,
        "white" => Color::White,
        _ => return None,
    })
}

#[must_use]
pub fn border() -> Style {
    Style::default().fg(palette().muted)
}

#[must_use]
pub fn title() -> Style {
    Style::default()
        .fg(palette().text)
        .add_modifier(Modifier::BOLD)
}

#[must_use]
pub fn muted() -> Style {
    Style::default().fg(palette().muted)
}

// No hue here by design — color is reserved for state (env tags, errors,
// the cursor row), not decoration. Weight carries emphasis instead.
#[must_use]
pub fn accent() -> Style {
    Style::default()
        .fg(palette().text)
        .add_modifier(Modifier::BOLD)
}

#[must_use]
pub fn info_label() -> Style {
    Style::default().fg(palette().muted)
}

#[must_use]
pub fn info_value() -> Style {
    Style::default().fg(palette().text)
}

#[must_use]
pub fn hotkey_key() -> Style {
    Style::default().fg(palette().text)
}

#[must_use]
pub fn focus_field() -> Style {
    Style::default().fg(palette().focus).bg(palette().bg_alt)
}

#[must_use]
pub fn focus_cursor() -> Style {
    Style::default().bg(palette().focus).fg(palette().on_focus)
}

#[must_use]
pub fn selection_row() -> Style {
    Style::default()
        .fg(palette().selection_fg)
        .bg(palette().selection_bg)
        .add_modifier(Modifier::BOLD)
}

#[must_use]
pub fn selection_col() -> Style {
    // No UNDERLINED here — ratatui applies column_highlight_style to every
    // row in the column, so underline would run down the whole column.
    Style::default().fg(palette().selection_bg)
}

#[must_use]
pub fn selection_cell() -> Style {
    Style::default()
        .add_modifier(Modifier::BOLD)
        .fg(palette().text)
        .bg(palette().bg_alt)
}

#[must_use]
pub fn draft_row() -> Style {
    Style::default().fg(palette().draft)
}

#[must_use]
pub fn multi_select_bg() -> Color {
    palette().multi_select
}

#[must_use]
pub fn status_idle() -> Style {
    Style::default().fg(palette().muted)
}

#[must_use]
pub fn status_message() -> Style {
    Style::default().fg(palette().info)
}

#[must_use]
pub fn error() -> Style {
    Style::default().fg(palette().error)
}

#[must_use]
pub fn success() -> Style {
    Style::default().fg(palette().success)
}

#[must_use]
pub fn modal_default_border() -> Style {
    Style::default().fg(palette().info)
}

#[must_use]
pub fn modal_danger_border() -> Style {
    Style::default().fg(palette().error)
}

#[must_use]
pub fn modal_confirm_border() -> Style {
    Style::default().fg(palette().warning)
}

#[must_use]
pub fn modal_connection_border() -> Style {
    Style::default().fg(palette().link)
}

#[must_use]
pub fn header_row() -> Style {
    // Plain, unbold — no background bar, no weight.
    Style::default().fg(palette().text)
}

#[must_use]
pub fn null_cell() -> Style {
    Style::default().fg(palette().muted)
}

#[derive(Debug, Deserialize, Default)]
struct SkinFile {
    #[serde(default)]
    colors: HashMap<String, String>,
}

/// Load `~/.config/<app>/skin.yml` and apply any color overrides it
/// contains, then install the resulting palette. Any field absent from the
/// file, or the file/dir being absent entirely, keeps the built-in default
/// for that field — a missing skin file is normal, not an error.
///
/// Must be called before the first `theme::*()` call to take effect (the
/// palette installs once, at first use).
pub fn load_skin(app: &str) {
    let mut palette = Palette::default();
    if let Some(dir) = config_dir(app) {
        let path = dir.join("skin.yml");
        if let Ok(contents) = std::fs::read_to_string(path)
            && let Ok(skin) = serde_yaml::from_str::<SkinFile>(&contents)
        {
            apply_overrides(&mut palette, &skin.colors);
        }
    }
    set_palette(palette);
}

fn apply_overrides(palette: &mut Palette, colors: &HashMap<String, String>) {
    macro_rules! apply {
        ($($name:literal => $field:ident),* $(,)?) => {
            $(
                if let Some(c) = colors.get($name).and_then(|s| parse_color(s)) {
                    palette.$field = c;
                }
            )*
        };
    }
    apply! {
        "muted" => muted,
        "text" => text,
        "focus" => focus,
        "on_focus" => on_focus,
        "selection_fg" => selection_fg,
        "selection_bg" => selection_bg,
        "bg_alt" => bg_alt,
        "draft" => draft,
        "multi_select" => multi_select,
        "info" => info,
        "error" => error,
        "success" => success,
        "warning" => warning,
        "link" => link,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_named_and_hex_colors() {
        assert_eq!(parse_color("Red"), Some(Color::Red));
        assert_eq!(parse_color("darkgray"), Some(Color::DarkGray));
        assert_eq!(parse_color("#ff00aa"), Some(Color::Rgb(0xff, 0x00, 0xaa)));
        assert_eq!(parse_color("not-a-color"), None);
        assert_eq!(parse_color("#zzzzzz"), None);
    }
}
