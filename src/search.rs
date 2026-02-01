use crate::editor::highlight::MatchPosition;
use crate::editor::TextBuffer;
use regex::{Regex, RegexBuilder};
use std::path::{Path, PathBuf};
use walkdir::{DirEntry, WalkDir};

use crate::file_ops::MAX_OPEN_FILE_SIZE;

#[derive(Debug, Clone, Copy)]
pub struct SearchOptions {
    pub regex: bool,
    pub case_sensitive: bool,
    pub include_hidden: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchScope {
    CurrentFile,
    Workspace,
}

impl SearchScope {
    pub fn label(self) -> &'static str {
        match self {
            SearchScope::CurrentFile => "Fichier",
            SearchScope::Workspace => "Workspace",
        }
    }
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub path: PathBuf,
    pub line: usize,
    pub column: usize,
    pub preview: String,
}

#[derive(Debug, Clone)]
pub struct SearchResultsSummary {
    pub results: Vec<SearchResult>,
    pub skipped_read_errors: usize,
    pub skipped_too_large: usize,
    pub skipped_invalid_utf8: usize,
    pub truncated: bool,
}

enum SearchMatcher {
    Plain {
        needle: String,
        case_sensitive: bool,
    },
    Regex(Regex),
}

struct SearchResultLine {
    line: usize,
    column: usize,
    _length: usize,
    preview: String,
}

pub fn find_matches(
    buffer: &TextBuffer,
    needle: &str,
    options: SearchOptions,
) -> Result<Vec<MatchPosition>, String> {
    if needle.trim().is_empty() {
        return Ok(Vec::new());
    }

    let matcher = build_matcher(needle, options)?;
    let mut matches = Vec::new();
    for (line_index, line) in buffer.lines().enumerate() {
        for (column, length) in find_matches_in_line(line, &matcher) {
            matches.push(MatchPosition {
                line: line_index,
                column,
                length,
            });
        }
    }

    Ok(matches)
}

pub async fn search_in_workspace(
    root: PathBuf,
    query: String,
    options: SearchOptions,
) -> Result<SearchResultsSummary, String> {
    if query.trim().is_empty() {
        return Ok(SearchResultsSummary {
            results: Vec::new(),
            skipped_read_errors: 0,
            skipped_too_large: 0,
            skipped_invalid_utf8: 0,
            truncated: false,
        });
    }

    let matcher = build_matcher(&query, options)?;
    let mut results = Vec::new();
    let mut collected = 0usize;
    let max_results = 500usize;
    let mut skipped_read_errors = 0usize;
    let mut skipped_too_large = 0usize;
    let mut skipped_invalid_utf8 = 0usize;
    let mut truncated = false;

    for entry in WalkDir::new(&root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| !should_skip_entry(entry, options.include_hidden))
    {
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => {
                skipped_read_errors += 1;
                continue;
            }
        };
        if !entry.file_type().is_file() {
            continue;
        }

        if should_skip_file(entry.path(), options.include_hidden) {
            continue;
        }

        let metadata = match entry.metadata() {
            Ok(metadata) => metadata,
            Err(_) => {
                skipped_read_errors += 1;
                continue;
            }
        };
        if metadata.len() > MAX_OPEN_FILE_SIZE {
            skipped_too_large += 1;
            continue;
        }

        let contents = match std::fs::read_to_string(entry.path()) {
            Ok(contents) => contents,
            Err(err) => {
                if err.kind() == std::io::ErrorKind::InvalidData {
                    skipped_invalid_utf8 += 1;
                } else {
                    skipped_read_errors += 1;
                }
                continue;
            }
        };

        for line_match in find_matches_in_text(&contents, &matcher) {
            results.push(SearchResult {
                path: entry.path().to_path_buf(),
                line: line_match.line,
                column: line_match.column,
                preview: line_match.preview,
            });
            collected += 1;
            if collected >= max_results {
                truncated = true;
                return Ok(SearchResultsSummary {
                    results,
                    skipped_read_errors,
                    skipped_too_large,
                    skipped_invalid_utf8,
                    truncated,
                });
            }
        }
    }

    Ok(SearchResultsSummary {
        results,
        skipped_read_errors,
        skipped_too_large,
        skipped_invalid_utf8,
        truncated,
    })
}

fn build_matcher(needle: &str, options: SearchOptions) -> Result<SearchMatcher, String> {
    if options.regex {
        RegexBuilder::new(needle)
            .case_insensitive(!options.case_sensitive)
            .build()
            .map(SearchMatcher::Regex)
            .map_err(|err| format!("regex invalide ({err})"))
    } else {
        Ok(SearchMatcher::Plain {
            needle: needle.to_string(),
            case_sensitive: options.case_sensitive,
        })
    }
}

fn normalize_casefolded(text: &str) -> String {
    text.chars().flat_map(|ch| ch.to_lowercase()).collect()
}

