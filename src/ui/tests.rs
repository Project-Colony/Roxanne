//! Behaviour of the editor that spans its parts: saving, reloading, routing
//! the results of background work to the right tab.

use super::file_tree::is_gitignored;
use super::menu::{MenuAction, fuzzy_match};
use super::panels::parse_goto_input;
use super::tabs::Tab;
use super::{FileLoadResult, Message, RoxanneApp, kept_edits_message};
use crate::config::AppConfig;
use crate::diagnostics;
use crate::editor::TextBuffer;
use crate::i18n::t;
use crate::search::{self, SearchOptions};
use iced::widget::text_editor::Content as EditorContent;
use std::fs;
use tempfile::tempdir;

#[test]
fn search_and_diagnostics_use_character_columns() {
    let text = "aé👍b\n🙂}";
    let buffer = TextBuffer::from(text);
    let options = SearchOptions {
        regex: false,
        case_sensitive: true,
        include_hidden: false,
    };

    let matches = search::find_matches(&buffer, "👍", options).expect("matches");
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].line, 0);
    assert_eq!(matches[0].column, 2);
    assert_eq!(matches[0].length, 1);

    let diagnostics = diagnostics::analyze(&buffer);
    let diagnostic = diagnostics
        .iter()
        .find(|item| item.severity == diagnostics::DiagnosticSeverity::Error)
        .expect("diagnostic");
    assert_eq!(diagnostic.line, 1);
    assert_eq!(diagnostic.column, 1);
}

#[test]
fn search_case_insensitive_preserves_original_columns_with_expanding_lowercase() {
    let text = "İa";
    let buffer = TextBuffer::from(text);
    let options = SearchOptions {
        regex: false,
        case_sensitive: false,
        include_hidden: false,
    };

    let matches = search::find_matches(&buffer, "a", options).expect("matches");
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].column, 1);
    assert_eq!(matches[0].length, 1);
}

#[test]
fn include_hidden_allows_hidden_files_in_filters() {
    let dir = tempdir().expect("tempdir");
    let hidden_path = dir.path().join(".secret.txt");
    fs::write(&hidden_path, "secret").expect("write hidden file");

    let hidden_entry = walkdir::WalkDir::new(dir.path())
        .min_depth(1)
        .max_depth(1)
        .into_iter()
        .filter_map(Result::ok)
        .find(|entry| entry.file_name() == ".secret.txt")
        .expect("hidden entry");

    assert!(search::should_skip_entry(&hidden_entry, false));
    assert!(!search::should_skip_entry(&hidden_entry, true));
    assert!(search::should_skip_file(&hidden_path, false));
    assert!(!search::should_skip_file(&hidden_path, true));
}

#[test]
fn diagnostics_ignore_escaped_quotes() {
    let text = r#"let value = "hello\"world";"#;
    let buffer = TextBuffer::from(text);

    let diagnostics = diagnostics::analyze(&buffer);
    assert!(
        diagnostics
            .iter()
            .all(|item| item.message != t("diag_unterminated_string")),
        "escaped quote should not trigger unterminated string diagnostic"
    );
}

#[test]
fn diagnostics_ignore_delimiters_in_strings_and_comments() {
    let text = r#"fn main() {
let value = "{[()]}"; // } ]) )
let url = "http://example.com";
}"#;
    let buffer = TextBuffer::from(text);

    let diagnostics = diagnostics::analyze(&buffer);
    assert!(
        diagnostics.is_empty(),
        "string/comment delimiters should be ignored: {diagnostics:?}"
    );
}

#[test]
fn diagnostics_ignore_comment_delimiters() {
    let text = "fn main() { // }\n    let value = \"{\";\n}";
    let buffer = TextBuffer::from(text);

    let diagnostics = diagnostics::analyze(&buffer);
    assert!(
        diagnostics.is_empty(),
        "comment delimiters should be ignored: {diagnostics:?}"
    );
}

#[test]
fn diagnostics_ignore_block_comment_delimiters() {
    let text = "fn main() {\n    /* { [ ( */\n    let value = 1;\n}\n";
    let buffer = TextBuffer::from(text);

    let diagnostics = diagnostics::analyze(&buffer);
    assert!(
        diagnostics.is_empty(),
        "block comment delimiters should be ignored: {diagnostics:?}"
    );
}

