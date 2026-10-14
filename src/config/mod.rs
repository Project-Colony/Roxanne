//! The configuration: the user's `config.toml`, the profile it selects and the
//! project's `.roxanne.toml`, applied in that order.
//!
//! Each file sets only the keys it names: a project's `[editor]` with
//! `tab_size = 2` changes the tab size and keeps the user's other editor
//! settings. [`AppConfig::sources`] remembers which file set each key.
//!
//! The user's files live in `Colony/Roxanne/` under the platform's config
//! directory (see [`user_config_dir`]). The Preferences page writes a change
//! back through [`store`] into the user's file that sets the key, else
//! `config.toml`, keeping comments and every other key.

mod migrate;
pub mod store;

use crate::i18n::{t, t_fmt};
use crate::keymap::{Keymap, KeymapConfig};
use crate::plugins::PluginConfig;
use colony_ui::preferences::StandardPreferences;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use toml::Value;

pub use migrate::{Migrated, migrate_legacy_config};

/// The program's name in `Colony/<Program>/`.
pub const PROGRAM: &str = "Roxanne";

const CURRENT_CONFIG_VERSION: u32 = 1;

#[derive(Debug, Clone, Default)]
pub struct AppConfig {
    /// Theme, accent, text sizes and accessibility, from the user's files only.
    pub preferences: StandardPreferences,
    /// `"en"` or `"fr"`; `None` follows the system language.
    pub language: Option<String>,
    pub keymap: Keymap,
    /// The keymap profile in use, from `keymap_profiles`.
    pub keymap_profile: Option<String>,
    /// The names of the keymap profiles the files define, sorted.
    pub keymap_profiles: Vec<String>,
    pub plugins: PluginConfig,
    pub editor: EditorConfig,
    pub lsp: LspConfig,
    pub load_warnings: Vec<String>,
    /// The file that set each key the Preferences page edits, by its dotted
    /// name (`language`, `editor.minimap`), when a file set it.
    pub sources: HashMap<String, Source>,
}

/// The config file a setting came from.
#[derive(Debug, Clone, PartialEq)]
pub struct Source {
    pub path: PathBuf,
    /// A project's `.roxanne.toml` or a profile it selects. The Preferences
    /// page never writes into a project's files.
    pub project: bool,
}

impl PartialEq for AppConfig {
    /// Equal when every setting is: the warnings and sources are not settings.
    fn eq(&self, other: &Self) -> bool {
        self.preferences == other.preferences
            && self.language == other.language
            && self.keymap == other.keymap
            && self.keymap_profile == other.keymap_profile
            && self.keymap_profiles == other.keymap_profiles
            && self.plugins == other.plugins
            && self.editor == other.editor
            && self.lsp == other.lsp
    }
}

/// Editor-level settings configurable via TOML.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EditorConfig {
    #[serde(default = "default_tab_size")]
    pub tab_size: usize,
    #[serde(default = "default_true")]
    pub use_spaces: bool,
    #[serde(default = "default_line_ending")]
    pub line_ending: String,
    #[serde(default)]
    pub word_wrap: bool,
    #[serde(default)]
    pub minimap: bool,
    #[serde(default = "default_true")]
    pub line_numbers: bool,
}

fn default_tab_size() -> usize {
    4
}
fn default_true() -> bool {
    true
}
fn default_line_ending() -> String {
    "lf".to_string()
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self {
            tab_size: 4,
            use_spaces: true,
            line_ending: "lf".to_string(),
            word_wrap: false,
            minimap: false,
            line_numbers: true,
        }
    }
}

/// The rust-analyzer client. Read from the user's files only: the command it
/// runs must never come from a project.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LspConfig {
    pub enabled: bool,
    /// The rust-analyzer program: a name looked up in `PATH`, or a path.
    pub rust_analyzer: String,
}

impl Default for LspConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            rust_analyzer: "rust-analyzer".to_string(),
        }
    }
}

impl AppConfig {
    pub fn load() -> Self {
        let mut config = Self::default();
        let mut warnings = Vec::new();

        if let Some(path) = user_config_path() {
            config.apply_layer(&path, Trust::User, &mut warnings);
        }
        if let Some(path) = workspace_config_path() {
            config.apply_layer(&path, Trust::Project, &mut warnings);
        }

        config.keymap_profiles.sort();
        config.keymap_profiles.dedup();
        config.load_warnings = warnings;
        config
    }

