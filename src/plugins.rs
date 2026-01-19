use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct PluginStatus {
    pub label: String,
    pub value: String,
}

pub struct PluginContext {
    pub filename: String,
}

pub trait Plugin {
    fn name(&self) -> &str;
    fn on_text_changed(&mut self, _text: &str, _context: &PluginContext) {}
    fn status(&self) -> Option<PluginStatus> {
        None
    }
}

pub struct PluginManager {
    plugins: Vec<Box<dyn Plugin>>,
}

const KNOWN_PLUGINS: &[&str] = &["word_count", "line_count"];

impl std::fmt::Debug for PluginManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PluginManager")
            .field("plugins", &self.plugins.len())
            .finish()
    }
}

impl PluginManager {
    pub fn new(config: &PluginConfig) -> Self {
        let mut manager = Self { plugins: Vec::new() };
        for plugin in &config.enabled {
            match plugin.as_str() {
                "word_count" => manager.register(Box::new(WordCountPlugin::default())),
                "line_count" => manager.register(Box::new(LineCountPlugin::default())),
                _ => {}
            }
        }
        manager
    }

    pub fn on_text_changed(&mut self, text: &str, filename: &str) {
        let context = PluginContext {
            filename: filename.to_string(),
        };
        for plugin in &mut self.plugins {
            plugin.on_text_changed(text, &context);
        }
    }

    pub fn statuses(&self) -> Vec<PluginStatus> {
        self.plugins
            .iter()
            .filter_map(|plugin| plugin.status())
            .collect()
    }

    fn register(&mut self, plugin: Box<dyn Plugin>) {
        self.plugins.push(plugin);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginConfig {
    #[serde(default = "default_plugins")]
    pub enabled: Vec<String>,
}

impl PluginConfig {
    pub fn warnings(&self) -> Vec<String> {
        self.enabled
            .iter()
            .filter(|plugin| !KNOWN_PLUGINS.contains(&plugin.as_str()))
            .map(|plugin| format!("Plugins: plugin inconnu '{plugin}'"))
            .collect()
    }
}

fn default_plugins() -> Vec<String> {
    vec!["word_count".to_string(), "line_count".to_string()]
}

#[derive(Default)]
struct WordCountPlugin {
    words: usize,
}

impl Plugin for WordCountPlugin {
    fn name(&self) -> &str {
        "word_count"
    }

    fn on_text_changed(&mut self, text: &str, _context: &PluginContext) {
        self.words = text.split_whitespace().count();
    }

    fn status(&self) -> Option<PluginStatus> {
        Some(PluginStatus {
            label: "Mots".to_string(),
            value: self.words.to_string(),
        })
    }
}

#[derive(Default)]
struct LineCountPlugin {
    lines: usize,
}

impl Plugin for LineCountPlugin {
    fn name(&self) -> &str {
        "line_count"
    }

    fn on_text_changed(&mut self, text: &str, _context: &PluginContext) {
        self.lines = text.lines().count().max(1);
    }

    fn status(&self) -> Option<PluginStatus> {
        Some(PluginStatus {
            label: "Lignes".to_string(),
            value: self.lines.to_string(),
        })
    }
}
