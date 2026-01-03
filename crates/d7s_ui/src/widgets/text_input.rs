/// A reusable text input component with cursor management
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TextInput {
    text: String,
    cursor_position: usize,
}

impl TextInput {
    /// Create a new empty `TextInput`
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a `TextInput` with initial text and cursor at the end
    #[must_use]
    pub const fn with_text(text: String) -> Self {
        let cursor_position = text.len();
        Self {
            text,
            cursor_position,
        }
    }

    /// Get the text content
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Get the cursor position
    #[must_use]
    pub const fn cursor_position(&self) -> usize {
        self.cursor_position
    }

    /// Set the text and move cursor to the end
    pub fn set_text(&mut self, text: String) {
        self.cursor_position = text.len();
        self.text = text;
    }

    /// Insert a character at the cursor position
    pub fn add_char(&mut self, ch: char) {
        self.text.insert(self.cursor_position, ch);
        self.cursor_position += 1;
    }

    /// Delete the character before the cursor (backspace)
    pub fn delete_char(&mut self) {
        if self.cursor_position > 0 {
            self.text.remove(self.cursor_position - 1);
            self.cursor_position -= 1;
        }
    }

    /// Move cursor one position to the left
    pub const fn move_cursor_left(&mut self) {
        if self.cursor_position > 0 {
            self.cursor_position -= 1;
        }
    }

    /// Move cursor one position to the right
    pub const fn move_cursor_right(&mut self) {
        if self.cursor_position < self.text.len() {
            self.cursor_position += 1;
        }
    }

    /// Move cursor to the start of the text
    pub const fn move_cursor_to_start(&mut self) {
        self.cursor_position = 0;
    }

    /// Move cursor to the end of the text
    pub const fn move_cursor_to_end(&mut self) {
        self.cursor_position = self.text.len();
    }

    /// Clear all text and reset cursor
    pub fn clear(&mut self) {
        self.text.clear();
        self.cursor_position = 0;
    }

    /// Check if the text is empty
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    /// Get the length of the text
    #[must_use]
    pub const fn len(&self) -> usize {
        self.text.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_char() {
        let mut input = TextInput::new();
        input.add_char('a');
        assert_eq!(input.text(), "a");
        assert_eq!(input.cursor_position(), 1);
    }

    #[test]
    fn test_delete_char() {
        let mut input = TextInput::with_text("hello".to_string());
        input.delete_char();
        assert_eq!(input.text(), "hell");
        assert_eq!(input.cursor_position(), 4);
    }

    #[test]
    fn test_cursor_movement() {
        let mut input = TextInput::with_text("hello".to_string());
        input.move_cursor_to_start();
        assert_eq!(input.cursor_position(), 0);
        input.move_cursor_right();
        assert_eq!(input.cursor_position(), 1);
        input.move_cursor_left();
        assert_eq!(input.cursor_position(), 0);
    }
}