    /// The settings the Preferences page changes, by dotted key. A save
    /// compares them before and after a change and writes only what differs.
    pub fn page_settings(&self) -> BTreeMap<String, Value> {
        let mut settings = BTreeMap::new();
        for (name, table) in [
            ("preferences", toml::Table::try_from(&self.preferences)),
            ("editor", toml::Table::try_from(&self.editor)),
            ("lsp", toml::Table::try_from(&self.lsp)),
        ] {
            for (key, value) in table.unwrap_or_default() {
                settings.insert(format!("{name}.{key}"), value);
            }
        }
        if let Some(language) = &self.language {
            settings.insert("language".to_string(), language.as_str().into());
        }
        if let Some(profile) = &self.keymap_profile {
            settings.insert("keymap_profile".to_string(), profile.as_str().into());
        }
        let enabled = self.plugins.enabled.iter().map(|name| name.as_str().into());
        settings.insert(
            "plugins.enabled".to_string(),
            Value::Array(enabled.collect()),
        );
        settings
    }

    /// Applies a config file, then the profile it selects, if any.
    fn apply_layer(&mut self, path: &Path, trust: Trust, warnings: &mut Vec<String>) {
        let Some(file) = load_file(path, warnings) else {
            return;
        };
        self.apply_file(path, &file, trust, warnings);
        if let Some(profile) = &file.profile
            && let Some(profile_path) = profile_config_path(profile)
        {
            if profile_path.exists() {
                if let Some(profile_file) = load_file(&profile_path, warnings) {
                    self.apply_file(&profile_path, &profile_file, trust, warnings);
                }
            } else {
                warnings.push(t_fmt(
                    "config_warn_profile_missing",
                    &[("profile", profile), ("path", &profile_path.display())],
                ));
            }
        }
    }

    fn apply_file(
        &mut self,
        path: &Path,
        file: &ConfigFile,
        trust: Trust,
        warnings: &mut Vec<String>,
    ) {
        // The dotted keys this file sets, for `sources`.
        let mut set = Vec::new();
        let mut layer = Layer {
            path,
            set: &mut set,
            warnings,
        };
        match trust {
            Trust::User => {
                if let Some(theme) = &file.theme {
                    // Read before `[preferences]`, which wins key by key. Once
                    // the file has `[preferences]`, the old table is left
                    // alone without a word.
                    let mut theme_warnings = Vec::new();
                    if apply_legacy_theme(theme, &mut self.preferences, &mut theme_warnings) {
                        layer.set.push("preferences.selected_variant".to_string());
                    }
                    if file.preferences.is_none() {
                        layer.warnings.extend(theme_warnings);
                    }
                }
                if let Some(preferences) = &file.preferences {
                    layer.overlay(&mut self.preferences, "preferences", preferences);
                }
                if let Some(language) = &file.language {
                    self.language = Some(language.clone());
                    layer.set.push("language".to_string());
                }
                if let Some(lsp) = &file.lsp {
                    layer.overlay(&mut self.lsp, "lsp", lsp);
                }
            }
            Trust::Project => {
                if file.preferences.is_some() || file.theme.is_some() || file.language.is_some() {
                    layer
                        .warnings
                        .push(t("config_warn_project_appearance").to_string());
                }
                if file.lsp.is_some() {
                    layer
                        .warnings
                        .push(t("config_warn_project_lsp").to_string());
                }
            }
        }
        if let Some(keymap) = &file.keymap {
            layer.warnings.extend(self.keymap.apply_config(keymap));
        }
        if let Some(keymap) = &file.keybindings {
            layer
                .warnings
                .push(t("config_warn_keybindings").to_string());
            layer.warnings.extend(self.keymap.apply_config(keymap));
        }
        if let Some(profiles) = &file.keymap_profiles {
            self.keymap_profiles.extend(profiles.keys().cloned());
        }
        if let Some(profile_name) = file.keymap_profile.as_deref() {
            self.keymap_profile = Some(profile_name.to_string());
            layer.set.push("keymap_profile".to_string());
            match file.keymap_profiles.as_ref() {
                Some(profiles) => match profiles.get(profile_name) {
                    Some(profile) => layer.warnings.extend(self.keymap.apply_config(profile)),
                    None => layer.warnings.push(t_fmt(
                        "config_warn_keymap_profile_missing",
                        &[("profile", &profile_name)],
                    )),
                },
                None => layer.warnings.push(t_fmt(
                    "config_warn_keymap_profiles_missing",
                    &[("profile", &profile_name)],
                )),
            }
        }
        if let Some(plugins) = &file.plugins {
            let mut plugins = plugins.clone();
            if matches!(trust, Trust::Project)
                && plugins
                    .remove("dynamic")
                    .and_then(|dynamic| dynamic.as_array().map(|list| !list.is_empty()))
                    .unwrap_or(false)
            {
                layer
                    .warnings
                    .push(t("config_warn_project_dynamic").to_string());
            }
            if layer.overlay(&mut self.plugins, "plugins", &plugins)
                && plugins.contains_key("enabled")
            {
                layer.warnings.extend(self.plugins.warnings());
            }
        }
        if let Some(editor) = &file.editor {
            layer.overlay(&mut self.editor, "editor", editor);
        }

        let source = Source {
            path: path.to_path_buf(),
            project: matches!(trust, Trust::Project),
        };
        for key in set {
            self.sources.insert(key, source.clone());
        }
    }
}