fn normalize_casefolded_with_mapping(text: &str) -> (String, Vec<usize>) {
    let mut normalized = String::new();
    let mut mapping = Vec::new();
    for (index, ch) in text.chars().enumerate() {
        for folded in ch.to_lowercase() {
            normalized.push(folded);
            mapping.push(index);
        }
    }
    (normalized, mapping)
}

fn find_matches_in_line(line: &str, matcher: &SearchMatcher) -> Vec<(usize, usize)> {
    match matcher {
        SearchMatcher::Regex(regex) => regex
            .find_iter(line)
            .map(|found| {
                let start = byte_index_to_char_index(line, found.start());
                let length = line[found.start()..found.end()].chars().count();
                (start, length)
            })
            .collect(),
        SearchMatcher::Plain {
            needle,
            case_sensitive,
        } => {
            if needle.is_empty() {
                return Vec::new();
            }

            if !case_sensitive {
                let normalized_needle = normalize_casefolded(needle);
                if normalized_needle.is_empty() {
                    return Vec::new();
                }
                let (normalized_line, mapping) = normalize_casefolded_with_mapping(line);
                let mut matches = Vec::new();
                let mut search_start = 0;
                while let Some(found) = normalized_line[search_start..].find(&normalized_needle) {
                    let start_byte = search_start + found;
                    let end_byte = start_byte + normalized_needle.len();
                    let start_index = byte_index_to_char_index(&normalized_line, start_byte);
                    let end_index = byte_index_to_char_index(&normalized_line, end_byte);
                    if end_index == 0 {
                        break;
                    }
                    let start_original = mapping.get(start_index).copied().unwrap_or_default();
                    let end_original = mapping
                        .get(end_index.saturating_sub(1))
                        .copied()
                        .map(|index| index + 1)
                        .unwrap_or(start_original);
                    let length = end_original.saturating_sub(start_original);
                    matches.push((start_original, length));
                    search_start = end_byte;
                }
                return matches;
            }

            let mut matches = Vec::new();
            let needle_len = needle.len();
            let needle_chars = needle.chars().count();
            let mut search_start = 0;
            while let Some(found) = line[search_start..].find(needle) {
                let byte_index = search_start + found;
                let column = byte_index_to_char_index(line, byte_index);
                matches.push((column, needle_chars));
                search_start = byte_index + needle_len;
            }

            matches
        }
    }
}

fn find_matches_in_text(text: &str, matcher: &SearchMatcher) -> Vec<SearchResultLine> {
    let mut matches = Vec::new();
    for (line_index, line) in text.lines().enumerate() {
        for (column, length) in find_matches_in_line(line, matcher) {
            matches.push(SearchResultLine {
                line: line_index,
                column,
                _length: length,
                preview: line.to_string(),
            });
        }
    }

    matches
}

pub fn should_skip_entry(entry: &DirEntry, include_hidden: bool) -> bool {
    if entry.depth() == 0 {
        return false;
    }
    let name = entry.file_name().to_string_lossy();
    let skip_dirs = [".git", "target", "node_modules", "dist", "build", "out"];
    if entry.file_type().is_dir() && skip_dirs.contains(&name.as_ref()) {
        return true;
    }
    if include_hidden {
        return false;
    }
    name.starts_with('.')
}

pub fn should_skip_file(path: &Path, include_hidden: bool) -> bool {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");
    if file_name.starts_with('.') && !include_hidden {
        return true;
    }

    let skip_extensions = [
        "png", "jpg", "jpeg", "gif", "svg", "ico", "zip", "tar", "gz", "pdf", "mp4", "mp3",
    ];
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| skip_extensions.contains(&ext))
        .unwrap_or(false)
}

fn byte_index_to_char_index(line: &str, byte_index: usize) -> usize {
    let mut index = byte_index.min(line.len());
    while index > 0 && !line.is_char_boundary(index) {
        index -= 1;
    }
    line[..index].chars().count()
}

/// Replace text using regex capture groups ($0, $1, $2, etc.).
/// If `is_regex` is false, performs a literal replacement.
pub fn replace_with_captures(
    text: &str,
    pattern: &str,
    replacement: &str,
    is_regex: bool,
    case_sensitive: bool,
) -> Result<String, String> {
    if !is_regex {
        // Literal replacement
        if case_sensitive {
            Ok(text.replace(pattern, replacement))
        } else {
            let re = RegexBuilder::new(&regex::escape(pattern))
                .case_insensitive(true)
                .build()
                .map_err(|e| format!("regex: {e}"))?;
            Ok(re.replace_all(text, replacement).to_string())
        }
    } else {
        let re = RegexBuilder::new(pattern)
            .case_insensitive(!case_sensitive)
            .build()
            .map_err(|e| format!("regex invalide: {e}"))?;
        // The regex crate supports $1, $2, ${name} in replacement strings
        Ok(re.replace_all(text, replacement).to_string())
    }
}
