//! Writing the settings the Preferences page changes into the user's files.
//!
//! Only the keys that changed are written, each into the user's file that
//! sets it (`config.toml` or the profile it selects), else into `config.toml`.
//! A key a project file sets is never written: the project's value would win
//! again at the next reload, and the user's own files must not pick it up.
//!
//! A file is edited, not regenerated: comments, key order and every key
//! Roxanne does not set stay as the user wrote them. A file that does not
//! parse is left alone rather than replaced. The write goes through
//! [`atomic_write`](crate::file_ops::atomic_write), so the file is never left
//! half written.

use super::AppConfig;
use crate::file_ops::atomic_write;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use toml_edit::{DocumentMut, Item, Table, TableLike, Value};

/// A dotted key (`language`, `editor.minimap`) and its new value; `None`
/// removes the key.
type Edit = (String, Option<toml::Value>);

/// Writes the settings of `config` that differ from `before`, which
/// [`AppConfig::page_settings`] took before the change. `user_config` is the
/// user's `config.toml`. Returns the project files that set a changed key:
/// those changes were not written.
pub fn save_changes(
    config: &AppConfig,
    before: &BTreeMap<String, toml::Value>,
    user_config: &Path,
) -> Result<Vec<PathBuf>, String> {
    let after = config.page_settings();
    let changed = after
        .iter()
        .filter(|(key, value)| before.get(*key) != Some(value))
        .map(|(key, value)| (key.clone(), Some(value.clone())))
        .chain(
            before
                .keys()
                .filter(|key| !after.contains_key(*key))
                .map(|key| (key.clone(), None)),
        );

    let mut files: BTreeMap<PathBuf, Vec<Edit>> = BTreeMap::new();
    let mut skipped = Vec::new();
    for (key, value) in changed {
        let target = match config.sources.get(&key) {
            Some(source) if source.project => {
                skipped.push(source.path.clone());
                continue;
            }
            Some(source) => source.path.clone(),
            None => user_config.to_path_buf(),
        };
        if value.is_none() && target != user_config {
            // Gone from the profile, the key must not come back from
            // config.toml.
            files
                .entry(user_config.to_path_buf())
                .or_default()
                .push((key.clone(), None));
        }
        files.entry(target).or_default().push((key, value));
    }
    for (path, edits) in files {
        save_to(&path, &edits)?;
    }
    skipped.dedup();
    Ok(skipped)
}

/// Applies `edits` to the config file at `path`, creating it if needed.
pub fn save_to(path: &Path, edits: &[Edit]) -> Result<(), String> {
    let failed = |err: &dyn std::fmt::Display| format!("{}: {err}", path.display());
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(err) => return Err(failed(&err)),
    };
    let mut document: DocumentMut = text.parse().map_err(|err| failed(&err))?;
    for (key, value) in edits {
        apply(&mut document, key, value.as_ref());
    }
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|err| failed(&err))?;
    }
    let path_text = path.to_str().ok_or_else(|| failed(&"not valid UTF-8"))?;
    atomic_write(path_text, &document.to_string())
}

fn apply(document: &mut DocumentMut, key: &str, value: Option<&toml::Value>) {
    let (table_name, field) = match key.split_once('.') {
        Some((table_name, field)) => (Some(table_name), field),
        None => (None, key),
    };
    match value.and_then(edit_value) {
        Some(value) => {
            let table = match table_name {
                Some(name) => table(document, name),
                None => document.as_table_mut(),
            };
            set(table, field, value);
        }
        None => {
            let table = match table_name {
                Some(name) => document.get_mut(name).and_then(Item::as_table_like_mut),
                None => Some(document.as_table_mut() as &mut dyn TableLike),
            };
            if let Some(table) = table {
                table.remove(field);
            }
        }
    }
}

