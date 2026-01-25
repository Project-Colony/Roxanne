use crate::theme::SyntaxPalette;
use iced::advanced::text::highlighter::{self, Highlighter};
use iced::{Font, Theme};
use std::ops::Range;
use std::sync::{LazyLock, RwLock};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum Language {
    Plain,
    Rust,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    pub language: Language,
    pub search_matches: Vec<MatchPosition>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            language: Language::Rust,
            search_matches: Vec::new(),
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

#[derive(Debug, Clone)]
pub struct RoxanneHighlighter {
    settings: Settings,
    current_line: usize,
}

impl Highlighter for RoxanneHighlighter {
    type Settings = Settings;
    type Highlight = HighlightToken;
    type Iterator<'a> = std::vec::IntoIter<(Range<usize>, Self::Highlight)>;

    fn new(settings: &Self::Settings) -> Self {
        Self {
            settings: settings.clone(),
            current_line: 0,
        }
    }

    fn update(&mut self, new_settings: &Self::Settings) {
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
            Language::Rust => highlight_rust_line(line),
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

    if let Some(comment_start) = line.find("//") {
        highlights.push((comment_start..line.len(), HighlightToken::Comment));
        protected_ranges.push(comment_start..line.len());
    }

    let mut search_index = 0;
    while let Some(start) = line[search_index..].find('"') {
        let quote_start = search_index + start;
        let after_start = quote_start + 1;
        if let Some(end) = line[after_start..].find('"') {
            let quote_end = after_start + end + 1;
            highlights.push((quote_start..quote_end, HighlightToken::String));
            protected_ranges.push(quote_start..quote_end);
            search_index = quote_end;
        } else {
            highlights.push((quote_start..line.len(), HighlightToken::String));
            protected_ranges.push(quote_start..line.len());
            break;
        }
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

fn is_in_ranges(start: usize, end: usize, ranges: &[Range<usize>]) -> bool {
    ranges
        .iter()
        .any(|range| start < range.end && end > range.start)
}