#[test]
fn diagnostics_ignore_nested_block_comment_delimiters() {
    let text = "fn main() {\n    /* outer { [ /* inner ( ) */ still ] } */\n}\n";
    let buffer = TextBuffer::from(text);

    let diagnostics = diagnostics::analyze(&buffer);
    assert!(
        diagnostics.is_empty(),
        "nested block comment delimiters should be ignored: {diagnostics:?}"
    );
}

#[test]
fn diagnostics_allow_multiline_strings() {
    let text = "fn main() {\n    let value = \"multi\nline\";\n}\n";
    let buffer = TextBuffer::from(text);

    let diagnostics = diagnostics::analyze(&buffer);
    assert!(
        diagnostics
            .iter()
            .all(|item| item.message != t("diag_unterminated_string")),
        "multiline strings should not trigger unterminated string diagnostic"
    );
}

#[test]
fn diagnostics_ignore_char_literals_with_braces() {
    let text = "fn main() { let left = '{'; let right = '}'; }";
    let buffer = TextBuffer::from(text);

    let diagnostics = diagnostics::analyze(&buffer);
    assert!(
        diagnostics.is_empty(),
        "char literal braces should be ignored: {diagnostics:?}"
    );
}

#[test]
fn diagnostics_ignore_raw_string_with_braces_and_quotes() {
    let text = r##"fn main() { let value = r#"{ "quoted" }"#; }"##;
    let buffer = TextBuffer::from(text);

    let diagnostics = diagnostics::analyze(&buffer);
    assert!(
        diagnostics.is_empty(),
        "raw string braces/quotes should be ignored: {diagnostics:?}"
    );
}

#[test]
fn fuzzy_match_basic() {
    assert!(fuzzy_match("open file", "opfl"));
    assert!(fuzzy_match("toggle file tree", "tft"));
    assert!(!fuzzy_match("save", "x"));
    assert!(fuzzy_match("save", ""));
    assert!(fuzzy_match("save", "s"));
    assert!(fuzzy_match("save", "save"));
    assert!(!fuzzy_match("", "a"));
}

#[test]
fn replace_with_captures_literal() {
    let result =
        search::replace_with_captures("hello world", "world", "rust", false, true).unwrap();
    assert_eq!(result, "hello rust");
}

#[test]
fn replace_with_captures_regex() {
    let result =
        search::replace_with_captures("foo123bar456", r"(\d+)", "[$1]", true, true).unwrap();
    assert_eq!(result, "foo[123]bar[456]");
}

#[test]
fn replace_with_captures_case_insensitive() {
    let result =
        search::replace_with_captures("Hello hello HELLO", "hello", "Hi", false, false).unwrap();
    assert_eq!(result, "Hi Hi Hi");
}

#[test]
fn buffer_word_completions() {
    let items =
        crate::completion::build_items_with_buffer("my", Some("let my_variable = myFunction();"));
    let labels: Vec<_> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(labels.contains(&"my_variable"));
    assert!(labels.contains(&"myFunction"));
}

#[test]
fn buffer_word_completions_no_short_words() {
    let items = crate::completion::build_items_with_buffer("a", Some("a ab abc"));
    // "a" and "ab" are too short (< 3 chars), only "abc" should appear
    let labels: Vec<_> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(labels.contains(&"abc"));
    assert!(!labels.contains(&"ab"));
}

#[test]
fn gitignore_patterns_basic() {
    assert!(is_gitignored("target", &["target".to_string()]));
    assert!(is_gitignored("foo.log", &["*.log".to_string()]));
    assert!(!is_gitignored("src", &["target".to_string()]));
}

#[test]
fn workspace_root_from_filename_uses_parent_for_missing_file() {
    let dir = tempdir().expect("tempdir");
    let root_marker = dir.path().join(".roxanne.toml");
    fs::write(&root_marker, "root = true").expect("write root marker");
    let nested = dir.path().join("nested");
    fs::create_dir_all(&nested).expect("create nested dir");
    let missing = nested.join("missing.txt");

    let resolved =
        RoxanneApp::workspace_root_from_filename(missing.to_str().expect("missing path"))
            .expect("workspace root");

    assert_eq!(resolved, dir.path());
}

fn app_with_tabs(tabs: Vec<Tab>, active_tab: usize) -> RoxanneApp {
    let mut app = RoxanneApp::new(AppConfig::default(), None).0;
    app.tabs = tabs;
    app.active_tab = active_tab;
    app
}

