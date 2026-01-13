//! Text input buffer utilities.
//!
//! Provides a text buffer with cursor management for use in overlays.
//! The actual keyboard capture happens at the parent view level.

/// A text buffer with cursor position tracking.
#[derive(Debug, Clone, Default)]
pub struct TextBuffer {
    /// The text content.
    pub text: String,
    /// Cursor position in bytes.
    pub cursor: usize,
}

impl TextBuffer {
    /// Create a new empty buffer.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a buffer with initial text.
    pub fn with_text(text: impl Into<String>) -> Self {
        let text = text.into();
        let cursor = text.len();
        Self { text, cursor }
    }

    /// Get the current text.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Check if the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    /// Set the text content.
    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
        self.cursor = self.text.len();
    }

    /// Clear the buffer.
    pub fn clear(&mut self) {
        self.text.clear();
        self.cursor = 0;
    }

    /// Insert a character at cursor position.
    pub fn insert_char(&mut self, c: char) {
        self.text.insert(self.cursor, c);
        self.cursor += c.len_utf8();
    }

    /// Insert a string at cursor position.
    pub fn insert_str(&mut self, s: &str) {
        self.text.insert_str(self.cursor, s);
        self.cursor += s.len();
    }

    /// Delete character before cursor (backspace).
    pub fn backspace(&mut self) -> bool {
        if self.cursor > 0 {
            let prev_char_boundary = self.prev_char_boundary();
            self.text.remove(prev_char_boundary);
            self.cursor = prev_char_boundary;
            true
        } else {
            false
        }
    }

    /// Delete character at cursor (delete key).
    pub fn delete(&mut self) -> bool {
        if self.cursor < self.text.len() {
            self.text.remove(self.cursor);
            true
        } else {
            false
        }
    }

    /// Move cursor left by one character.
    pub fn move_left(&mut self) {
        if self.cursor > 0 {
            self.cursor = self.prev_char_boundary();
        }
    }

    /// Move cursor right by one character.
    pub fn move_right(&mut self) {
        if self.cursor < self.text.len() {
            self.cursor = self.next_char_boundary();
        }
    }

    /// Move cursor to the start.
    pub fn move_to_start(&mut self) {
        self.cursor = 0;
    }

    /// Move cursor to the end.
    pub fn move_to_end(&mut self) {
        self.cursor = self.text.len();
    }

    /// Delete word before cursor (Ctrl+Backspace).
    pub fn delete_word_backward(&mut self) {
        if self.cursor == 0 {
            return;
        }

        // Skip trailing whitespace
        while self.cursor > 0
            && self
                .char_before_cursor()
                .map(|c| c.is_whitespace())
                .unwrap_or(false)
        {
            self.backspace();
        }

        // Delete until whitespace or start
        while self.cursor > 0
            && self
                .char_before_cursor()
                .map(|c| !c.is_whitespace())
                .unwrap_or(false)
        {
            self.backspace();
        }
    }

    /// Get the character before the cursor.
    fn char_before_cursor(&self) -> Option<char> {
        if self.cursor == 0 {
            return None;
        }
        self.text[..self.cursor].chars().last()
    }

    /// Find the previous character boundary.
    fn prev_char_boundary(&self) -> usize {
        self.text[..self.cursor]
            .char_indices()
            .last()
            .map(|(i, _)| i)
            .unwrap_or(0)
    }

    /// Find the next character boundary.
    fn next_char_boundary(&self) -> usize {
        self.text[self.cursor..]
            .char_indices()
            .nth(1)
            .map(|(i, _)| self.cursor + i)
            .unwrap_or(self.text.len())
    }
}

/// Result of processing a key input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyInputResult {
    /// The key was consumed and the text changed.
    TextChanged,
    /// The key was consumed but text didn't change.
    Consumed,
    /// The key should trigger submit (Enter).
    Submit,
    /// The key should trigger cancel (Escape).
    Cancel,
    /// The key was not handled.
    Ignored,
}