/// The file being applied, what it set and what went wrong.
struct Layer<'a> {
    path: &'a Path,
    set: &'a mut Vec<String>,
    warnings: &'a mut Vec<String>,
}

impl Layer<'_> {
    /// Sets the keys `table` names on `value`, keeping the others, and notes
    /// them as `name.key`. A value of the wrong type leaves `value` as it was.
    fn overlay<T: Serialize + DeserializeOwned>(
        &mut self,
        value: &mut T,
        name: &str,
        table: &toml::Table,
    ) -> bool {
        let mut merged = toml::Table::try_from(&*value).unwrap_or_default();
        merged.extend(table.clone());
        match merged.try_into() {
            Ok(merged) => {
                *value = merged;
                self.set
                    .extend(table.keys().map(|key| format!("{name}.{key}")));
                true
            }
            Err(err) => {
                self.warnings.push(t_fmt(
                    "config_warn_file",
                    &[("path", &self.path.display()), ("error", &err)],
                ));
                false
            }
        }
    }
}

/// A `[theme]` table from before the Colony themes: `name = "light"` picks the
/// light variant of the default family, colour overrides are gone.
/// Returns whether it set the variant.
fn apply_legacy_theme(
    theme: &Value,
    preferences: &mut StandardPreferences,
    warnings: &mut Vec<String>,
) -> bool {
    if theme.get("palette").is_some() || theme.get("syntax").is_some() {
        warnings.push(t("config_warn_theme_colors").to_string());
    }
    match theme.get("name").and_then(Value::as_str) {
        Some(name) if name.eq_ignore_ascii_case("light") => {
            preferences.selected_variant = "light".to_string();
            true
        }
        Some(name) if name.eq_ignore_ascii_case("dark") || name.eq_ignore_ascii_case("default") => {
            false
        }
        Some(name) => {
            warnings.push(t_fmt("config_warn_theme_unknown", &[("name", &name)]));
            false
        }
        None => false,
    }
}

/// Who wrote a config file, which decides what it may set.
#[derive(Debug, Clone, Copy)]
enum Trust {
    /// The user's own config and the profiles it selects: may list
    /// `plugins.dynamic`, pick the rust-analyzer program and set the
    /// preferences.
    User,
    /// A `.roxanne.toml` found in the working directory or one of its
    /// ancestors, and any profile it selects. Opening a project must never run
    /// code from it, so its `plugins.dynamic` and `[lsp]` are ignored, and it
    /// does not change the user's theme or language.
    Project,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct ConfigFile {
    #[allow(dead_code)]
    config_version: Option<u32>,
    profile: Option<String>,
    /// Before the Colony themes. Read once to keep a light theme light.
    theme: Option<Value>,
    // Tables that set only the keys they name, see `Layer::overlay`.
    preferences: Option<toml::Table>,
    language: Option<String>,
    keymap: Option<KeymapConfig>,
    keybindings: Option<KeymapConfig>,
    keymap_profile: Option<String>,
    keymap_profiles: Option<HashMap<String, KeymapConfig>>,
    plugins: Option<toml::Table>,
    editor: Option<toml::Table>,
    lsp: Option<toml::Table>,
}

fn load_file(path: &Path, warnings: &mut Vec<String>) -> Option<ConfigFile> {
    let failed = |err: &dyn std::fmt::Display| {
        t_fmt(
            "config_warn_file",
            &[("path", &path.display()), ("error", err)],
        )
    };
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(err) => {
            if err.kind() != std::io::ErrorKind::NotFound {
                warnings.push(failed(&err));
            }
            return None;
        }
    };