/// A tab loaded from `path` whose text was then changed to `edited`.
fn edited_tab(path: &str, edited: &str) -> Tab {
    let mut tab = Tab::new(path, "on disk");
    tab.disk_path = Some(path.to_string());
    tab.buffer.record_snapshot();
    tab.buffer.replace(edited);
    tab.content = EditorContent::with_text(edited);
    tab
}

fn saved_tab(path: &str) -> Tab {
    let mut tab = Tab::new(path, "");
    tab.disk_path = Some(path.to_string());
    tab
}

/// The read of `tab`'s file finishing with `text`.
fn loaded(tab: &Tab, text: &str) -> Message {
    Message::FileLoaded(Ok(FileLoadResult {
        tab_id: tab.id,
        path: tab.filename.clone(),
        text: text.to_string(),
        lossy: false,
    }))
}

fn absolute(path: &str) -> String {
    std::path::absolute(path)
        .expect("absolute path")
        .display()
        .to_string()
}

#[test]
fn reload_routing_finds_the_tab_by_its_file_path() {
    let app = app_with_tabs(
        vec![saved_tab("a.txt"), saved_tab("b.txt"), saved_tab("a.txt")],
        2,
    );
    assert_eq!(app.tab_index_for_disk_path("a.txt"), Some(2));
    // The watcher reports absolute paths.
    assert_eq!(app.tab_index_for_disk_path(&absolute("b.txt")), Some(1));
    assert_eq!(app.tab_index_for_disk_path("c.txt"), None);
    // A tab that was never loaded or saved has no file, whatever its name.
    let app = app_with_tabs(vec![Tab::new("b.txt", "")], 0);
    assert_eq!(app.tab_index_for_disk_path("b.txt"), None);
}

#[test]
fn a_save_lands_in_the_tab_that_was_written() {
    // Two new tabs with the same name; the second one became active
    // while the first one was being saved.
    let mut app = app_with_tabs(
        vec![
            Tab::new("notes.txt", "first"),
            Tab::new("notes.txt", "second"),
        ],
        1,
    );
    let written = app.tabs[0].content.text();

    let _ = app.update(Message::FileSaved(Ok((
        app.tabs[0].id,
        "notes.txt".to_string(),
        written,
    ))));

    assert_eq!(app.tabs[0].disk_path.as_deref(), Some("notes.txt"));
    assert_eq!(
        app.tabs[1].disk_path, None,
        "the other tab was marked saved"
    );
}

#[test]
fn a_loaded_file_goes_to_its_own_tab_not_the_active_one() {
    let mut app = app_with_tabs(
        vec![edited_tab("a.txt", "draft"), Tab::new("b.txt", "old")],
        0,
    );

    let _ = app.update(loaded(&app.tabs[1], "new"));

    assert_eq!(app.tabs[0].buffer.text(), "draft");
    assert!(app.tabs[0].is_modified());
    assert_eq!(app.tabs[1].buffer.text(), "new");
    assert!(!app.tabs[1].is_modified());
    assert_eq!(app.tabs[1].disk_path.as_deref(), Some("b.txt"));
}

#[test]
fn reloading_a_modified_tab_keeps_its_edits_and_undo_history() {
    let mut app = app_with_tabs(
        vec![Tab::new("a.txt", "a"), edited_tab("b.txt", "draft")],
        0,
    );

    let _ = app.update(Message::FileChangedExternally(absolute("b.txt")));
    let _ = app.update(Message::FileChangedReload);
    assert_eq!(
        app.status_message.as_deref(),
        Some(kept_edits_message(&absolute("b.txt")).as_str())
    );
    // A read that was already under way when the edits were made.
    let _ = app.update(loaded(&app.tabs[1], "changed on disk"));

    let tab = &mut app.tabs[1];
    assert_eq!(tab.buffer.text(), "draft");
    assert!(tab.is_modified());
    assert!(tab.buffer.undo(), "the undo history was cleared");
    assert_eq!(app.tabs[0].buffer.text(), "a");
}

#[test]
fn lsp_actions_skip_tabs_that_are_not_saved_rust_files() {
    let mut app = app_with_tabs(vec![Tab::new("untitled.txt", "text")], 0);

    let _ = app.update(Message::MenuAction(MenuAction::LspHover));

    assert!(
        app.lsp.is_none(),
        "started rust-analyzer for a non-Rust tab"
    );
    assert!(
        app.status_message
            .as_deref()
            .is_some_and(|m| m == t("status_lsp_rust_only"))
    );
}

