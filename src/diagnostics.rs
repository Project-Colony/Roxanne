use crate::editor::{Position, TextBuffer};

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub line: usize,
    pub column: usize,
    pub message: String,
    pub severity: DiagnosticSeverity,
}

#[derive(Debug, Clone, Copy)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
}

pub fn analyze(buffer: &TextBuffer) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut stack: Vec<(char, Position)> = Vec::new();
    let mut in_string = false;
    let mut in_char = false;
    let mut raw_string_hashes: Option<usize> = None;
    let mut block_comment_depth = 0usize;
    let mut escaped = false;
    let mut string_start: Option<Position> = None;
    let mut raw_string_start: Option<Position> = None;
    let mut char_start: Option<Position> = None;

    for (line_index, line) in buffer.lines().enumerate() {
        if in_string || in_char {
            escaped = false;
        }
        let mut chars = line.chars().enumerate().peekable();
        while let Some((column, ch)) = chars.next() {
            if block_comment_depth > 0 {
                if ch == '/' && matches!(chars.peek(), Some((_, '*'))) {
                    chars.next();
                    block_comment_depth += 1;
                } else if ch == '*' && matches!(chars.peek(), Some((_, '/'))) {
                    chars.next();
                    block_comment_depth = block_comment_depth.saturating_sub(1);
                }
                continue;
            }

            if let Some(hashes) = raw_string_hashes {
                if ch == '"' {
                    if hashes == 0 {
                        raw_string_hashes = None;
                        raw_string_start = None;
                    } else {
                        let mut lookahead = chars.clone();
                        let mut matched = 0;
                        while matched < hashes {
                            if matches!(lookahead.peek(), Some((_, '#'))) {
                                matched += 1;
                                lookahead.next();
                            } else {
                                break;
                            }
                        }
                        if matched == hashes {
                            for _ in 0..hashes {
                                chars.next();
                            }
                            raw_string_hashes = None;
                            raw_string_start = None;
                        }
                    }
                }
                continue;
            }

            if in_string {
                if escaped {
                    escaped = false;
                    continue;
                }
                match ch {
                    '\\' => {
                        escaped = true;
                    }
                    '"' => {
                        in_string = false;
                        string_start = None;
                    }
                    _ => {}
                }
                continue;
            }

            if in_char {
                if escaped {
                    escaped = false;
                    continue;
                }
                match ch {
                    '\\' => {
                        escaped = true;
                    }
                    '\'' => {
                        in_char = false;
                        char_start = None;
                    }
                    _ => {}
                }
                continue;
            }

            if ch == '/' && matches!(chars.peek(), Some((_, '*'))) {
                chars.next();
                block_comment_depth += 1;
                continue;
            }

            if ch == '/' && matches!(chars.peek(), Some((_, '/'))) {
                break;
            }

            match ch {
                'r' => {
                    let mut lookahead = chars.clone();
                    let mut hashes = 0;
                    let mut valid = false;
                    if let Some((_, next)) = lookahead.peek() {
                        if *next == '"' {
                            valid = true;
                        } else if *next == '#' {
                            while let Some((_, '#')) = lookahead.peek() {
                                hashes += 1;
                                lookahead.next();
                            }
                            if matches!(lookahead.peek(), Some((_, '"'))) {
                                valid = true;
                            }
                        }
                    }
                    if valid {
                        if hashes == 0 {
                            chars.next();
                        } else {
                            for _ in 0..hashes {
                                chars.next();
                            }
                            chars.next();
                        }
                        raw_string_hashes = Some(hashes);
                        raw_string_start = Some(Position::new(line_index, column));
                        continue;
                    }
                }
                '"' => {
                    in_string = true;
                    escaped = false;
                    string_start = Some(Position::new(line_index, column));
                }
                '\'' => {
                    in_char = true;
                    escaped = false;
                    char_start = Some(Position::new(line_index, column));
                }
                '{' | '(' | '[' => stack.push((ch, Position::new(line_index, column))),
                '}' | ')' | ']' => {
                    if let Some((open, position)) = stack.pop() {
                        if !matches!((open, ch), ('{', '}') | ('(', ')') | ('[', ']')) {
                            diagnostics.push(Diagnostic {
                                line: line_index,
                                column,
                                message: format!(
                                    "Fermeture inattendue '{ch}' (ouverture '{open}' ligne {}).",
                                    position.line + 1
                                ),
                                severity: DiagnosticSeverity::Error,
                            });
                        }
                    } else {
                        diagnostics.push(Diagnostic {
                            line: line_index,
                            column,
                            message: format!("Fermeture inattendue '{ch}'."),
                            severity: DiagnosticSeverity::Error,
                        });
                    }
                }
                _ => {}
            }
        }
    }

    if raw_string_hashes.is_some() {
        let position = raw_string_start.unwrap_or_else(|| Position::new(0, 0));
        diagnostics.push(Diagnostic {
            line: position.line,
            column: position.column,
            message: "Chaîne non terminée.".to_string(),
            severity: DiagnosticSeverity::Warning,
        });
    }

    if in_string {
        let position = string_start.unwrap_or_else(|| Position::new(0, 0));
        diagnostics.push(Diagnostic {
            line: position.line,
            column: position.column,
            message: "Chaîne non terminée.".to_string(),
            severity: DiagnosticSeverity::Warning,
        });
    }

    if in_char {
        let position = char_start.unwrap_or_else(|| Position::new(0, 0));
        diagnostics.push(Diagnostic {
            line: position.line,
            column: position.column,
            message: "Caractère non terminé.".to_string(),
            severity: DiagnosticSeverity::Warning,
        });
    }

    for (open, position) in stack {
        diagnostics.push(Diagnostic {
            line: position.line,
            column: position.column,
            message: format!("Ouverture '{open}' sans fermeture."),
            severity: DiagnosticSeverity::Warning,
        });
    }

    diagnostics
}

/// Merge local diagnostics with LSP diagnostics from an external source.
pub fn merge_with_lsp(
    local: Vec<Diagnostic>,
    lsp_diagnostics: &[crate::lsp::LspDiagnostic],
) -> Vec<Diagnostic> {
    let mut merged = local;
    for lsp_diag in lsp_diagnostics {
        let severity = match lsp_diag.severity {
            crate::lsp::LspDiagnosticSeverity::Error => DiagnosticSeverity::Error,
            crate::lsp::LspDiagnosticSeverity::Warning => DiagnosticSeverity::Warning,
            crate::lsp::LspDiagnosticSeverity::Info | crate::lsp::LspDiagnosticSeverity::Hint => {
                DiagnosticSeverity::Warning
            }
        };
        merged.push(Diagnostic {
            line: lsp_diag.line,
            column: lsp_diag.column,
            message: format!("[LSP] {}", lsp_diag.message),
            severity,
        });
    }
    merged.sort_by_key(|d| (d.line, d.column));
    merged
}
