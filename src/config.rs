use crate::keymap::{Keymap, KeymapConfig};
use crate::plugins::PluginConfig;
use crate::theme::{ThemeConfig, ThemePalette};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use toml::Value;

const CURRENT_CONFIG_VERSION: u32 = 1;

#[derive(Debug, Clone, Default)]
pub struct AppConfig {
    pub theme: ThemePalette,
    pub keymap: Keymap,
    pub plugins: PluginConfig,
    pub editor: EditorConfig,
    pub load_warnings: Vec<String>,
}

/// Editor-level settings configurable via TOML.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    #[serde(default)]
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

impl AppConfig {
    pub fn load() -> Self {
        let mut config = Self::default();
        let mut warnings = Vec::new();

        if let Some(path) = user_config_path()
            && let Some(file) = load_file(&path, &mut warnings)
        {
            let profile = file.profile.clone();
            config.apply_file(&file, &mut warnings);
            if let Some(profile) = profile
                && let Some(profile_path) = profile_config_path(&profile)
            {
                if profile_path.exists() {
                    if let Some(profile_file) = load_file(&profile_path, &mut warnings) {
                        config.apply_file(&profile_file, &mut warnings);
                    }
                } else {
                    warnings.push(format!(
                        "Config: profil '{profile}' introuvable ({profile_path:?})."
                    ));
                }
            }
        }

        if let Some(workspace_path) = workspace_config_path()
            && let Some(file) = load_file(&workspace_path, &mut warnings)
        {
            let profile = file.profile.clone();
            config.apply_file(&file, &mut warnings);
            if let Some(profile) = profile
                && let Some(profile_path) = profile_config_path(&profile)
            {
                if profile_path.exists() {
                    if let Some(profile_file) = load_file(&profile_path, &mut warnings) {
                        config.apply_file(&profile_file, &mut warnings);
                    }
                } else {
                    warnings.push(format!(
                        "Config: profil '{profile}' introuvable ({profile_path:?})."
                    ));
                }
            }
        }

        config.load_warnings = warnings;
        config
    }

    fn apply_file(&mut self, file: &ConfigFile, warnings: &mut Vec<String>) {
        if let Some(theme) = &file.theme {
            warnings.extend(theme.apply_to(&mut self.theme));
        }
        if let Some(keymap) = &file.keymap {
            warnings.extend(self.keymap.apply_config(keymap));
        }
        if let Some(keymap) = &file.keybindings {
            warnings.push("Config: section 'keybindings' obsolète, utilisez 'keymap'.".to_string());
            warnings.extend(self.keymap.apply_config(keymap));
        }
        if let Some(profile_name) = file.keymap_profile.as_deref() {
            match file.keymap_profiles.as_ref() {
                Some(profiles) => match profiles.get(profile_name) {
                    Some(profile) => warnings.extend(self.keymap.apply_config(profile)),
                    None => warnings.push(format!(
                        "Config: profil keymap '{profile_name}' introuvable."
                    )),
                },
                None => warnings.push(format!(
                    "Config: keymap_profile '{profile_name}' défini sans keymap_profiles."
                )),
            }
        }
        if let Some(plugins) = &file.plugins {
            self.plugins = plugins.clone();
            warnings.extend(plugins.warnings());
        }
        if let Some(editor) = &file.editor {
            self.editor = editor.clone();
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct ConfigFile {
    pub config_version: Option<u32>,
    pub profile: Option<String>,
    pub theme: Option<ThemeConfig>,
    pub keymap: Option<KeymapConfig>,
    pub keybindings: Option<KeymapConfig>,
    pub keymap_profile: Option<String>,
    pub keymap_profiles: Option<HashMap<String, KeymapConfig>>,
    pub plugins: Option<PluginConfig>,
    pub editor: Option<EditorConfig>,
}

fn load_file(path: &Path, warnings: &mut Vec<String>) -> Option<ConfigFile> {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(err) => {
            if err.kind() != std::io::ErrorKind::NotFound {
                warnings.push(format!("Config: {path:?}: {err}"));
            }
            return None;
        }
    };

    let raw = match toml::from_str::<Value>(&contents) {
        Ok(raw) => raw,
        Err(err) => {
            warnings.push(format!("Config: {path:?}: {err}"));
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
            warnings.push(format!("Config: {path:?}: {err}"));
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

/// Returns all config file paths that should be watched for hot reload.
pub fn watch_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(p) = user_config_path() {
        paths.push(p);
    }
    if let Some(p) = workspace_config_path() {
        paths.push(p);
    }
    paths
}

fn user_config_path() -> Option<PathBuf> {
    let home = std::env::var("HOME").ok()?;
    Some(PathBuf::from(home).join(".config/roxanne/config.toml"))
}

fn profile_config_path(profile: &str) -> Option<PathBuf> {
    let home = std::env::var("HOME").ok()?;
    Some(
        PathBuf::from(home)
            .join(".config/roxanne/profiles")
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