    let raw = match toml::from_str::<Value>(&contents) {
        Ok(raw) => raw,
        Err(err) => {
            warnings.push(failed(&err));
            return None;
        }
    };

    let migrated = if needs_migration(&raw) {
        migrate_config(raw)
    } else {
        raw
    };

    match migrated.try_into::<ConfigFile>() {
        Ok(file) => Some(file),
        Err(err) => {
            warnings.push(failed(&err));
            None
        }
    }
}

fn needs_migration(raw: &Value) -> bool {
    let version = raw
        .get("config_version")
        .and_then(Value::as_integer)
        .and_then(|value| u32::try_from(value).ok());
    version.is_none_or(|value| value < CURRENT_CONFIG_VERSION)
}

pub fn migrate_config(mut raw: Value) -> Value {
    let Some(table) = raw.as_table_mut() else {
        return raw;
    };

    if let Some(keybindings) = table.remove("keybindings") {
        match table.get_mut("keymap") {
            Some(existing) => merge_tables(existing, keybindings),
            None => {
                table.insert("keymap".to_string(), keybindings);
            }
        }
    }

    table.insert(
        "config_version".to_string(),
        Value::Integer(CURRENT_CONFIG_VERSION.into()),
    );

    raw
}

fn merge_tables(target: &mut Value, source: Value) {
    let (Some(target_table), Some(source_table)) = (target.as_table_mut(), source.as_table())
    else {
        return;
    };

    for (key, value) in source_table {
        match target_table.get_mut(key) {
            Some(existing) => {
                if existing.is_table() && value.is_table() {
                    merge_tables(existing, value.clone());
                }
            }
            None => {
                target_table.insert(key.clone(), value.clone());
            }
        }
    }
}

/// The config files the hot reload watches, whether they exist yet or not:
/// the user config, every profile (`profiles/*.toml` stands for all of them)
/// and the workspace `.roxanne.toml`. With no workspace config yet, a new one
/// is looked for in the working directory, except at a root or in the home
/// folder: no project lives there, and on macOS a watch there would report
/// every change below it.
pub fn watch_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(dir) = user_config_dir() {
        paths.push(dir.join("config.toml"));
        paths.push(dir.join("profiles").join("*.toml"));
    }
    if let Some(path) = workspace_config_path() {
        paths.push(path);
    } else if let Ok(dir) = std::env::current_dir()
        && dir.parent().is_some()
        && std::env::var_os("HOME").is_none_or(|home| dir != Path::new(&home))
    {
        paths.push(dir.join(".roxanne.toml"));
    }
    paths
}

/// `Colony/Roxanne/` in the platform's config directory, without creating it:
/// `~/.config/Colony/Roxanne` on Linux, `%LOCALAPPDATA%\Colony\Roxanne` on
/// Windows, `~/Library/Application Support/Colony/Roxanne` on macOS.
pub fn user_config_dir() -> Option<PathBuf> {
    colony_ui::paths::locate::config_dir(PROGRAM).ok()
}

pub fn user_config_path() -> Option<PathBuf> {
    Some(user_config_dir()?.join("config.toml"))
}

fn profile_config_path(profile: &str) -> Option<PathBuf> {
    Some(
        user_config_dir()?
            .join("profiles")
            .join(format!("{profile}.toml")),
    )
}

