use crate::editor::syntax;
use crate::theme::SyntaxPalette;
use iced::advanced::text::highlighter::{self, Highlighter};
use iced::{Font, Theme};
use std::ops::Range;
use std::sync::{Arc, LazyLock, RwLock};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum Language {
    Plain,
    Rust,
    JavaScript,
    Python,
    C,
    Go,
    Json,
    Toml,
    Markdown,
}

impl Language {
    pub fn from_extension(ext: &str) -> Self {
        match ext {
            "rs" => Language::Rust,
            "js" | "jsx" | "mjs" | "cjs" => Language::JavaScript,
            "py" | "pyi" => Language::Python,
            "c" | "h" => Language::C,
            "go" => Language::Go,
            "json" => Language::Json,
            "toml" => Language::Toml,
            "md" | "markdown" => Language::Markdown,
            _ => Language::Plain,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    pub language: Language,
    pub search_matches: Vec<MatchPosition>,
    pub buffer_text: Arc<str>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            language: Language::Rust,
            search_matches: Vec::new(),
            buffer_text: Arc::from(""),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HighlightToken {
    Keyword,
    Type,
    String,
    Comment,
    Number,
    SearchMatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MatchPosition {
    pub line: usize,
    pub column: usize,
    pub length: usize,
}

pub struct RoxanneHighlighter {
    settings: Settings,
    current_line: usize,
    syntax_highlighter: Option<syntax::SyntaxHighlighter>,
}

impl Highlighter for RoxanneHighlighter {
    type Settings = Settings;
    type Highlight = HighlightToken;
    type Iterator<'a> = std::vec::IntoIter<(Range<usize>, Self::Highlight)>;

    fn new(settings: &Self::Settings) -> Self {
        Self {
            settings: settings.clone(),
            current_line: 0,
            syntax_highlighter: syntax::SyntaxHighlighter::new(
                settings.language,
                &settings.buffer_text,
            ),
        }
    }

    fn update(&mut self, new_settings: &Self::Settings) {
        if self.settings.language != new_settings.language {
            self.syntax_highlighter =
                syntax::SyntaxHighlighter::new(new_settings.language, &new_settings.buffer_text);
        } else if let Some(highlighter) = self.syntax_highlighter.as_mut() {
            highlighter.update_text(&new_settings.buffer_text);
        } else {
            self.syntax_highlighter =
                syntax::SyntaxHighlighter::new(new_settings.language, &new_settings.buffer_text);
        }

        if &self.settings != new_settings {
            self.settings = new_settings.clone();
            self.current_line = 0;
        }
    }

    fn change_line(&mut self, line: usize) {
        self.current_line = line;
    }

    fn highlight_line(&mut self, line: &str) -> Self::Iterator<'_> {
        let line_index = self.current_line;
        self.current_line = self.current_line.saturating_add(1);

        let mut highlights = match self.settings.language {
            Language::Plain => Vec::new(),
            _ => {
                if let Some(highlighter) = self.syntax_highlighter.as_mut() {
                    if let Some(line_range) = highlighter.line_byte_range(line_index) {
                        let line_start = line_range.start;
                        let line_end = line_start + line.len();
                        let tokens = highlighter
                            .highlight_range(line_index..line_index.saturating_add(1))
                            .unwrap_or_default();
                        tokens
                            .into_iter()
                            .filter_map(|(range, token)| {
                                let start = range.start.max(line_start);
                                let end = range.end.min(line_end);
                                if start < end {
                                    Some((start - line_start..end - line_start, token))
                                } else {
                                    None
                                }
                            })
                            .collect()
                    } else {
                        Vec::new()
                    }
                } else {
                    highlight_generic_line(line, self.settings.language)
                }
            }
        };

        if !self.settings.search_matches.is_empty() {
            let mut char_to_byte = Vec::new();
            char_to_byte.reserve(line.chars().count() + 1);
            for (byte_index, _) in line.char_indices() {
                char_to_byte.push(byte_index);
            }
            char_to_byte.push(line.len());
            let mut match_ranges = self
                .settings
                .search_matches
                .iter()
                .filter(|match_position| match_position.line == line_index)
                .map(|match_position| {
                    let start_char = match_position.column;
                    let end_char = match_position.column + match_position.length;
                    let start = char_to_byte
                        .get(start_char)
                        .copied()
                        .unwrap_or(line.len());
                    let end = char_to_byte.get(end_char).copied().unwrap_or(line.len());
                    (
                        start..end,
                        HighlightToken::SearchMatch,
                    )
                })
                .collect::<Vec<_>>();
            highlights.append(&mut match_ranges);
        }

        highlights.into_iter()
    }

    fn current_line(&self) -> usize {
        self.current_line
    }
}

pub fn highlight_format(token: &HighlightToken, _theme: &Theme) -> highlighter::Format<Font> {
    let palette = current_syntax_palette();
    let color = match token {
        HighlightToken::Keyword => palette.keyword,
        HighlightToken::Type => palette.r#type,
        HighlightToken::String => palette.string,
        HighlightToken::Comment => palette.comment,
        HighlightToken::Number => palette.number,
        HighlightToken::SearchMatch => palette.search_match,
    };

    highlighter::Format {
        color: Some(color),
        font: None,
    }
}

static SYNTAX_PALETTE: LazyLock<RwLock<SyntaxPalette>> =
    LazyLock::new(|| RwLock::new(SyntaxPalette::default()));

pub fn set_syntax_palette(palette: SyntaxPalette) {
    if let Ok(mut current) = SYNTAX_PALETTE.write() {
        *current = palette;
    }
}

fn current_syntax_palette() -> SyntaxPalette {
    SYNTAX_PALETTE
        .read()
        .map(|palette| *palette)
        .unwrap_or_else(|_| SyntaxPalette::default())
}

fn highlight_rust_line(line: &str) -> Vec<(Range<usize>, HighlightToken)> {
    let mut highlights = Vec::new();
    let mut protected_ranges = Vec::new();
    let bytes = line.as_bytes();
    let mut string_start = None;
    let mut escaped = false;
    let mut comment_start = None;

    let mut index = 0;
    while index < bytes.len() {
        let byte = bytes[index];

        if let Some(start) = string_start {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                let end = index + 1;
                highlights.push((start..end, HighlightToken::String));
                protected_ranges.push(start..end);
                string_start = None;
            }
            index += 1;
            continue;
        }

        if byte == b'"' {
            string_start = Some(index);
            index += 1;
            continue;
        }

        if byte == b'/' && index + 1 < bytes.len() && bytes[index + 1] == b'/' {
            comment_start = Some(index);
            break;
        }

        index += 1;
    }

    if let Some(start) = string_start {
        highlights.push((start..line.len(), HighlightToken::String));
        protected_ranges.push(start..line.len());
    }

    if let Some(start) = comment_start {
        highlights.push((start..line.len(), HighlightToken::Comment));
        protected_ranges.push(start..line.len());
    }

    let keywords = [
        "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false", "fn",
        "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref",
        "return", "self", "Self", "static", "struct", "super", "trait", "true", "type", "unsafe",
        "use", "where", "while",
    ];
    let types = [
        "bool", "char", "i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16", "u32", "u64",
        "u128", "usize", "f32", "f64", "str", "String", "Option", "Result", "Vec",
    ];

    let mut word_start = None;
    for (index, ch) in line.char_indices() {
        if ch.is_alphanumeric() || ch == '_' {
            if word_start.is_none() {
                word_start = Some(index);
            }
        } else if let Some(start) = word_start.take() {
            let end = index;
            if !is_in_ranges(start, end, &protected_ranges) {
                let word = &line[start..end];
                if keywords.contains(&word) {
                    highlights.push((start..end, HighlightToken::Keyword));
                } else if types.contains(&word) {
                    highlights.push((start..end, HighlightToken::Type));
                } else if word.chars().all(|c| c.is_numeric()) {
                    highlights.push((start..end, HighlightToken::Number));
                }
            }
        }
    }

    if let Some(start) = word_start.take() {
        let end = line.len();
        if !is_in_ranges(start, end, &protected_ranges) {
            let word = &line[start..end];
            if keywords.contains(&word) {
                highlights.push((start..end, HighlightToken::Keyword));
            } else if types.contains(&word) {
                highlights.push((start..end, HighlightToken::Type));
            } else if word.chars().all(|c| c.is_numeric()) {
                highlights.push((start..end, HighlightToken::Number));
            }
        }
    }

    highlights
}

/// Generic fallback highlighter that works for any language.
/// Highlights strings, comments (// and #), and numbers.
/// For Rust, also highlights Rust keywords and types.
fn highlight_generic_line(line: &str, language: Language) -> Vec<(Range<usize>, HighlightToken)> {
    if language == Language::Rust {
        return highlight_rust_line(line);
    }

    let mut highlights = Vec::new();
    let mut protected_ranges = Vec::new();
    let bytes = line.as_bytes();
    let mut string_start = None;
    let mut string_delimiter = b'"';
    let mut escaped = false;

    let mut index = 0;
    while index < bytes.len() {
        let byte = bytes[index];

        if let Some(start) = string_start {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == string_delimiter {
                let end = index + 1;
                highlights.push((start..end, HighlightToken::String));
                protected_ranges.push(start..end);
                string_start = None;
            }
            index += 1;
            continue;
        }

        // String start
        if byte == b'"' || byte == b'\'' {
            string_start = Some(index);
            string_delimiter = byte;
            index += 1;
            continue;
        }

        // Line comment: // or #
        if byte == b'/' && index + 1 < bytes.len() && bytes[index + 1] == b'/' {
            highlights.push((index..line.len(), HighlightToken::Comment));
            protected_ranges.push(index..line.len());
            break;
        }
        if byte == b'#' && (language == Language::Python) {
            highlights.push((index..line.len(), HighlightToken::Comment));
            protected_ranges.push(index..line.len());
            break;
        }

        index += 1;
    }

    // Unclosed string
    if let Some(start) = string_start {
        highlights.push((start..line.len(), HighlightToken::String));
        protected_ranges.push(start..line.len());
    }

    // Keywords per language
    let keywords: &[&str] = match language {
        Language::JavaScript => &[
            "var", "let", "const", "function", "return", "if", "else", "for", "while", "do",
            "switch", "case", "break", "continue", "new", "this", "class", "extends", "import",
            "export", "default", "from", "async", "await", "try", "catch", "finally", "throw",
            "typeof", "instanceof", "in", "of", "true", "false", "null", "undefined", "yield",
        ],
        Language::Python => &[
            "def", "class", "return", "if", "elif", "else", "for", "while", "break", "continue",
            "import", "from", "as", "try", "except", "finally", "raise", "with", "yield",
            "lambda", "pass", "True", "False", "None", "and", "or", "not", "in", "is", "global",
            "nonlocal", "assert", "del", "async", "await",
        ],
        Language::C => &[
            "auto", "break", "case", "char", "const", "continue", "default", "do", "double",
            "else", "enum", "extern", "float", "for", "goto", "if", "int", "long", "register",
            "return", "short", "signed", "sizeof", "static", "struct", "switch", "typedef",
            "union", "unsigned", "void", "volatile", "while", "inline", "restrict",
        ],
        Language::Go => &[
            "break", "case", "chan", "const", "continue", "default", "defer", "else",
            "fallthrough", "for", "func", "go", "goto", "if", "import", "interface", "map",
            "package", "range", "return", "select", "struct", "switch", "type", "var",
            "true", "false", "nil",
        ],
        _ => &[],
    };

    let mut word_start = None;
    for (idx, ch) in line.char_indices() {
        if ch.is_alphanumeric() || ch == '_' {
            if word_start.is_none() {
                word_start = Some(idx);
            }
        } else if let Some(start) = word_start.take() {
            let end = idx;
            if !is_in_ranges(start, end, &protected_ranges) {
                let word = &line[start..end];
                if keywords.contains(&word) {
                    highlights.push((start..end, HighlightToken::Keyword));
                } else if word.chars().all(|c| c.is_numeric() || c == '.') && word.chars().any(|c| c.is_numeric()) {
                    highlights.push((start..end, HighlightToken::Number));
                }
            }
        }
    }
    if let Some(start) = word_start.take() {
        let end = line.len();
        if !is_in_ranges(start, end, &protected_ranges) {
            let word = &line[start..end];
            if keywords.contains(&word) {
                highlights.push((start..end, HighlightToken::Keyword));
            } else if word.chars().all(|c| c.is_numeric() || c == '.') && word.chars().any(|c| c.is_numeric()) {
                highlights.push((start..end, HighlightToken::Number));
            }
        }
    }

    highlights
}

fn is_in_ranges(start: usize, end: usize, ranges: &[Range<usize>]) -> bool {
    ranges
        .iter()
        .any(|range| start < range.end && end > range.start)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fallback_ignores_comment_markers_inside_string() {
        let line = r#"let s = "http://example.com // still string";"#;
        let highlights = highlight_rust_line(line);

        assert!(
            !highlights
                .iter()
                .any(|(_, token)| *token == HighlightToken::Comment),
            "expected no comment tokens in line: {line}"
        );
    }

    #[test]
    fn does_not_mark_string_slashes_as_comment() {
        let line = r#"let s = "// not comment";"#;
        let settings = Settings {
            buffer_text: line.into(),
            ..Settings::default()
        };
        let mut highlighter = RoxanneHighlighter::new(&settings);
        let highlights: Vec<_> = highlighter.highlight_line(line).collect();

        assert!(
            !highlights
                .iter()
                .any(|(_, token)| *token == HighlightToken::Comment),
            "expected no comment tokens in line: {line}"
        );
    }

    #[test]
    fn still_highlights_actual_comment_after_string() {
        let line = r#"let s = "// not comment"; // real comment"#;
        let settings = Settings {
            buffer_text: line.into(),
            ..Settings::default()
        };
        let mut highlighter = RoxanneHighlighter::new(&settings);
        let highlights: Vec<_> = highlighter.highlight_line(line).collect();
        let comment_start = line
            .rfind("// real comment")
            .expect("comment marker should exist");

        let comment_ranges: Vec<_> = highlights
            .iter()
            .filter_map(|(range, token)| {
                if *token == HighlightToken::Comment {
                    Some(range.clone())
                } else {
                    None
                }
            })
            .collect();

        assert!(
            comment_ranges.iter().any(|range| range.start == comment_start),
            "expected comment token starting at {comment_start}, got {comment_ranges:?}"
        );
    }

    #[test]
    fn tree_sitter_keeps_multiline_comments_coherent() {
        let text = "/* bloc\ncommentaire */\nlet value = 42;";
        let settings = Settings {
            buffer_text: text.into(),
            ..Settings::default()
        };
        let mut highlighter = RoxanneHighlighter::new(&settings);
        let first_line_highlights: Vec<_> = highlighter.highlight_line("/* bloc").collect();
        let second_line_highlights: Vec<_> = highlighter.highlight_line("commentaire */").collect();

        assert!(
            first_line_highlights
                .iter()
                .any(|(_, token)| *token == HighlightToken::Comment),
            "expected comment token on first line"
        );
        assert!(
            second_line_highlights
                .iter()
                .any(|(_, token)| *token == HighlightToken::Comment),
            "expected comment token on second line"
        );
    }

    #[test]
    fn generic_fallback_highlights_python_keywords() {
        let highlights = highlight_generic_line("def foo(x):", Language::Python);
        assert!(
            highlights.iter().any(|(range, token)| {
                *token == HighlightToken::Keyword && &"def foo(x):"[range.clone()] == "def"
            }),
            "expected 'def' to be highlighted as keyword"
        );
    }

    #[test]
    fn generic_fallback_highlights_js_keywords() {
        let highlights = highlight_generic_line("const x = 42;", Language::JavaScript);
        assert!(
            highlights.iter().any(|(_, token)| *token == HighlightToken::Keyword),
            "expected 'const' to be highlighted as keyword"
        );
    }

    #[test]
    fn generic_fallback_highlights_python_comments() {
        let highlights = highlight_generic_line("x = 1 # comment", Language::Python);
        assert!(
            highlights.iter().any(|(_, token)| *token == HighlightToken::Comment),
            "expected comment token"
        );
    }

    #[test]
    fn generic_fallback_highlights_strings() {
        let highlights = highlight_generic_line(r#"let s = "hello";"#, Language::JavaScript);
        assert!(
            highlights.iter().any(|(_, token)| *token == HighlightToken::String),
            "expected string token"
        );
    }

    #[test]
    fn generic_fallback_highlights_numbers() {
        let highlights = highlight_generic_line("let x = 42;", Language::Go);
        assert!(
            highlights.iter().any(|(_, token)| *token == HighlightToken::Number),
            "expected number token"
        );
    }

    #[test]
    fn markdown_language_detection() {
        assert_eq!(Language::from_extension("md"), Language::Markdown);
        assert_eq!(Language::from_extension("markdown"), Language::Markdown);
    }
}
