//! The configuration: the user's `config.toml`, the profile it selects and the
//! project's `.roxanne.toml`, applied in that order.
//!
//! The user's files live in `Colony/Roxanne/` under the platform's config
//! directory (see [`user_config_dir`]). The Preferences page writes the user's
//! `config.toml` back through [`store`], keeping its comments and every key it
//! does not set.

mod migrate;
pub mod store;

use crate::i18n::{t, t_fmt};
use crate::keymap::{Keymap, KeymapConfig};
use crate::plugins::PluginConfig;
use colony_ui::preferences::StandardPreferences;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use toml::Value;

pub use migrate::migrate_legacy_config;

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
}

impl PartialEq for AppConfig {
    /// Equal when every setting is: the warnings are not settings.
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

    /// Applies a config file, then the profile it selects, if any.
    fn apply_layer(&mut self, path: &Path, trust: Trust, warnings: &mut Vec<String>) {
        let Some(file) = load_file(path, warnings) else {
            return;
        };
        self.apply_file(&file, trust, warnings);
        if let Some(profile) = &file.profile
            && let Some(profile_path) = profile_config_path(profile)
        {
            if profile_path.exists() {
                if let Some(profile_file) = load_file(&profile_path, warnings) {
                    self.apply_file(&profile_file, trust, warnings);
                }
            } else {
                warnings.push(t_fmt(
                    "config_warn_profile_missing",
                    &[("profile", profile), ("path", &profile_path.display())],
                ));
            }
        }
    }

    fn apply_file(&mut self, file: &ConfigFile, trust: Trust, warnings: &mut Vec<String>) {
        match trust {
            Trust::User => {
                if let Some(preferences) = &file.preferences {
                    self.preferences = preferences.clone();
                } else if let Some(theme) = &file.theme {
                    apply_legacy_theme(theme, &mut self.preferences, warnings);
                }
                if file.language.is_some() {
                    self.language = file.language.clone();
                }
                if let Some(lsp) = &file.lsp {
                    self.lsp = lsp.clone();
                }
            }
            Trust::Project => {
                if file.preferences.is_some() || file.theme.is_some() || file.language.is_some() {
                    warnings.push(t("config_warn_project_appearance").to_string());
                }
                if file.lsp.is_some() {
                    warnings.push(t("config_warn_project_lsp").to_string());
                }
            }
        }
        if let Some(keymap) = &file.keymap {
            warnings.extend(self.keymap.apply_config(keymap));
        }
        if let Some(keymap) = &file.keybindings {
            warnings.push(t("config_warn_keybindings").to_string());
            warnings.extend(self.keymap.apply_config(keymap));
        }
        if let Some(profiles) = &file.keymap_profiles {
            self.keymap_profiles.extend(profiles.keys().cloned());
        }
        if let Some(profile_name) = file.keymap_profile.as_deref() {
            self.keymap_profile = Some(profile_name.to_string());
            match file.keymap_profiles.as_ref() {
                Some(profiles) => match profiles.get(profile_name) {
                    Some(profile) => warnings.extend(self.keymap.apply_config(profile)),
                    None => warnings.push(t_fmt(
                        "config_warn_keymap_profile_missing",
                        &[("profile", &profile_name)],
                    )),
                },
                None => warnings.push(t_fmt(
                    "config_warn_keymap_profiles_missing",
                    &[("profile", &profile_name)],
                )),
            }
        }
        if let Some(plugins) = &file.plugins {
            warnings.extend(plugins.warnings());
            match trust {
                Trust::User => self.plugins = plugins.clone(),
                Trust::Project => {
                    self.plugins.enabled = plugins.enabled.clone();
                    if !plugins.dynamic.is_empty() {
                        warnings.push(t("config_warn_project_dynamic").to_string());
                    }
                }
            }
        }
        if let Some(editor) = &file.editor {
            self.editor = editor.clone();
        }
    }
}

/// A `[theme]` table from before the Colony themes: `name = "light"` picks the
/// light variant of the default family, colour overrides are gone.
fn apply_legacy_theme(
    theme: &Value,
    preferences: &mut StandardPreferences,
    warnings: &mut Vec<String>,
) {
    match theme.get("name").and_then(Value::as_str) {
        Some(name) if name.eq_ignore_ascii_case("light") => {
            preferences.selected_variant = "light".to_string();
        }
        Some(name) if name.eq_ignore_ascii_case("dark") || name.eq_ignore_ascii_case("default") => {
        }
        Some(name) => warnings.push(t_fmt("config_warn_theme_unknown", &[("name", &name)])),
        None => {}
    }
    if theme.get("palette").is_some() || theme.get("syntax").is_some() {
        warnings.push(t("config_warn_theme_colors").to_string());
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
    preferences: Option<StandardPreferences>,
    language: Option<String>,
    keymap: Option<KeymapConfig>,
    keybindings: Option<KeymapConfig>,
    keymap_profile: Option<String>,
    keymap_profiles: Option<HashMap<String, KeymapConfig>>,
    plugins: Option<PluginConfig>,
    editor: Option<EditorConfig>,
    lsp: Option<LspConfig>,
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
        config.apply_file(&file, Trust::User, &mut warnings);
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
        config.apply_file(&file, Trust::Project, &mut warnings);
        assert_eq!(config, AppConfig::default());
        assert_eq!(warnings.len(), 2, "{warnings:?}");

        config.apply_file(&file, Trust::User, &mut warnings);
        assert_eq!(config.language.as_deref(), Some("fr"));
        assert_eq!(config.preferences.selected_theme, "nord");
        assert_eq!(config.lsp.rust_analyzer, "./evil");
    }

    #[test]
    fn an_unknown_theme_falls_back_to_the_default_palette() {
        let file = parse("[preferences]\nselected_theme = \"no-such-family\"\n");
        let preferences = file.preferences.expect("preferences");
        assert_eq!(
            colony_ui::resolve(&preferences.selected_theme, &preferences.selected_variant),
            colony_ui::FALLBACK_PALETTE
        );
    }
}