fn workspace_config_path() -> Option<PathBuf> {
    let current = std::env::current_dir().ok()?;
    for dir in current.ancestors() {
        let candidate = dir.join(".roxanne.toml");
        if candidate.exists() {
            return Some(candidate);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(text: &str) -> ConfigFile {
        toml::from_str::<Value>(text)
            .expect("toml")
            .try_into()
            .expect("config file")
    }

    #[test]
    fn a_legacy_light_theme_stays_light_and_colours_are_reported() {
        let mut config = AppConfig::default();
        let mut warnings = Vec::new();
        let file = parse("[theme]\nname = \"light\"\n[theme.palette]\nmenu_bar = \"#000000\"\n");
        config.apply_file(Path::new("config.toml"), &file, Trust::User, &mut warnings);
        assert_eq!(config.preferences.selected_variant, "light");
        assert_eq!(warnings.len(), 1, "{warnings:?}");
    }

    #[test]
    fn a_project_never_sets_the_theme_language_or_language_server() {
        let mut config = AppConfig::default();
        let mut warnings = Vec::new();
        let file = parse(
            "language = \"fr\"\n[preferences]\nselected_theme = \"nord\"\n\
             [lsp]\nrust_analyzer = \"./evil\"\n",
        );
        config.apply_file(
            Path::new(".roxanne.toml"),
            &file,
            Trust::Project,
            &mut warnings,
        );
        assert_eq!(config, AppConfig::default());
        assert!(config.sources.is_empty());
        assert_eq!(warnings.len(), 2, "{warnings:?}");

        config.apply_file(Path::new("config.toml"), &file, Trust::User, &mut warnings);
        assert_eq!(config.language.as_deref(), Some("fr"));
        assert_eq!(config.preferences.selected_theme, "nord");
        assert_eq!(config.lsp.rust_analyzer, "./evil");
    }

    #[test]
    fn an_unknown_theme_falls_back_to_the_default_palette() {
        let file = parse("[preferences]\nselected_theme = \"no-such-family\"\n");
        let preferences: StandardPreferences = file
            .preferences
            .expect("preferences")
            .try_into()
            .expect("lenient");
        assert_eq!(
            colony_ui::resolve(&preferences.selected_theme, &preferences.selected_variant),
            colony_ui::FALLBACK_PALETTE
        );
    }

    #[test]
    fn each_file_sets_only_the_keys_it_names() {
        let mut config = AppConfig::default();
        let mut warnings = Vec::new();
        let user = parse(
            "[editor]\nminimap = true\n[preferences]\nfont_size = \"large\"\n\
             [theme]\nname = \"light\"\n[plugins]\nenabled = [\"word_count\"]\n",
        );
        config.apply_file(Path::new("config.toml"), &user, Trust::User, &mut warnings);
        let project = parse("[editor]\ntab_size = 2\n[plugins]\ndynamic = [\"/tmp/evil.so\"]\n");
        config.apply_file(
            Path::new(".roxanne.toml"),
            &project,
            Trust::Project,
            &mut warnings,
        );
        assert_eq!(warnings.len(), 1, "{warnings:?}");

        assert_eq!(config.editor.tab_size, 2);
        assert!(
            config.editor.minimap,
            "the project reset the user's minimap"
        );
        // A `[preferences]` table keeps the variant of the old `[theme]`.
        assert_eq!(config.preferences.selected_variant, "light");
        assert_eq!(config.preferences.font_size, colony_ui::FontSize::Large);
        assert_eq!(config.plugins.enabled, ["word_count"]);
        assert!(config.plugins.dynamic.is_empty());

        let source = |key: &str| config.sources.get(key).map(|source| source.project);
        assert_eq!(source("editor.tab_size"), Some(true));
        assert_eq!(source("editor.minimap"), Some(false));
        assert_eq!(source("preferences.selected_variant"), Some(false));
        assert_eq!(source("plugins.enabled"), Some(false));
        assert_eq!(source("plugins.dynamic"), None);
        assert_eq!(source("editor.word_wrap"), None);
    }

    #[test]
    fn a_value_of_the_wrong_type_keeps_the_previous_one() {
        let mut config = AppConfig::default();
        let mut warnings = Vec::new();
        let file = parse("[editor]\ntab_size = \"two\"\n");
        config.apply_file(Path::new("config.toml"), &file, Trust::User, &mut warnings);
        assert_eq!(config.editor, EditorConfig::default());
        assert!(config.sources.is_empty());
        assert_eq!(warnings.len(), 1, "{warnings:?}");
    }
}
