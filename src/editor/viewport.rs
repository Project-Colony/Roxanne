use crate::editor::TextBuffer;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ViewportLine {
    pub line: usize,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct ViewportCache {
    start_line: usize,
    height: usize,
    lines: Vec<ViewportLine>,
    revision: usize,
}

impl ViewportCache {
    pub fn new() -> Self {
        Self {
            start_line: 0,
            height: 0,
            lines: Vec::new(),
            revision: 0,
        }
    }

    pub fn update(&mut self, buffer: &TextBuffer, start_line: usize, height: usize) {
        let height = height.max(1);
        let line_count = buffer.line_count().max(1);
        let start_line = start_line.min(line_count.saturating_sub(1));
        let end_line = (start_line + height).min(line_count);

        let mut cached_map: HashMap<usize, String> = self
            .lines
            .iter()
            .map(|line| (line.line, line.text.clone()))
            .collect();

        let mut next_lines = Vec::with_capacity(end_line.saturating_sub(start_line));
        let revision_changed = self.revision != buffer.revision();

        for line_index in start_line..end_line {
            let current = buffer.line(line_index).unwrap_or("");
            if let Some(cached) = cached_map.remove(&line_index) {
                if !revision_changed || cached == current {
                    next_lines.push(ViewportLine {
                        line: line_index,
                        text: cached,
                    });
                    continue;
                }
            }

            next_lines.push(ViewportLine {
                line: line_index,
                text: current.to_string(),
            });
        }

        self.start_line = start_line;
        self.height = height;
        self.lines = next_lines;
        self.revision = buffer.revision();
    }

    pub fn range(&self) -> Option<(usize, usize)> {
        if self.lines.is_empty() {
            None
        } else {
            Some((self.start_line, self.start_line + self.lines.len()))
        }
    }

    pub fn lines(&self) -> &[ViewportLine] {
        &self.lines
    }
}

impl Default for ViewportCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::ViewportCache;
    use crate::editor::TextBuffer;

    #[test]
    fn caches_visible_lines() {
        let buffer = TextBuffer::from("one\ntwo\nthree\nfour");
        let mut cache = ViewportCache::new();
        cache.update(&buffer, 1, 2);

        let range = cache.range().expect("range");
        assert_eq!(range, (1, 3));
        let lines = cache.lines();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].text, "two");
        assert_eq!(lines[1].text, "three");
    }

    #[test]
    fn refreshes_changed_lines() {
        let mut buffer = TextBuffer::from("alpha\nbravo\ncharlie");
        let mut cache = ViewportCache::new();
        cache.update(&buffer, 0, 3);
        buffer.replace("alpha\nbeta\ncharlie");
        cache.update(&buffer, 0, 3);

        let lines = cache.lines();
        assert_eq!(lines[1].text, "beta");
    }
}
