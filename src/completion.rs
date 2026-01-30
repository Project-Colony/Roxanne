use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct CompletionItem {
    pub label: String,
    pub detail: String,
}

pub fn build_items(prefix: &str) -> Vec<CompletionItem> {
    if prefix.is_empty() {
        return Vec::new();
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
    let mut items = Vec::new();
    let mut seen = HashSet::new();
    for keyword in keywords.iter().chain(types.iter()) {
        if keyword.starts_with(prefix) && seen.insert(*keyword) {
            let detail = if types.contains(keyword) {
                "Type"
            } else {
                "Keyword"
            };
            items.push(CompletionItem {
                label: (*keyword).to_string(),
                detail: detail.to_string(),
            });
        }
    }
    items.sort_by(|a, b| a.label.cmp(&b.label));
    items
}

pub fn extract_prefix(line: &str, column: usize) -> String {
    let byte_column = char_index_to_byte_index(line, column);
    let mut start = byte_column;
    for (index, ch) in line.char_indices() {
        if index >= byte_column {
            break;
        }
        if !ch.is_alphanumeric() && ch != '_' {
            start = index + ch.len_utf8();
        }
    }
    line[start..byte_column].to_string()
}

fn char_index_to_byte_index(line: &str, char_index: usize) -> usize {
    if char_index == 0 {
        return 0;
    }
    line.char_indices()
        .nth(char_index)
        .map(|(index, _)| index)
        .unwrap_or(line.len())
}
