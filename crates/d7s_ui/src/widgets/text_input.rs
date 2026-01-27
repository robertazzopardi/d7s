/// A reusable text input component with cursor management
/// Uses character-based indexing for proper cursor positioning with multi-byte characters
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TextInput {
    text: String,
    /// Character index (not byte index) for cursor position
    character_index: usize,
}

impl TextInput {
    /// Create a new empty `TextInput`
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a `TextInput` with initial text and cursor at the end
    #[must_use]
    pub fn with_text(text: String) -> Self {
        let character_index = text.chars().count();
        Self {
            text,
            character_index,
        }
    }

    /// Get the text content
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Get the cursor position (character index)
    #[must_use]
    pub const fn cursor_position(&self) -> usize {
        self.character_index
    }

    /// Returns the byte index based on the character position.
    /// Since each character in a string can contain multiple bytes, it's necessary to calculate
    /// the byte index based on the index of the character.
    #[must_use]
    pub fn byte_index(&self) -> usize {
        self.text
            .char_indices()
            .map(|(i, _)| i)
            .nth(self.character_index)
            .unwrap_or(self.text.len())
    }

    /// Set the text and move cursor to the end
    pub fn set_text(&mut self, text: String) {
        self.character_index = text.chars().count();
        self.text = text;
    }

    /// Insert a character at the cursor position
    pub fn add_char(&mut self, ch: char) {
        let byte_index = self.byte_index();
        self.text.insert(byte_index, ch);
        self.move_cursor_right();
    }

    /// Delete the character before the cursor (backspace)
    pub fn delete_char(&mut self) {
        let is_not_cursor_leftmost = self.character_index != 0;
        if is_not_cursor_leftmost {
            // Method "remove" is not used on the saved text for deleting the selected char.
            // Reason: Using remove on String works on bytes instead of the chars.
            // Using remove would require special care because of char boundaries.
            let current_index = self.character_index;
            let from_left_to_current_index = current_index - 1;
            // Getting all characters before the selected character.
            let before_char_to_delete = self.text.chars().take(from_left_to_current_index);
            // Getting all characters after selected character.
            let after_char_to_delete = self.text.chars().skip(current_index);
            // Put all characters together except the selected one.
            // By leaving the selected one out, it is forgotten and therefore deleted.
            self.text = before_char_to_delete.chain(after_char_to_delete).collect();
            self.move_cursor_left();
        }
    }

    /// Move cursor one position to the left
    pub fn move_cursor_left(&mut self) {
        let cursor_moved_left = self.character_index.saturating_sub(1);
        self.character_index = self.clamp_cursor(cursor_moved_left);
    }

    /// Move cursor one position to the right
    pub fn move_cursor_right(&mut self) {
        let cursor_moved_right = self.character_index.saturating_add(1);
        self.character_index = self.clamp_cursor(cursor_moved_right);
    }

    /// Move cursor to the start of the text
    pub const fn move_cursor_to_start(&mut self) {
        self.character_index = 0;
    }

    /// Move cursor to the end of the text
    pub fn move_cursor_to_end(&mut self) {
        self.character_index = self.text.chars().count();
    }

    /// Clamp cursor position to valid range
    fn clamp_cursor(&self, new_cursor_pos: usize) -> usize {
        new_cursor_pos.clamp(0, self.text.chars().count())
    }

    /// Clear all text and reset cursor
    pub fn clear(&mut self) {
        self.text.clear();
        self.character_index = 0;
    }

    /// Check if the text is empty
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    /// Get the length of the text (character count)
    #[must_use]
    pub fn len(&self) -> usize {
        self.text.chars().count()
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
        // Cursor starts at end (position 5)
        input.move_cursor_left(); // Move to position 4
        input.delete_char(); // Delete 'o' at position 4
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
