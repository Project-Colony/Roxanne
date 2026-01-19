use crate::keymap::{Keymap, KeymapConfig};
use crate::plugins::PluginConfig;
use crate::theme::{ThemeConfig, ThemePalette};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub theme: ThemePalette,
    pub keymap: Keymap,
    pub plugins: PluginConfig,
    pub load_warnings: Vec<String>,
}

impl AppConfig {
    pub fn load() -> Self {
        let mut config = Self::default();
        let mut warnings = Vec::new();

        if let Some(path) = user_config_path() {
            if let Some(file) = load_file(&path, &mut warnings) {
                let profile = file.profile.clone();
                config.apply_file(&file, &mut warnings);
                if let Some(profile) = profile {
                    if let Some(profile_path) = profile_config_path(&profile) {
                        if let Some(profile_file) = load_file(&profile_path, &mut warnings) {
                            config.apply_file(&profile_file, &mut warnings);
                        }
                    }
                }
            }
        }

        if let Ok(path) = std::env::current_dir() {
            let workspace_path = path.join(".roxanne.toml");
            if let Some(file) = load_file(&workspace_path, &mut warnings) {
                config.apply_file(&file, &mut warnings);
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
        if let Some(plugins) = &file.plugins {
            self.plugins = plugins.clone();
            warnings.extend(plugins.warnings());
        }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            theme: ThemePalette::default(),
            keymap: Keymap::default(),
            plugins: PluginConfig::default(),
            load_warnings: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct ConfigFile {
    pub profile: Option<String>,
    pub theme: Option<ThemeConfig>,
    pub keymap: Option<KeymapConfig>,
    pub plugins: Option<PluginConfig>,
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

    match toml::from_str::<ConfigFile>(&contents) {
        Ok(file) => Some(file),
        Err(err) => {
            warnings.push(format!("Config: {path:?}: {err}"));
            None
        }
    }
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
