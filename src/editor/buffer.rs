use ropey::Rope;
use std::collections::VecDeque;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

impl Position {
    pub fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cursor {
    pub position: Position,
}

impl Cursor {
    pub fn new(position: Position) -> Self {
        Self { position }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selection {
    pub start: Position,
    pub end: Position,
}

impl Selection {
    pub fn new(start: Position, end: Position) -> Self {
        Self { start, end }
    }

    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    pub fn normalized(&self) -> Self {
        if (self.start.line, self.start.column) <= (self.end.line, self.end.column) {
            *self
        } else {
            Self {
                start: self.end,
                end: self.start,
            }
        }
    }
}

#[derive(Debug, Clone)]
struct BufferSnapshot {
    rope: Rope,
}

const MAX_HISTORY: usize = 200;

#[derive(Debug, Clone)]
pub struct TextBuffer {
    rope: Rope,
    /// Cached full text for callers that need `&str`. Rebuilt on mutation.
    text_cache: String,
    undo_stack: VecDeque<BufferSnapshot>,
    redo_stack: VecDeque<BufferSnapshot>,
    revision: usize,
}

impl TextBuffer {
    pub fn new() -> Self {
        Self::from("")
    }

    pub fn from(text: &str) -> Self {
        let rope = Rope::from_str(text);
        Self {
            text_cache: text.to_string(),
            rope,
            undo_stack: VecDeque::new(),
            redo_stack: VecDeque::new(),
            revision: 0,
        }
    }

    pub fn replace(&mut self, text: &str) {
        if self.text_cache == text {
            return;
        }
        self.rope = Rope::from_str(text);
        self.text_cache = text.to_string();
        self.bump_revision();
    }

    /// Returns the full text as a borrowed string slice.
    pub fn text(&self) -> &str {
        &self.text_cache
    }

    pub fn line_count(&self) -> usize {
        // Ropey counts a trailing newline as an extra empty line; match old behavior.
        let len = self.rope.len_lines();
        if len == 0 { 1 } else { len }
    }

    /// Returns a single line without the trailing newline.
    pub fn line(&self, index: usize) -> Option<&str> {
        if index >= self.rope.len_lines() {
            return None;
        }
        let line_slice = self.rope.line(index);
        let line_str = line_slice.as_str()?;
        Some(line_str.trim_end_matches('\n'))
    }

    pub fn lines(&self) -> impl Iterator<Item = &str> {
        // Use the cached text to iterate lines, matching old split('\n') behavior.
        self.text_cache.split('\n')
    }

    #[allow(dead_code)]
    pub fn insert(&mut self, position: Position, text: &str) -> Position {
        if text.is_empty() {
            return self.clamp_position(position);
        }

        let index = self.char_index_from_position(position);
        self.rope.insert(index, text);
        self.sync_cache();
        self.bump_revision();
        self.position_from_char_index(index + text.chars().count())
    }

    #[allow(dead_code)]
    pub fn delete_range(&mut self, start: Position, end: Position) -> Position {
        let start_index = self.char_index_from_position(start);
        let end_index = self.char_index_from_position(end);
        let (from, to) = if start_index <= end_index {
            (start_index, end_index)
        } else {
            (end_index, start_index)
        };
        if from != to {
            self.rope.remove(from..to);
            self.sync_cache();
            self.bump_revision();
        }
        self.position_from_char_index(from)
    }

    fn clamp_position(&self, position: Position) -> Position {
        let line_count = self.rope.len_lines();
        let line = position.line.min(line_count.saturating_sub(1));
        let line_len = self.line_char_count(line);
        let column = position.column.min(line_len);
        Position { line, column }
    }

    fn line_char_count(&self, line: usize) -> usize {
        if line >= self.rope.len_lines() {
            return 0;
        }
        let line_slice = self.rope.line(line);
        let len = line_slice.len_chars();
        // Subtract the trailing newline if present.
        if len > 0 && line_slice.char(len - 1) == '\n' {
            len - 1
        } else {
            len
        }
    }

    /// Convert a Position (line, char column) to a char index into the rope.
    fn char_index_from_position(&self, position: Position) -> usize {
        let position = self.clamp_position(position);
        let line_start = self.rope.line_to_char(position.line);
        line_start + position.column
    }

    /// Convert a char index into the rope to a Position.
    fn position_from_char_index(&self, index: usize) -> Position {
        let index = index.min(self.rope.len_chars());
        let line = self.rope.char_to_line(index);
        let line_start = self.rope.line_to_char(line);
        let column = index - line_start;
        Position { line, column }
    }

    /// Public API: convert Position to byte index (for compatibility with app.rs).
    pub fn index_from_position(&self, position: Position) -> usize {
        let char_idx = self.char_index_from_position(position);
        self.rope.char_to_byte(char_idx)
    }

    /// Public API: convert byte index to Position (for compatibility with app.rs).
    pub fn position_from_index(&self, byte_index: usize) -> Position {
        let byte_index = byte_index.min(self.rope.len_bytes());
        let char_idx = self.rope.byte_to_char(byte_index);
        self.position_from_char_index(char_idx)
    }

    pub fn record_snapshot(&mut self) {
        let is_duplicate = self
            .undo_stack
            .back()
            .map_or(false, |snapshot| snapshot.rope == self.rope);
        if !is_duplicate {
            if self.undo_stack.len() >= MAX_HISTORY {
                self.undo_stack.pop_front();
            }
            // Rope::clone() is O(1) due to structural sharing.
            self.undo_stack
                .push_back(BufferSnapshot { rope: self.rope.clone() });
        }
        self.redo_stack.clear();
    }

    pub fn undo(&mut self) -> bool {
        let Some(snapshot) = self.undo_stack.pop_back() else {
            return false;
        };
        self.redo_stack
            .push_back(BufferSnapshot { rope: self.rope.clone() });
        self.rope = snapshot.rope;
        self.sync_cache();
        self.bump_revision();
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some(snapshot) = self.redo_stack.pop_back() else {
            return false;
        };
        self.undo_stack
            .push_back(BufferSnapshot { rope: self.rope.clone() });
        self.rope = snapshot.rope;
        self.sync_cache();
        self.bump_revision();
        true
    }

    pub fn clear_history(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

    pub fn revision(&self) -> usize {
        self.revision
    }

    fn bump_revision(&mut self) {
        self.revision = self.revision.wrapping_add(1);
    }

    fn sync_cache(&mut self) {
        self.text_cache = self.rope.to_string();
    }
}

impl Default for TextBuffer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{Cursor, Position, Selection, TextBuffer};

    fn build_text(lines: usize, line_len: usize) -> String {
        let line = "x".repeat(line_len);
        std::iter::repeat(line)
            .take(lines)
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn text_buffer_handles_large_payload_positions() {
        let text = build_text(10_000, 80);
        let buffer = TextBuffer::from(&text);

        assert_eq!(buffer.line_count(), 10_000);
        assert_eq!(buffer.line(9_999).unwrap_or(""), "x".repeat(80));

        let end_position = buffer.position_from_index(text.len());
        assert_eq!(end_position.line, 9_999);
        assert_eq!(end_position.column, 80);

        let mid_position = buffer.position_from_index(text.len() / 2);
        let back_index = buffer.index_from_position(mid_position);
        assert_eq!(back_index, text.len() / 2);
    }

    #[test]
    fn insert_single_line_text() {
        let mut buffer = TextBuffer::from("abc");
        let position = buffer.insert(Position::new(0, 1), "X");
        assert_eq!(buffer.text(), "aXbc");
        assert_eq!(position, Position::new(0, 2));
    }

    #[test]
    fn insert_multiline_text() {
        let mut buffer = TextBuffer::from("abc");
        let position = buffer.insert(Position::new(0, 1), "\nX");
        assert_eq!(buffer.text(), "a\nXbc");
        assert_eq!(position, Position::new(1, 1));
    }

    #[test]
    fn delete_across_lines() {
        let mut buffer = TextBuffer::from("ab\ncd\nef");
        let position = buffer.delete_range(Position::new(0, 1), Position::new(1, 1));
        assert_eq!(buffer.text(), "ad\nef");
        assert_eq!(position, Position::new(0, 1));
    }

    #[test]
    fn index_and_position_roundtrip() {
        let buffer = TextBuffer::from("one\ntwo\nthree");
        let pos = Position::new(2, 2);
        let index = buffer.index_from_position(pos);
        assert_eq!(buffer.position_from_index(index), pos);
    }

    #[test]
    fn undo_redo_roundtrip() {
        let mut buffer = TextBuffer::from("hello");
        buffer.record_snapshot();
        buffer.record_snapshot();
        assert_eq!(buffer.undo_stack.len(), 1);
        buffer.insert(Position::new(0, 5), " world");
        assert_eq!(buffer.text(), "hello world");
        assert!(buffer.undo());
        assert_eq!(buffer.text(), "hello");
        assert!(buffer.redo());
        assert_eq!(buffer.text(), "hello world");

        for index in 0..(super::MAX_HISTORY + 5) {
            buffer.record_snapshot();
            buffer.replace(&format!("entry {index}"));
        }
        assert_eq!(buffer.undo_stack.len(), super::MAX_HISTORY);
    }

    #[test]
    fn cursor_and_selection_helpers() {
        let cursor = Cursor::new(Position::new(2, 3));
        assert_eq!(cursor.position, Position::new(2, 3));

        let selection = Selection::new(Position::new(4, 2), Position::new(1, 9));
        assert!(!selection.is_empty());
        let normalized = selection.normalized();
        assert_eq!(normalized.start, Position::new(1, 9));
        assert_eq!(normalized.end, Position::new(4, 2));
    }

    #[test]
    fn line_returns_content_without_trailing_newline() {
        let buffer = TextBuffer::from("hello\nworld\n");
        assert_eq!(buffer.line(0), Some("hello"));
        assert_eq!(buffer.line(1), Some("world"));
        assert_eq!(buffer.line(2), Some(""));
    }

    #[test]
    fn undo_snapshot_is_cheap() {
        // Rope::clone is O(1) due to structural sharing.
        // This test just verifies correctness, not performance.
        let mut buffer = TextBuffer::from(&build_text(10_000, 80));
        buffer.record_snapshot();
        buffer.insert(Position::new(5000, 0), "INSERTED");
        assert!(buffer.undo());
        assert!(!buffer.text().contains("INSERTED"));
    }
}
