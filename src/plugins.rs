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

const KNOWN_PLUGINS: &[&str] = &[
    "word_count",
    "line_count",
    "character_count",
    "byte_count",
    "longest_line",
];

impl std::fmt::Debug for PluginManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PluginManager")
            .field("plugins", &self.plugins.len())
            .finish()
    }
}

impl PluginManager {
    pub fn new(config: &PluginConfig) -> (Self, Vec<String>) {
        let mut manager = Self {
            plugins: Vec::new(),
        };
        let mut warnings = Vec::new();
        for plugin in &config.enabled {
            match plugin.as_str() {
                "word_count" => manager.register(Box::new(WordCountPlugin::default())),
                "line_count" => manager.register(Box::new(LineCountPlugin::default())),
                "character_count" => manager.register(Box::new(CharacterCountPlugin::default())),
                "byte_count" => manager.register(Box::new(ByteCountPlugin::default())),
                "longest_line" => manager.register(Box::new(LongestLinePlugin::default())),
                _ => {}
            }
        }
        for plugin_path in &config.dynamic {
            // A relative path would resolve against the working directory, so
            // opening a project could load a library from inside it.
            if !Path::new(plugin_path).is_absolute() {
                warnings.push(format!(
                    "Plugins: {plugin_path}: chemin absolu requis pour un plugin natif."
                ));
                continue;
            }
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
    vec![
        "word_count".to_string(),
        "line_count".to_string(),
        "character_count".to_string(),
        "byte_count".to_string(),
        "longest_line".to_string(),
    ]
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
        // On Windows, plain LoadLibrary looks for the plugin's own DLL
        // dependencies in the working directory (the opened project) and not
        // next to the plugin. Search the plugin's folder, the application folder
        // and System32 instead.
        #[cfg(windows)]
        let library = unsafe {
            use libloading::os::windows;
            windows::Library::load_with_flags(
                path,
                windows::LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR
                    | windows::LOAD_LIBRARY_SEARCH_DEFAULT_DIRS,
            )
        }
        .map(Library::from);
        #[cfg(not(windows))]
        let library = unsafe { Library::new(Path::new(path)) };
        let library = library.map_err(|err| format!("chargement impossible: {err}"))?;
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

    fn to_cstring_with_warning(&self, value: &str, label: &str) -> CString {
        match CString::new(value) {
            Ok(cstring) => cstring,
            Err(_) => {
                let sanitized = value.replace('\0', "�");
                eprintln!(
                    "Plugin {}: {label} contient des caractères NUL, remplacement avant callback.",
                    self.name
                );
                CString::new(sanitized).unwrap_or_else(|_| {
                    CString::new("�").expect("CString de secours valide pour plugin")
                })
            }
        }
    }
}

impl Plugin for DynamicPlugin {
    fn name(&self) -> &str {
        &self.name
    }

    fn on_file_opened(&mut self, text: &str, context: &PluginContext) {
        if let Some(callback) = self.api.on_file_opened {
            let text = self.to_cstring_with_warning(text, "texte");
            let filename =
                self.to_cstring_with_warning(context.filename.as_str(), "nom de fichier");
            unsafe {
                callback(text.as_ptr(), filename.as_ptr());
            }
        }
    }

    fn on_file_saved(&mut self, text: &str, context: &PluginContext) {
        if let Some(callback) = self.api.on_file_saved {
            let text = self.to_cstring_with_warning(text, "texte");
            let filename =
                self.to_cstring_with_warning(context.filename.as_str(), "nom de fichier");
            unsafe {
                callback(text.as_ptr(), filename.as_ptr());
            }
        }
    }

    fn on_text_changed(&mut self, text: &str, context: &PluginContext) {
        if let Some(callback) = self.api.on_text_changed {
            let text = self.to_cstring_with_warning(text, "texte");
            let filename =
                self.to_cstring_with_warning(context.filename.as_str(), "nom de fichier");
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

#[derive(Default)]
struct CharacterCountPlugin {
    characters: usize,
}

impl Plugin for CharacterCountPlugin {
    fn name(&self) -> &str {
        "character_count"
    }

    fn on_text_changed(&mut self, text: &str, _context: &PluginContext) {
        self.characters = text.chars().count();
    }

    fn status(&self) -> Option<PluginStatus> {
        Some(PluginStatus {
            label: "Caractères".to_string(),
            value: self.characters.to_string(),
        })
    }
}

#[derive(Default)]
struct ByteCountPlugin {
    bytes: usize,
}

impl Plugin for ByteCountPlugin {
    fn name(&self) -> &str {
        "byte_count"
    }

    fn on_text_changed(&mut self, text: &str, _context: &PluginContext) {
        self.bytes = text.len();
    }

    fn status(&self) -> Option<PluginStatus> {
        Some(PluginStatus {
            label: "Octets".to_string(),
            value: self.bytes.to_string(),
        })
    }
}

#[derive(Default)]
struct LongestLinePlugin {
    longest_line: usize,
    longest_line_index: usize,
}

impl Plugin for LongestLinePlugin {
    fn name(&self) -> &str {
        "longest_line"
    }

    fn on_text_changed(&mut self, text: &str, _context: &PluginContext) {
        let mut max_len = 0;
        let mut max_index = 0;
        for (index, line) in text.split('\n').enumerate() {
            let len = line.chars().count();
            if len > max_len {
                max_len = len;
                max_index = index;
            }
        }
        self.longest_line = max_len;
        self.longest_line_index = max_index;
    }

    fn status(&self) -> Option<PluginStatus> {
        Some(PluginStatus {
            label: "Ligne max".to_string(),
            value: format!("{} (L{})", self.longest_line, self.longest_line_index + 1),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{PluginConfig, PluginManager};

    #[test]
    fn plugin_manager_collects_statuses() {
        let config = PluginConfig {
            enabled: vec![
                "word_count".to_string(),
                "line_count".to_string(),
                "character_count".to_string(),
                "byte_count".to_string(),
                "longest_line".to_string(),
            ],
            dynamic: Vec::new(),
        };
        let (mut manager, warnings) = PluginManager::new(&config);
        assert!(warnings.is_empty());

        manager.on_text_changed("hi\nBonjour", "demo.txt");

        let statuses = manager
            .statuses()
            .into_iter()
            .map(|status| (status.label, status.value))
            .collect::<std::collections::HashMap<_, _>>();

        assert_eq!(statuses.get("Mots").map(String::as_str), Some("2"));
        assert_eq!(statuses.get("Lignes").map(String::as_str), Some("2"));
        assert_eq!(statuses.get("Caractères").map(String::as_str), Some("10"));
        assert_eq!(statuses.get("Octets").map(String::as_str), Some("10"));
        assert_eq!(
            statuses.get("Ligne max").map(String::as_str),
            Some("7 (L2)")
        );
    }

    #[test]
    fn plugin_manager_refuses_relative_native_plugin_paths() {
        let config = PluginConfig {
            enabled: Vec::new(),
            dynamic: vec!["target/release/libroxanne_sample.so".to_string()],
        };
        let (manager, warnings) = PluginManager::new(&config);
        assert!(manager.plugins.is_empty());
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("chemin absolu"));
    }

    #[test]
    fn plugin_config_warns_on_unknown_plugins() {
        let config = PluginConfig {
            enabled: vec!["word_count".to_string(), "mystery".to_string()],
            dynamic: Vec::new(),
        };

        let warnings = config.warnings();
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("mystery"));
    }
}
