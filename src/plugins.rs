use libloading::Library;
use serde::{Deserialize, Serialize};
use std::ffi::{CStr, CString};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct PluginStatus {
    pub label: String,
    pub value: String,
}

pub struct PluginContext {
    pub filename: String,
}

pub trait Plugin {
    #[allow(dead_code)]
    fn name(&self) -> &str;
    fn on_file_opened(&mut self, _text: &str, _context: &PluginContext) {}
    fn on_file_saved(&mut self, _text: &str, _context: &PluginContext) {}
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
    pub fn new(config: &PluginConfig) -> (Self, Vec<String>) {
        let mut manager = Self { plugins: Vec::new() };
        let mut warnings = Vec::new();
        for plugin in &config.enabled {
            match plugin.as_str() {
                "word_count" => manager.register(Box::new(WordCountPlugin::default())),
                "line_count" => manager.register(Box::new(LineCountPlugin::default())),
                _ => {}
            }
        }
        for plugin_path in &config.dynamic {
            match DynamicPlugin::load(plugin_path) {
                Ok(plugin) => manager.register(Box::new(plugin)),
                Err(err) => warnings.push(format!("Plugins: {plugin_path}: {err}")),
            }
        }
        (manager, warnings)
    }

    pub fn on_text_changed(&mut self, text: &str, filename: &str) {
        let context = PluginContext {
            filename: filename.to_string(),
        };
        for plugin in &mut self.plugins {
            plugin.on_text_changed(text, &context);
        }
    }

    pub fn on_file_opened(&mut self, text: &str, filename: &str) {
        let context = PluginContext {
            filename: filename.to_string(),
        };
        for plugin in &mut self.plugins {
            plugin.on_file_opened(text, &context);
        }
    }

    pub fn on_file_saved(&mut self, text: &str, filename: &str) {
        let context = PluginContext {
            filename: filename.to_string(),
        };
        for plugin in &mut self.plugins {
            plugin.on_file_saved(text, &context);
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
    #[serde(default)]
    pub dynamic: Vec<String>,
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

#[repr(C)]
#[derive(Clone, Copy)]
struct PluginStatusV1 {
    label: *const std::ffi::c_char,
    value: *const std::ffi::c_char,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct PluginApiV1 {
    name: unsafe extern "C" fn() -> *const std::ffi::c_char,
    on_text_changed: Option<unsafe extern "C" fn(*const std::ffi::c_char, *const std::ffi::c_char)>,
    on_file_opened: Option<unsafe extern "C" fn(*const std::ffi::c_char, *const std::ffi::c_char)>,
    on_file_saved: Option<unsafe extern "C" fn(*const std::ffi::c_char, *const std::ffi::c_char)>,
    status: Option<unsafe extern "C" fn() -> PluginStatusV1>,
}

struct DynamicPlugin {
    _library: Library,
    api: PluginApiV1,
    #[allow(dead_code)]
    name: String,
}

impl DynamicPlugin {
    fn load(path: &str) -> Result<Self, String> {
        let library = unsafe { Library::new(Path::new(path)) }
            .map_err(|err| format!("chargement impossible: {err}"))?;
        let symbol: libloading::Symbol<unsafe extern "C" fn() -> PluginApiV1> = unsafe {
            library
                .get(b"roxanne_plugin_api_v1")
                .map_err(|err| format!("symbole manquant: {err}"))?
        };
        let api = unsafe { symbol() };
        let name = cstr_to_string(unsafe { (api.name)() })?;
        Ok(Self {
            _library: library,
            api,
            name,
        })
    }
}

impl Plugin for DynamicPlugin {
    fn name(&self) -> &str {
        &self.name
    }

    fn on_file_opened(&mut self, text: &str, context: &PluginContext) {
        if let Some(callback) = self.api.on_file_opened {
            let text = CString::new(text).unwrap_or_default();
            let filename = CString::new(context.filename.as_str()).unwrap_or_default();
            unsafe {
                callback(text.as_ptr(), filename.as_ptr());
            }
        }
    }

    fn on_file_saved(&mut self, text: &str, context: &PluginContext) {
        if let Some(callback) = self.api.on_file_saved {
            let text = CString::new(text).unwrap_or_default();
            let filename = CString::new(context.filename.as_str()).unwrap_or_default();
            unsafe {
                callback(text.as_ptr(), filename.as_ptr());
            }
        }
    }

    fn on_text_changed(&mut self, text: &str, context: &PluginContext) {
        if let Some(callback) = self.api.on_text_changed {
            let text = CString::new(text).unwrap_or_default();
            let filename = CString::new(context.filename.as_str()).unwrap_or_default();
            unsafe {
                callback(text.as_ptr(), filename.as_ptr());
            }
        }
    }

    fn status(&self) -> Option<PluginStatus> {
        let callback = self.api.status?;
        let status = unsafe { callback() };
        let label = cstr_to_string(status.label).ok()?;
        let value = cstr_to_string(status.value).ok()?;
        Some(PluginStatus { label, value })
    }
}

fn cstr_to_string(ptr: *const std::ffi::c_char) -> Result<String, String> {
    if ptr.is_null() {
        return Err("chaîne nulle".to_string());
    }
    let value = unsafe { CStr::from_ptr(ptr) };
    value
        .to_str()
        .map(|value| value.to_string())
        .map_err(|err| format!("chaîne invalide: {err}"))
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
