use crate::editor::highlight::{HighlightToken, Language};
use streaming_iterator::StreamingIterator;
use std::ops::Range;
use std::sync::LazyLock;
use tree_sitter::{InputEdit, Parser, Point, Query, QueryCursor, Tree};

struct LangDef {
    ts_language: tree_sitter::Language,
    highlights_query: &'static str,
}

fn lang_def_for(language: Language) -> Option<&'static LangDef> {
    static RUST: LazyLock<LangDef> = LazyLock::new(|| LangDef {
        ts_language: tree_sitter_rust::LANGUAGE.into(),
        highlights_query: tree_sitter_rust::HIGHLIGHTS_QUERY,
    });
    static JAVASCRIPT: LazyLock<LangDef> = LazyLock::new(|| LangDef {
        ts_language: tree_sitter_javascript::LANGUAGE.into(),
        highlights_query: tree_sitter_javascript::HIGHLIGHT_QUERY,
    });
    static PYTHON: LazyLock<LangDef> = LazyLock::new(|| LangDef {
        ts_language: tree_sitter_python::LANGUAGE.into(),
        highlights_query: tree_sitter_python::HIGHLIGHTS_QUERY,
    });
    static C: LazyLock<LangDef> = LazyLock::new(|| LangDef {
        ts_language: tree_sitter_c::LANGUAGE.into(),
        highlights_query: tree_sitter_c::HIGHLIGHT_QUERY,
    });
    static GO: LazyLock<LangDef> = LazyLock::new(|| LangDef {
        ts_language: tree_sitter_go::LANGUAGE.into(),
        highlights_query: tree_sitter_go::HIGHLIGHTS_QUERY,
    });
    static JSON: LazyLock<LangDef> = LazyLock::new(|| LangDef {
        ts_language: tree_sitter_json::LANGUAGE.into(),
        highlights_query: tree_sitter_json::HIGHLIGHTS_QUERY,
    });
    static TOML: LazyLock<LangDef> = LazyLock::new(|| LangDef {
        ts_language: tree_sitter_toml_ng::LANGUAGE.into(),
        highlights_query: tree_sitter_toml_ng::HIGHLIGHTS_QUERY,
    });
    static MARKDOWN: LazyLock<LangDef> = LazyLock::new(|| LangDef {
        ts_language: tree_sitter_md::LANGUAGE.into(),
        highlights_query: tree_sitter_md::HIGHLIGHT_QUERY_BLOCK,
    });
    match language {
        Language::Rust => Some(&RUST),
        Language::JavaScript => Some(&JAVASCRIPT),
        Language::Python => Some(&PYTHON),
        Language::C => Some(&C),
        Language::Go => Some(&GO),
        Language::Json => Some(&JSON),
        Language::Toml => Some(&TOML),
        Language::Markdown => Some(&MARKDOWN),
        Language::Plain => None,
    }
}

pub struct SyntaxHighlighter {
    inner: Option<GenericHighlighter>,
}

impl SyntaxHighlighter {
    pub fn new(language: Language, buffer_text: &str) -> Option<Self> {
        let inner = lang_def_for(language)
            .and_then(|def| GenericHighlighter::new(def, buffer_text));
        Some(Self { inner })
    }

    pub fn update_text(&mut self, buffer_text: &str) {
        if let Some(inner) = self.inner.as_mut() {
            inner.update_text(buffer_text);
        }
    }

    pub fn highlight_range(
        &mut self,
        line_range: Range<usize>,
    ) -> Option<Vec<(Range<usize>, HighlightToken)>> {
        self.inner.as_mut()?.highlight_range(line_range)
    }

    pub fn line_byte_range(&self, line_index: usize) -> Option<Range<usize>> {
        self.inner.as_ref()?.line_byte_range(line_index)
    }
}

struct GenericHighlighter {
    parser: Parser,
    query: Query,
    tree: Option<Tree>,
    text: String,
    line_offsets: Vec<usize>,
}

impl GenericHighlighter {
    fn new(def: &LangDef, buffer_text: &str) -> Option<Self> {
        let query = Query::new(&def.ts_language, def.highlights_query).ok()?;
        let mut parser = Parser::new();
        parser.set_language(&def.ts_language).ok()?;
        let tree = parser.parse(buffer_text, None);
        let text = buffer_text.to_string();
        let line_offsets = compute_line_offsets(&text);
        Some(Self {
            parser,
            query,
            tree,
            text,
            line_offsets,
        })
    }

