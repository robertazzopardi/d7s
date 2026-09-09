use crate::theme;
use crossterm::event::KeyEvent;
use ratatui::style::{Color, Style};
use ratatui_textarea::TextArea;

#[derive(Debug, Clone)]
pub struct ModalField {
    pub label: &'static str,
    pub input: TextArea<'static>,
    pub is_focused: bool,
    /// When set, this field is a dropdown; value must be one of these options.
    pub options: Option<Vec<&'static str>>,
}

impl ModalField {
    fn make_input(text: &str) -> TextArea<'static> {
        let mut input = TextArea::new(vec![text.to_string()]);
        // Disable cursor line highlight (not needed for single-line form fields)
        input.set_cursor_line_style(Style::default());
        // Hide cursor until focused
        input.set_cursor_style(Style::default());
        // No undo/redo needed for form fields
        input.set_max_histories(0);
        input
    }

    #[must_use]
    pub fn new(label: &'static str) -> Self {
        Self {
            label,
            input: Self::make_input(""),
            is_focused: false,
            options: None,
        }
    }

    /// Get the current text value of this field.
    #[must_use]
    pub fn value(&self) -> &str {
        self.input.lines().first().map_or("", |s| s.as_str())
    }

    /// Set the text value of this field, replacing all content.
    pub fn set_value(&mut self, text: impl Into<String>) {
        let text = text.into();
        self.input = Self::make_input(&text);
        // Move cursor to end of the pre-filled text
        self.input.move_cursor(ratatui_textarea::CursorMove::End);
        // Restore cursor style based on current focus state
        if self.is_focused {
            self.input.set_cursor_style(theme::focus_cursor());
        }
    }

    pub fn set_focus(&mut self, focused: bool) {
        self.is_focused = focused;
        if focused {
            self.input.set_style(theme::focus_field());
            self.input.set_cursor_style(theme::focus_cursor());
        } else {
            self.input.set_style(Style::default().fg(Color::White));
            self.input.set_cursor_style(Style::default());
        }
    }

    pub fn input_key(&mut self, key: KeyEvent) {
        if self.options.is_none() {
            self.input.input(key);
        }
    }

    /// Enable character masking (for password fields).
    pub fn set_masked(&mut self) {
        self.input.set_mask_char('•');
    }

    /// Set dropdown options. If value is empty, sets value to first option.
    #[allow(clippy::indexing_slicing)]
    pub fn set_options(&mut self, options: Vec<&'static str>) {
        if !options.is_empty() {
            let first = options[0].to_string();
            self.options = Some(options);
            if self.value().is_empty() {
                self.set_value(first);
            }
        }
    }

    /// Ensure value is one of the options. Sets to first option if invalid.
    #[allow(clippy::indexing_slicing)]
    pub fn clamp_to_options(&mut self) {
        if let Some(ref opts) = self.options
            && !opts.is_empty()
            && !opts.iter().any(|o| *o == self.value())
        {
            self.set_value(opts[0]);
        }
    }

    #[must_use]
    pub const fn is_dropdown(&self) -> bool {
        self.options.is_some()
    }
}
