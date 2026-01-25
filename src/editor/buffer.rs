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
    text: String,
}

const MAX_HISTORY: usize = 200;

#[derive(Debug, Clone)]
pub struct TextBuffer {
    text: String,
    lines: Vec<String>,
    line_offsets: Vec<usize>,
    undo_stack: VecDeque<BufferSnapshot>,
    redo_stack: VecDeque<BufferSnapshot>,
    revision: usize,
}

impl TextBuffer {
    pub fn new() -> Self {
        Self::from("")
    }

    pub fn from(text: &str) -> Self {
        let (lines, line_offsets) = build_lines(text);
        Self {
            text: text.to_string(),
            lines,
            line_offsets,
            undo_stack: VecDeque::new(),
            redo_stack: VecDeque::new(),
            revision: 0,
        }
    }

    pub fn replace(&mut self, text: &str) {
        if self.text == text {
            return;
        }
        self.text = text.to_string();
        let (lines, line_offsets) = build_lines(text);
        self.lines = lines;
        self.line_offsets = line_offsets;
        self.bump_revision();
    }

    pub fn text(&self) -> String {
        self.text.clone()
    }

    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    pub fn line(&self, index: usize) -> Option<&str> {
        self.lines.get(index).map(String::as_str)
    }

    pub fn lines(&self) -> impl Iterator<Item = &str> {
        self.lines.iter().map(String::as_str)
    }

    #[allow(dead_code)]
    pub fn insert(&mut self, position: Position, text: &str) -> Position {
        if text.is_empty() {
            return self.clamp_position(position);
        }

        let index = self.index_from_position(position);
        let mut new_text = self.text.clone();
        new_text.insert_str(index, text);
        self.replace(&new_text);
        self.position_from_index(index.saturating_add(text.len()))
    }

    #[allow(dead_code)]
    pub fn delete_range(&mut self, start: Position, end: Position) -> Position {
        let start_index = self.index_from_position(start);
        let end_index = self.index_from_position(end);
        let (from, to) = if start_index <= end_index {
            (start_index, end_index)
        } else {
            (end_index, start_index)
        };
        if from != to {
            let mut new_text = self.text.clone();
            new_text.replace_range(from..to, "");
            self.replace(&new_text);
        }
        self.position_from_index(from)
    }

    fn clamp_position(&self, position: Position) -> Position {
        let line = position.line.min(self.lines.len().saturating_sub(1));
        let column = position
            .column
            .min(self.lines.get(line).map_or(0, String::len));
        Position { line, column }
    }

    pub fn index_from_position(&self, position: Position) -> usize {
        let position = self.clamp_position(position);
        let line_offset = self.line_offsets.get(position.line).copied().unwrap_or(0);
        line_offset.saturating_add(position.column)
    }

    pub fn position_from_index(&self, mut index: usize) -> Position {
        if self.lines.is_empty() {
            return Position { line: 0, column: 0 };
        }

        if index > self.text.len() {
            index = self.text.len();
        }

        let line = match self
            .line_offsets
            .iter()
            .rposition(|offset| *offset <= index)
        {
            Some(line_index) => line_index,
            None => 0,
        };
        let line_offset = self.line_offsets.get(line).copied().unwrap_or(0);
        let column = index.saturating_sub(line_offset);
        let line_len = self.lines.get(line).map_or(0, String::len);
        Position {
            line,
            column: column.min(line_len),
        }
    }

    pub fn record_snapshot(&mut self) {
        if self.undo_stack.len() >= MAX_HISTORY {
            self.undo_stack.pop_front();
        }
        self.undo_stack.push_back(BufferSnapshot {
            text: self.text.clone(),
        });
        self.redo_stack.clear();
    }

    pub fn undo(&mut self) -> Option<String> {
        let snapshot = self.undo_stack.pop_back()?;
        self.redo_stack.push_back(BufferSnapshot {
            text: self.text.clone(),
        });
        self.replace(&snapshot.text);
        Some(snapshot.text)
    }

    pub fn redo(&mut self) -> Option<String> {
        let snapshot = self.redo_stack.pop_back()?;
        self.undo_stack.push_back(BufferSnapshot {
            text: self.text.clone(),
        });
        self.replace(&snapshot.text);
        Some(snapshot.text)
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
}

fn build_lines(text: &str) -> (Vec<String>, Vec<usize>) {
    let mut lines: Vec<String> = text.split('\n').map(String::from).collect();
    if lines.is_empty() {
        lines.push(String::new());
    }

    let mut offsets = Vec::with_capacity(lines.len());
    let mut index = 0usize;
    for line in &lines {
        offsets.push(index);
        index = index.saturating_add(line.len() + 1);
    }
    (lines, offsets)
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
        buffer.insert(Position::new(0, 5), " world");
        assert_eq!(buffer.text(), "hello world");
        buffer.undo();
        assert_eq!(buffer.text(), "hello");
        buffer.redo();
        assert_eq!(buffer.text(), "hello world");
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
}
