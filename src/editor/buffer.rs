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

#[derive(Debug, Clone)]
pub struct TextBuffer {
    lines: Vec<String>,
}

impl TextBuffer {
    pub fn new() -> Self {
        Self {
            lines: vec![String::new()],
        }
    }

    pub fn from(text: &str) -> Self {
        let mut lines: Vec<String> = text.split('\n').map(String::from).collect();
        if lines.is_empty() {
            lines.push(String::new());
        }
        Self { lines }
    }

    pub fn replace(&mut self, text: &str) {
        *self = Self::from(text);
    }

    pub fn text(&self) -> String {
        self.lines.join("\n")
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

    pub fn insert(&mut self, position: Position, text: &str) -> Position {
        if text.is_empty() {
            return self.clamp_position(position);
        }

        let mut full_text = self.text();
        let index = self.index_from_position(position);
        full_text.insert_str(index, text);
        self.replace(&full_text);
        self.position_from_index(index + text.len())
    }

    pub fn delete_range(&mut self, start: Position, end: Position) -> Position {
        let mut full_text = self.text();
        let start_index = self.index_from_position(start);
        let end_index = self.index_from_position(end);
        let (from, to) = if start_index <= end_index {
            (start_index, end_index)
        } else {
            (end_index, start_index)
        };
        if from != to {
            full_text.replace_range(from..to, "");
            self.replace(&full_text);
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
        let mut index = 0;
        for (line_index, line) in self.lines.iter().enumerate() {
            if line_index == position.line {
                return index + position.column;
            }
            index += line.len() + 1;
        }
        index
    }

    pub fn position_from_index(&self, mut index: usize) -> Position {
        for (line_index, line) in self.lines.iter().enumerate() {
            if index <= line.len() {
                return Position {
                    line: line_index,
                    column: index,
                };
            }
            index = index.saturating_sub(line.len() + 1);
        }
        let last_line = self.lines.len().saturating_sub(1);
        Position {
            line: last_line,
            column: self.lines.last().map_or(0, |line| line.len().min(index)),
        }
    }
}

impl Default for TextBuffer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{Position, TextBuffer};

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
}