/// The `[name]` table, replacing a value of another type there.
fn table<'d>(document: &'d mut DocumentMut, name: &str) -> &'d mut dyn TableLike {
    if !document.get(name).is_some_and(Item::is_table_like) {
        document.insert(name, Item::Table(Table::new()));
    }
    document[name]
        .as_table_like_mut()
        .expect("the item was just made a table")
}

/// Sets `key`, keeping the comments around an existing value.
fn set(table: &mut dyn TableLike, key: &str, value: Value) {
    match table.get_mut(key).and_then(Item::as_value_mut) {
        Some(existing) => {
            let decor = existing.decor().clone();
            *existing = value;
            *existing.decor_mut() = decor;
        }
        None => {
            table.insert(key, Item::Value(value));
        }
    }
}

fn edit_value(value: &toml::Value) -> Option<Value> {
    Some(match value {
        toml::Value::String(text) => text.as_str().into(),
        toml::Value::Integer(number) => (*number).into(),
        toml::Value::Float(number) => (*number).into(),
        toml::Value::Boolean(flag) => (*flag).into(),
        toml::Value::Array(items) => {
            Value::Array(items.iter().map(edit_value).collect::<Option<_>>()?)
        }
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::super::{Trust, load_file};
    use super::*;
    use colony_ui::preferences::{Change, StandardPreferences};
    use colony_ui::{FontSize, TextSize};

    /// The config `files` give, applied in order with their trust.
    fn load(files: &[(&Path, Trust)]) -> AppConfig {
        let mut config = AppConfig::default();
        let mut warnings = Vec::new();
        for (path, trust) in files {
            if let Some(file) = load_file(path, &mut warnings) {
                config.apply_file(path, &file, *trust, &mut warnings);
            }
        }
        assert!(warnings.is_empty(), "{warnings:?}");
        config
    }

    /// `StandardPreferences::update` without `apply`, which would change the
    /// globals other tests read.
    fn record(preferences: &mut StandardPreferences, change: Change) {
        match change {
            Change::Theme { family, variant } => {
                preferences.selected_theme = family.to_string();
                preferences.selected_variant = variant.to_string();
            }
            Change::Accent(key) => {
                preferences.selected_accent = Some(key.to_string());
                preferences.auto_accent = false;
            }
            Change::FontSize(size) => preferences.font_size = size,
            Change::TextSize(size) => preferences.text_size_a11y = size,
            Change::DyslexiaFont(on) => preferences.dyslexia_font = on,
            Change::ReduceMotion(on) => preferences.reduce_motion = on,
            _ => {}
        }
    }

    #[test]
    fn preferences_round_trip_and_only_changed_keys_are_written() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("config.toml");
        let mut config = AppConfig::default();

        let before = config.page_settings();
        record(&mut config.preferences, Change::FontSize(FontSize::Large));
        assert!(
            save_changes(&config, &before, &path)
                .expect("save")
                .is_empty()
        );
        assert_eq!(
            fs::read_to_string(&path).expect("read"),
            "[preferences]\nfont_size = \"large\"\n"
        );

        for change in [
            Change::Theme {
                family: "nord",
                variant: "light",
            },
            Change::Accent("violet"),
            Change::TextSize(TextSize::XLarge),
            Change::DyslexiaFont(true),
            Change::ReduceMotion(true),
        ] {
            let before = config.page_settings();
            record(&mut config.preferences, change);
            save_changes(&config, &before, &path).expect("save");
        }
        let before = config.page_settings();
        config.language = Some("fr".to_string());
        save_changes(&config, &before, &path).expect("save language");
        assert_eq!(load(&[(&path, Trust::User)]), config);

        // Back to the theme's own accent and the system language: the keys go.
        let before = config.page_settings();
        config.preferences.selected_accent = None;
        config.language = None;
        save_changes(&config, &before, &path).expect("save");
        let text = fs::read_to_string(&path).expect("read");
        assert!(!text.contains("selected_accent") && !text.contains("language"));
        assert_eq!(load(&[(&path, Trust::User)]), config);
    }

    #[test]
    fn a_change_goes_to_the_users_file_that_sets_it_and_never_to_a_project() {
        let dir = tempfile::tempdir().expect("tempdir");
        let user = dir.path().join("config.toml");
        let profile = dir.path().join("profiles/work.toml");
        let project = dir.path().join("project/.roxanne.toml");
        fs::create_dir_all(profile.parent().expect("dir")).expect("profiles");
        fs::create_dir_all(project.parent().expect("dir")).expect("project");
        fs::write(&user, "language = \"en\"\n").expect("user");
        let profile_text = "language = \"fr\"\n[preferences]\nselected_theme = \"nord\"\n";
        fs::write(&profile, profile_text).expect("profile");
        let project_text = "[editor]\ntab_size = 2\nword_wrap = true\n";
        fs::write(&project, project_text).expect("project");
        let files = [
            (user.as_path(), Trust::User),
            (profile.as_path(), Trust::User),
            (project.as_path(), Trust::Project),
        ];
        let mut config = load(&files);

        let before = config.page_settings();
        record(
            &mut config.preferences,
            Change::Theme {
                family: "gruvbox",
                variant: "light",
            },
        );
        config.editor.line_numbers = false;
        config.editor.word_wrap = false;
        let skipped = save_changes(&config, &before, &user).expect("save");

        assert_eq!(skipped, std::slice::from_ref(&project));
        assert_eq!(fs::read_to_string(&project).expect("project"), project_text);
        // The profile sets the theme family: the change lands there, the
        // variant it does not set goes to config.toml with line_numbers.
        assert!(
            fs::read_to_string(&profile)
                .expect("profile")
                .contains("selected_theme = \"gruvbox\"")
        );
        let user_text = fs::read_to_string(&user).expect("user");
        for written in ["selected_variant = \"light\"", "line_numbers = false"] {
            assert!(
                user_text.contains(written),
                "{written:?} not in:\n{user_text}"
            );
        }
        for kept_out in ["selected_theme", "word_wrap", "tab_size"] {
            assert!(
                !user_text.contains(kept_out),
                "{kept_out:?} in:\n{user_text}"
            );
        }

        // A reload keeps every saved change; the word wrap stays the project's.
        let reloaded = load(&files);
        assert_eq!(reloaded.preferences, config.preferences);
        assert!(!reloaded.editor.line_numbers);
        assert!(reloaded.editor.word_wrap);
        assert_eq!(reloaded.editor.tab_size, 2);
    }

    #[test]
    fn keeps_the_users_comments_and_unknown_keys() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("config.toml");
        let original = "# My settings, do not lose this.\n\
                        profile = \"work\"\n\
                        future_key = 42\n\
                        \n\
                        [editor]\n\
                        tab_size = 2 # two, like the team\n\
                        minimap = true # keep it\n";
        fs::write(&path, original).expect("write");

        let edits = [
            ("editor.minimap".to_string(), Some(false.into())),
            ("editor.line_numbers".to_string(), Some(false.into())),
            (
                "plugins.enabled".to_string(),
                Some(toml::Value::Array(vec!["word_count".into()])),
            ),
        ];
        save_to(&path, &edits).expect("save");

        let text = fs::read_to_string(&path).expect("read");
        for kept in [
            "# My settings, do not lose this.",
            "profile = \"work\"",
            "future_key = 42",
            "tab_size = 2 # two, like the team",
            "minimap = false # keep it",
            "line_numbers = false",
            "enabled = [\"word_count\"]",
        ] {
            assert!(text.contains(kept), "lost {kept:?} in:\n{text}");
        }
    }

    #[test]
    fn leaves_a_broken_file_alone() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("config.toml");
        fs::write(&path, "[editor\nbroken").expect("write");
        let edits = [("language".to_string(), Some("en".into()))];
        assert!(save_to(&path, &edits).is_err());
        assert_eq!(fs::read_to_string(&path).expect("read"), "[editor\nbroken");
    }
}