    fn update_text(&mut self, buffer_text: &str) {
        if self.text == buffer_text {
            return;
        }
        if let Some(tree) = self.tree.as_mut() {
            if let Some(edit) = compute_input_edit(&self.text, buffer_text) {
                tree.edit(&edit);
            }
        }
        self.tree = self.parser.parse(buffer_text, self.tree.as_ref());
        self.text = buffer_text.to_string();
        self.line_offsets = compute_line_offsets(&self.text);
    }

    fn highlight_range(
        &mut self,
        line_range: Range<usize>,
    ) -> Option<Vec<(Range<usize>, HighlightToken)>> {
        let tree = self.tree.as_ref()?;
        let line_count = self.line_offsets.len();
        if line_range.start >= line_count {
            return Some(Vec::new());
        }

        let start_byte = self.line_offsets[line_range.start];
        let clamped_end = line_range.end.min(line_count);
        let end_byte = if clamped_end < line_count {
            self.line_offsets[clamped_end]
        } else {
            self.text.len()
        };

        let mut cursor = QueryCursor::new();
        cursor.set_byte_range(start_byte..end_byte);
        let mut highlights = Vec::new();
        let mut captures = cursor.captures(&self.query, tree.root_node(), self.text.as_bytes());
        while let Some((query_match, capture_index)) = captures.next() {
            let capture = query_match.captures[*capture_index];
            let range = capture.node.byte_range();
            if range.end <= start_byte || range.start >= end_byte {
                continue;
            }
            let capture_name = self.query.capture_names()[capture.index as usize];
            let token = if capture_name.starts_with("comment") {
                HighlightToken::Comment
            } else if capture_name.starts_with("string") {
                HighlightToken::String
            } else if capture_name.starts_with("keyword") {
                HighlightToken::Keyword
            } else if capture_name.starts_with("type") {
                HighlightToken::Type
            } else if capture_name.starts_with("constant") || capture_name.starts_with("number") {
                HighlightToken::Number
            } else {
                continue;
            };
            highlights.push((range, token));
        }

        Some(highlights)
    }

    fn line_byte_range(&self, line_index: usize) -> Option<Range<usize>> {
        let start = self.line_offsets.get(line_index).copied()?;
        let end = self
            .line_offsets
            .get(line_index + 1)
            .copied()
            .unwrap_or(self.text.len());
        Some(start..end)
    }
}

fn compute_line_offsets(text: &str) -> Vec<usize> {
    let mut offsets = vec![0];
    for (index, byte) in text.bytes().enumerate() {
        if byte == b'\n' {
            offsets.push(index + 1);
        }
    }
    offsets
}

fn compute_input_edit(old_text: &str, new_text: &str) -> Option<InputEdit> {
    if old_text == new_text {
        return None;
    }
    let old_bytes = old_text.as_bytes();
    let new_bytes = new_text.as_bytes();
    let mut start = 0;
    let min_len = old_bytes.len().min(new_bytes.len());
    while start < min_len && old_bytes[start] == new_bytes[start] {
        start += 1;
    }

    let mut old_end = old_bytes.len();
    let mut new_end = new_bytes.len();
    while old_end > start
        && new_end > start
        && old_bytes[old_end - 1] == new_bytes[new_end - 1]
    {
        old_end -= 1;
        new_end -= 1;
    }

    Some(InputEdit {
        start_byte: start,
        old_end_byte: old_end,
        new_end_byte: new_end,
        start_position: point_for_byte(old_text, start),
        old_end_position: point_for_byte(old_text, old_end),
        new_end_position: point_for_byte(new_text, new_end),
    })
}

fn point_for_byte(text: &str, byte_index: usize) -> Point {
    let mut row = 0;
    let mut column = 0;
    let mut current = 0;
    for byte in text.bytes() {
        if current == byte_index {
            break;
        }
        if byte == b'\n' {
            row += 1;
            column = 0;
        } else {
            column += 1;
        }
        current += 1;
    }
    Point { row, column }
}
