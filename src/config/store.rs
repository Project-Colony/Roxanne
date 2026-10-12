//! Writing the settings the Preferences page changes into the user's
//! `config.toml`.
//!
//! The file is edited, not regenerated: comments, key order and every key
//! Roxanne does not set stay as the user wrote them. A file that does not
//! parse is left alone rather than replaced. The write goes through
//! [`atomic_write`](crate::file_ops::atomic_write), so the file is never left
//! half written.

use super::{EditorConfig, LspConfig, PROGRAM};
use crate::file_ops::atomic_write;
use colony_ui::preferences::StandardPreferences;
use std::fs;
use std::path::Path;
use toml_edit::{Array, DocumentMut, Item, Table, TableLike, Value};

/// A setting the Preferences page changed, with its new value.
#[derive(Debug, Clone, Copy)]
pub enum Setting<'a> {
    /// The whole `[preferences]` table: theme, accent, sizes, accessibility.
    Preferences(&'a StandardPreferences),
    /// `language`; `None` follows the system.
    Language(Option<&'a str>),
    /// The editor options the page shows: line numbers, wrapping, minimap.
    Editor(&'a EditorConfig),
    /// `keymap_profile`; `None` uses the keymap without a profile.
    KeymapProfile(Option<&'a str>),
    /// The `[lsp]` table.
    Lsp(&'a LspConfig),
    /// `plugins.enabled`, the built-in plugins that run.
    PluginsEnabled(&'a [String]),
}

/// Saves `setting` into `Colony/Roxanne/config.toml`, creating it if needed.
pub fn save(setting: Setting<'_>) -> Result<(), String> {
    let dir = colony_ui::paths::config_dir(PROGRAM).map_err(|err| err.to_string())?;
    save_to(&dir.join("config.toml"), setting)
}

/// Saves `setting` into the config file at `path`.
pub fn save_to(path: &Path, setting: Setting<'_>) -> Result<(), String> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(err) => return Err(format!("{}: {err}", path.display())),
    };
    let mut document: DocumentMut = text
        .parse()
        .map_err(|err| format!("{}: {err}", path.display()))?;
    apply(&mut document, setting);
    let path = path
        .to_str()
        .ok_or_else(|| format!("{}: not valid UTF-8", path.display()))?;
    atomic_write(path, &document.to_string())
}

fn apply(document: &mut DocumentMut, setting: Setting<'_>) {
    match setting {
        Setting::Preferences(preferences) => {
            let table = table(document, "preferences");
            let Ok(toml::Value::Table(values)) = toml::Value::try_from(preferences) else {
                return;
            };
            // The only field that serializes to nothing: back to the theme's
            // own accent.
            if preferences.selected_accent.is_none() {
                table.remove("selected_accent");
            }
            for (key, value) in values {
                if let Some(value) = edit_value(&value) {
                    set(table, &key, value);
                }
            }
        }
        Setting::Language(language) => set_or_remove(document.as_table_mut(), "language", language),
        Setting::Editor(editor) => {
            let table = table(document, "editor");
            set(table, "line_numbers", editor.line_numbers.into());
            set(table, "word_wrap", editor.word_wrap.into());
            set(table, "minimap", editor.minimap.into());
        }
        Setting::KeymapProfile(profile) => {
            set_or_remove(document.as_table_mut(), "keymap_profile", profile)
        }
        Setting::Lsp(lsp) => {
            let table = table(document, "lsp");
            set(table, "enabled", lsp.enabled.into());
            set(table, "rust_analyzer", lsp.rust_analyzer.as_str().into());
        }
        Setting::PluginsEnabled(enabled) => {
            let table = table(document, "plugins");
            set(
                table,
                "enabled",
                Value::Array(enabled.iter().map(String::as_str).collect::<Array>()),
            );
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

fn set_or_remove(table: &mut dyn TableLike, key: &str, value: Option<&str>) {
    match value {
        Some(value) => set(table, key, value.into()),
        None => {
            table.remove(key);
        }
    }
}

fn edit_value(value: &toml::Value) -> Option<Value> {
    Some(match value {
        toml::Value::String(text) => text.as_str().into(),
        toml::Value::Integer(number) => (*number).into(),
        toml::Value::Float(number) => (*number).into(),
        toml::Value::Boolean(flag) => (*flag).into(),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use colony_ui::preferences::Change;
    use colony_ui::{FontSize, TextSize};

    fn load(path: &Path) -> super::super::ConfigFile {
        let mut warnings = Vec::new();
        let file = super::super::load_file(path, &mut warnings).expect("config loads");
        assert!(warnings.is_empty(), "{warnings:?}");
        file
    }

    #[test]
    fn preferences_round_trip() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("config.toml");
        let mut preferences = StandardPreferences::default();
        for change in [
            Change::Theme {
                family: "nord",
                variant: "light",
            },
            Change::Accent("violet"),
            Change::FontSize(FontSize::Large),
            Change::TextSize(TextSize::XLarge),
            Change::DyslexiaFont(true),
            Change::ReduceMotion(true),
        ] {
            // Only the record is wanted here, not the process-wide effect.
            preferences = apply_change(preferences, change);
            save_to(&path, Setting::Preferences(&preferences)).expect("save");
        }
        save_to(&path, Setting::Language(Some("fr"))).expect("save language");

        let file = load(&path);
        assert_eq!(file.preferences.as_ref(), Some(&preferences));
        assert_eq!(file.language.as_deref(), Some("fr"));

        // Back to the theme's own accent: the key goes away.
        preferences.selected_accent = None;
        save_to(&path, Setting::Preferences(&preferences)).expect("save");
        assert!(
            !fs::read_to_string(&path)
                .expect("read")
                .contains("selected_accent")
        );
        assert_eq!(load(&path).preferences, Some(preferences));
    }

    /// `StandardPreferences::update` without `apply`, which would change the
    /// globals other tests read.
    fn apply_change(mut preferences: StandardPreferences, change: Change) -> StandardPreferences {
        match change {
            Change::Theme { family, variant } => {
                preferences.selected_theme = family.to_string();
                preferences.selected_variant = variant.to_string();
            }
            Change::Accent(key) => preferences.selected_accent = Some(key.to_string()),
            Change::FontSize(size) => preferences.font_size = size,
            Change::TextSize(size) => preferences.text_size_a11y = size,
            Change::DyslexiaFont(on) => preferences.dyslexia_font = on,
            Change::ReduceMotion(on) => preferences.reduce_motion = on,
            _ => {}
        }
        preferences
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

        let editor = EditorConfig {
            minimap: false,
            line_numbers: false,
            ..EditorConfig::default()
        };
        save_to(&path, Setting::Editor(&editor)).expect("save editor");
        save_to(&path, Setting::PluginsEnabled(&["word_count".to_string()])).expect("save");

        let text = fs::read_to_string(&path).expect("read");
        for kept in [
            "# My settings, do not lose this.",
            "profile = \"work\"",
            "future_key = 42",
            "tab_size = 2 # two, like the team",
            "minimap = false # keep it",
            "line_numbers = false",
        ] {
            assert!(text.contains(kept), "lost {kept:?} in:\n{text}");
        }
        let file = load(&path);
        assert_eq!(file.editor.expect("editor").tab_size, 2);
        assert_eq!(file.plugins.expect("plugins").enabled, ["word_count"]);
    }

    #[test]
    fn leaves_a_broken_file_alone() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("config.toml");
        fs::write(&path, "[editor\nbroken").expect("write");
        assert!(save_to(&path, Setting::Language(Some("en"))).is_err());
        assert_eq!(fs::read_to_string(&path).expect("read"), "[editor\nbroken");
    }
}
