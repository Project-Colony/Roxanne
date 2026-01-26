use crate::editor::highlight::{HighlightToken, Language};
use streaming_iterator::StreamingIterator;
use std::ops::Range;
use std::sync::{LazyLock, Mutex};
use tree_sitter::{Parser, Query, QueryCursor};

static RUST_LANGUAGE: LazyLock<tree_sitter::Language> =
    LazyLock::new(|| tree_sitter_rust::LANGUAGE.into());

static RUST_PARSER: LazyLock<Result<Mutex<Parser>, tree_sitter::LanguageError>> = LazyLock::new(
    || {
        let mut parser = Parser::new();
        parser.set_language(&*RUST_LANGUAGE)?;
        Ok(Mutex::new(parser))
    },
);

static RUST_QUERY: LazyLock<Result<Query, tree_sitter::QueryError>> = LazyLock::new(|| {
    Query::new(&*RUST_LANGUAGE, tree_sitter_rust::HIGHLIGHTS_QUERY)
});

pub fn highlight_line(
    language: Language,
    line: &str,
) -> Option<Vec<(Range<usize>, HighlightToken)>> {
    match language {
        Language::Rust => highlight_rust_line(line),
        Language::Plain => None,
    }
}


fn highlight_rust_line(line: &str) -> Option<Vec<(Range<usize>, HighlightToken)>> {
    let parser = match &*RUST_PARSER {
        Ok(parser) => parser,
        Err(_) => return None,
    };
    let query = match &*RUST_QUERY {
        Ok(query) => query,
        Err(_) => return None,
    };

    let mut parser = parser.lock().ok()?;
    let tree = parser.parse(line, None)?;
    let mut cursor = QueryCursor::new();
    let mut highlights = Vec::new();
    let mut captures = cursor.captures(query, tree.root_node(), line.as_bytes());
    while let Some((query_match, capture_index)) = captures.next() {
        let capture = query_match.captures[*capture_index];
        let range = capture.node.byte_range();
        let capture_name = query.capture_names()[capture.index as usize];
        let token = if capture_name.starts_with("comment") {
            HighlightToken::Comment
        } else if capture_name == "string" {
            HighlightToken::String
        } else if capture_name == "keyword" {
            HighlightToken::Keyword
        } else if capture_name.starts_with("type") {
            HighlightToken::Type
        } else if capture_name.starts_with("constant") {
            HighlightToken::Number
        } else {
            continue;
        };
        highlights.push((range.clone(), token));
    }

    Some(highlights)
}