#[test]
fn saving_a_new_tab_asks_for_a_file_name() {
    let mut app = app_with_tabs(vec![Tab::new("untitled.txt", "text")], 0);

    let _ = app.update(Message::MenuAction(MenuAction::Save));

    assert!(app.save_as_open);
    assert_eq!(app.save_as_input, "untitled.txt");
    assert_eq!(app.tab().disk_path, None);
}

#[test]
fn save_as_asks_before_replacing_an_existing_file() {
    let dir = tempdir().expect("tempdir");
    let taken = dir.path().join("taken.txt");
    fs::write(&taken, "keep").expect("write file");
    let taken = taken.to_str().expect("path").to_string();
    let mut app = app_with_tabs(vec![Tab::new("untitled.txt", "text")], 0);

    let _ = app.update(Message::MenuAction(MenuAction::Save));
    let _ = app.update(Message::SaveAsChanged(taken.clone()));
    let _ = app.update(Message::SaveAsSubmit);
    assert!(app.save_as_open, "replaced an existing file without asking");
    assert_eq!(app.tab().filename, "untitled.txt");

    let _ = app.update(Message::SaveAsSubmit);
    assert!(!app.save_as_open);
    assert_eq!(app.tab().filename, taken);
}

#[test]
fn save_as_asks_about_replaced_characters_before_renaming_the_tab() {
    let dir = tempdir().expect("tempdir");
    let target = dir.path().join("copy.txt").display().to_string();
    let mut tab = saved_tab("broken.txt");
    tab.file_is_lossy = true;
    let mut app = app_with_tabs(vec![tab], 0);

    let _ = app.open_save_as();
    let _ = app.update(Message::SaveAsChanged(target.clone()));
    let _ = app.update(Message::SaveAsSubmit);
    assert!(app.save_as_open, "the panel closed before the confirmation");
    assert_eq!(app.tab().filename, "broken.txt");

    let _ = app.update(Message::SaveAsSubmit);
    assert!(!app.save_as_open);
    assert_eq!(app.tab().filename, target);
    assert!(
        !app.tab().lossy_save_acknowledged,
        "the second submit did not write"
    );
}

#[test]
fn goto_input_counts_from_one() {
    assert_eq!(parse_goto_input("42"), Ok((41, 0)));
    assert_eq!(parse_goto_input("42:5"), Ok((41, 4)));
    assert_eq!(parse_goto_input("3,2"), Ok((2, 1)));
    assert!(parse_goto_input("0").is_err());
    assert!(parse_goto_input("4:0").is_err());
    assert!(parse_goto_input("1:2:3").is_err());
    assert!(parse_goto_input("x").is_err());
}

#[test]
fn the_cursor_moves_by_characters_on_lines_with_multibyte_text() {
    let mut app = app_with_tabs(vec![Tab::new("a.txt", "aé👍b\nzz")], 0);
    app.jump_to_position(0, 3);
    // The editor counts bytes: a (1) + é (2) + 👍 (4).
    assert_eq!(app.cursor_position(), (0, 7));
    app.jump_to_position(5, 99);
    assert_eq!(app.cursor_position(), (1, 2));
}

/// Colours come from the palette, fonts and sizes from the typography kit:
/// no widget writes its own, or the theme, the dyslexia font and the text
/// scale would miss it.
#[test]
fn widgets_take_colours_fonts_and_sizes_from_the_kit() {
    // Split, so this file does not match itself.
    let forbidden = [
        concat!("Color::from", "_rgb"),
        concat!("Color", "::new("),
        concat!("color", "!("),
        concat!("Font::", "MONOSPACE"),
        concat!("Font::", "with_name"),
    ];
    let raw_size = regex::Regex::new(r"\.(size|text_size)\(\s*[0-9]").expect("regex");
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    for entry in walkdir::WalkDir::new(src)
        .into_iter()
        .filter_map(Result::ok)
    {
        let path = entry.path();
        if path.extension().is_none_or(|ext| ext != "rs") {
            continue;
        }
        let text = fs::read_to_string(path).expect("source");
        for pattern in forbidden {
            assert!(!text.contains(pattern), "{}: {pattern}", path.display());
        }
        assert!(
            !raw_size.is_match(&text),
            "{}: a size without sz()",
            path.display()
        );
    }
}