impl TextBuffer {
    /// Process a key input. Returns how the key was handled.
    ///
    /// This is designed to be called from a parent view's key handler.
    pub fn process_key(&mut self, key: &str, shift: bool, ctrl: bool, cmd: bool) -> KeyInputResult {
        // Handle special keys first
        match key {
            "backspace" => {
                if ctrl || cmd {
                    self.delete_word_backward();
                } else {
                    self.backspace();
                }
                KeyInputResult::TextChanged
            }
            "delete" => {
                self.delete();
                KeyInputResult::TextChanged
            }
            "left" => {
                self.move_left();
                KeyInputResult::Consumed
            }
            "right" => {
                self.move_right();
                KeyInputResult::Consumed
            }
            "home" => {
                self.move_to_start();
                KeyInputResult::Consumed
            }
            "end" => {
                self.move_to_end();
                KeyInputResult::Consumed
            }
            "enter" => KeyInputResult::Submit,
            "escape" => KeyInputResult::Cancel,
            "tab" => KeyInputResult::Ignored, // Let parent handle tab
            "space" => {
                self.insert_char(' ');
                KeyInputResult::TextChanged
            }
            _ => {
                // Handle printable characters
                if key.len() == 1 {
                    if let Some(c) = key.chars().next() {
                        if c.is_ascii_graphic() || c.is_ascii_alphanumeric() {
                            let c = if shift {
                                // Apply shift for uppercase
                                c.to_ascii_uppercase()
                            } else {
                                c
                            };
                            self.insert_char(c);
                            return KeyInputResult::TextChanged;
                        }
                    }
                }
                KeyInputResult::Ignored
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_buffer() {
        let buffer = TextBuffer::new();
        assert!(buffer.is_empty());
        assert_eq!(buffer.cursor, 0);
    }

    #[test]
    fn test_with_text() {
        let buffer = TextBuffer::with_text("hello");
        assert_eq!(buffer.text(), "hello");
        assert_eq!(buffer.cursor, 5); // Cursor at end
    }

    #[test]
    fn test_insert_char() {
        let mut buffer = TextBuffer::new();
        buffer.insert_char('h');
        buffer.insert_char('i');
        assert_eq!(buffer.text(), "hi");
        assert_eq!(buffer.cursor, 2);
    }

    #[test]
    fn test_insert_str() {
        let mut buffer = TextBuffer::new();
        buffer.insert_str("hello");
        assert_eq!(buffer.text(), "hello");
        assert_eq!(buffer.cursor, 5);
    }

    #[test]
    fn test_backspace() {
        let mut buffer = TextBuffer::with_text("hello");
        assert!(buffer.backspace());
        assert_eq!(buffer.text(), "hell");
        assert_eq!(buffer.cursor, 4);
    }

    #[test]
    fn test_backspace_at_start() {
        let mut buffer = TextBuffer::new();
        assert!(!buffer.backspace());
        assert_eq!(buffer.text(), "");
    }

    #[test]
    fn test_delete() {
        let mut buffer = TextBuffer::with_text("hello");
        buffer.move_to_start();
        assert!(buffer.delete());
        assert_eq!(buffer.text(), "ello");
    }

    #[test]
    fn test_delete_at_end() {
        let mut buffer = TextBuffer::with_text("hello");
        assert!(!buffer.delete());
        assert_eq!(buffer.text(), "hello");
    }

    #[test]
    fn test_cursor_movement() {
        let mut buffer = TextBuffer::with_text("hello");
        assert_eq!(buffer.cursor, 5);

        buffer.move_left();
        assert_eq!(buffer.cursor, 4);

        buffer.move_to_start();
        assert_eq!(buffer.cursor, 0);

        buffer.move_right();
        assert_eq!(buffer.cursor, 1);

        buffer.move_to_end();
        assert_eq!(buffer.cursor, 5);
    }

    #[test]
    fn test_insert_in_middle() {
        let mut buffer = TextBuffer::with_text("hllo");
        buffer.cursor = 1; // After 'h'
        buffer.insert_char('e');
        assert_eq!(buffer.text(), "hello");
    }

    #[test]
    fn test_clear() {
        let mut buffer = TextBuffer::with_text("hello");
        buffer.clear();
        assert!(buffer.is_empty());
        assert_eq!(buffer.cursor, 0);
    }

    #[test]
    fn test_unicode() {
        let mut buffer = TextBuffer::new();
        buffer.insert_str("hello ");
        buffer.insert_char('🎉');
        assert_eq!(buffer.text(), "hello 🎉");

        buffer.backspace();
        assert_eq!(buffer.text(), "hello ");
    }

    #[test]
    fn test_process_key_text() {
        let mut buffer = TextBuffer::new();

        let result = buffer.process_key("h", false, false, false);
        assert_eq!(result, KeyInputResult::TextChanged);
        assert_eq!(buffer.text(), "h");

        let result = buffer.process_key("i", false, false, false);
        assert_eq!(result, KeyInputResult::TextChanged);
        assert_eq!(buffer.text(), "hi");
    }

    #[test]
    fn test_process_key_shift() {
        let mut buffer = TextBuffer::new();

        buffer.process_key("h", true, false, false);
        assert_eq!(buffer.text(), "H");
    }

    #[test]
    fn test_process_key_special() {
        let mut buffer = TextBuffer::with_text("hello");

        let result = buffer.process_key("backspace", false, false, false);
        assert_eq!(result, KeyInputResult::TextChanged);
        assert_eq!(buffer.text(), "hell");

        let result = buffer.process_key("enter", false, false, false);
        assert_eq!(result, KeyInputResult::Submit);

        let result = buffer.process_key("escape", false, false, false);
        assert_eq!(result, KeyInputResult::Cancel);
    }

    #[test]
    fn test_process_key_cursor() {
        let mut buffer = TextBuffer::with_text("hello");

        let result = buffer.process_key("left", false, false, false);
        assert_eq!(result, KeyInputResult::Consumed);
        assert_eq!(buffer.cursor, 4);

        let result = buffer.process_key("home", false, false, false);
        assert_eq!(result, KeyInputResult::Consumed);
        assert_eq!(buffer.cursor, 0);
    }

    #[test]
    fn test_delete_word_backward() {
        let mut buffer = TextBuffer::with_text("hello world");
        buffer.delete_word_backward();
        assert_eq!(buffer.text(), "hello ");

        buffer.delete_word_backward();
        assert_eq!(buffer.text(), "");
    }
}
