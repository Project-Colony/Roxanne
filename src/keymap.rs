use iced::keyboard;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyAction {
    Save,
    Open,
    Find,
    FindNext,
    FindPrevious,
    Completion,
    CompletionClose,
    EnterInsertMode,
    EnterNormalMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeymapMode {
    Insert,
    Normal,
}

impl KeymapMode {
    pub fn label(self) -> &'static str {
        match self {
            KeymapMode::Insert => "Insert",
            KeymapMode::Normal => "Normal",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Keymap {
    insert: Vec<KeyBinding>,
    normal: Vec<KeyBinding>,
}

impl Keymap {
    pub fn default() -> Self {
        let mut insert = Vec::new();
        let mut normal = Vec::new();

        let default_bindings = [
            (KeyAction::Save, "cmd+s"),
            (KeyAction::Open, "cmd+o"),
            (KeyAction::Find, "cmd+f"),
            (KeyAction::FindNext, "f3"),
            (KeyAction::FindPrevious, "shift+f3"),
        ];

        for (action, combo) in default_bindings {
            let binding = KeyBinding::new(action, KeyCombo::parse(combo).unwrap());
            insert.push(binding);
            normal.push(binding);
        }

        insert.push(KeyBinding::new(
            KeyAction::Completion,
            KeyCombo::parse("ctrl+space").unwrap(),
        ));
        insert.push(KeyBinding::new(
            KeyAction::CompletionClose,
            KeyCombo::parse("escape").unwrap(),
        ));
        insert.push(KeyBinding::new(
            KeyAction::EnterNormalMode,
            KeyCombo::parse("ctrl+[").unwrap(),
        ));

        normal.push(KeyBinding::new(
            KeyAction::EnterInsertMode,
            KeyCombo::parse("i").unwrap(),
        ));

        Self { insert, normal }
    }

    pub fn apply_config(&mut self, config: &KeymapConfig) -> Vec<String> {
        let mut warnings = Vec::new();
        for entry in config.entries_for_mode(KeymapMode::Insert) {
            match KeyCombo::parse(entry.shortcut) {
                Ok(combo) => self.set_binding(KeymapMode::Insert, entry.action, combo),
                Err(err) => warnings.push(format!(
                    "Keymap: action {:?}: {err}",
                    entry.action
                )),
            }
        }
        for entry in config.entries_for_mode(KeymapMode::Normal) {
            match KeyCombo::parse(entry.shortcut) {
                Ok(combo) => self.set_binding(KeymapMode::Normal, entry.action, combo),
                Err(err) => warnings.push(format!(
                    "Keymap: action {:?}: {err}",
                    entry.action
                )),
            }
        }
        warnings
    }

    fn set_binding(&mut self, mode: KeymapMode, action: KeyAction, combo: KeyCombo) {
        let bindings = match mode {
            KeymapMode::Insert => &mut self.insert,
            KeymapMode::Normal => &mut self.normal,
        };
        if let Some(binding) = bindings
            .iter_mut()
            .find(|binding| binding.action == action)
        {
            binding.combo = combo;
        } else {
            bindings.push(KeyBinding::new(action, combo));
        }
    }

    pub fn match_event(
        &self,
        key: &keyboard::Key,
        modifiers: keyboard::Modifiers,
        mode: KeymapMode,
    ) -> Option<KeyAction> {
        let combo = KeyCombo::from_event(key, modifiers)?;
        let bindings = match mode {
            KeymapMode::Insert => &self.insert,
            KeymapMode::Normal => &self.normal,
        };
        bindings
            .iter()
            .find(|binding| binding.combo == combo)
            .map(|binding| binding.action)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct KeymapConfig {
    pub save: Option<String>,
    pub open: Option<String>,
    pub find: Option<String>,
    pub find_next: Option<String>,
    pub find_previous: Option<String>,
    pub completion: Option<String>,
    pub completion_close: Option<String>,
    pub enter_insert_mode: Option<String>,
    pub enter_normal_mode: Option<String>,
    #[serde(default)]
    pub insert: Option<KeymapModeConfig>,
    #[serde(default)]
    pub normal: Option<KeymapModeConfig>,
}

impl KeymapConfig {
    fn entries_for_mode(&self, mode: KeymapMode) -> Vec<KeymapEntry<'_>> {
        let mut entries = Vec::new();
        let mode_config = match mode {
            KeymapMode::Insert => self.insert.as_ref(),
            KeymapMode::Normal => self.normal.as_ref(),
        };

        entries.extend(self.entries_from_config(self, mode));
        if let Some(mode_config) = mode_config {
            entries.extend(self.entries_from_config(mode_config, mode));
        }

        entries
    }

    fn entries_from_config<'a>(
        &self,
        config: &'a impl KeymapEntries,
        mode: KeymapMode,
    ) -> Vec<KeymapEntry<'a>> {
        let mut entries = Vec::new();
        if let Some(value) = config.save() {
            entries.push(KeymapEntry {
                action: KeyAction::Save,
                shortcut: value,
            });
        }
        if let Some(value) = config.open() {
            entries.push(KeymapEntry {
                action: KeyAction::Open,
                shortcut: value,
            });
        }
        if let Some(value) = config.find() {
            entries.push(KeymapEntry {
                action: KeyAction::Find,
                shortcut: value,
            });
        }
        if let Some(value) = config.find_next() {
            entries.push(KeymapEntry {
                action: KeyAction::FindNext,
                shortcut: value,
            });
        }
        if let Some(value) = config.find_previous() {
            entries.push(KeymapEntry {
                action: KeyAction::FindPrevious,
                shortcut: value,
            });
        }
        if let Some(value) = config.completion() {
            entries.push(KeymapEntry {
                action: KeyAction::Completion,
                shortcut: value,
            });
        }
        if let Some(value) = config.completion_close() {
            entries.push(KeymapEntry {
                action: KeyAction::CompletionClose,
                shortcut: value,
            });
        }
        if let Some(value) = config.enter_insert_mode() {
            entries.push(KeymapEntry {
                action: KeyAction::EnterInsertMode,
                shortcut: value,
            });
        }
        if let Some(value) = config.enter_normal_mode() {
            entries.push(KeymapEntry {
                action: KeyAction::EnterNormalMode,
                shortcut: value,
            });
        }

        if mode == KeymapMode::Normal {
            entries.retain(|entry| entry.action != KeyAction::Completion);
        }

        entries
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct KeymapModeConfig {
    pub save: Option<String>,
    pub open: Option<String>,
    pub find: Option<String>,
    pub find_next: Option<String>,
    pub find_previous: Option<String>,
    pub completion: Option<String>,
    pub completion_close: Option<String>,
    pub enter_insert_mode: Option<String>,
    pub enter_normal_mode: Option<String>,
}

trait KeymapEntries {
    fn save(&self) -> Option<&str>;
    fn open(&self) -> Option<&str>;
    fn find(&self) -> Option<&str>;
    fn find_next(&self) -> Option<&str>;
    fn find_previous(&self) -> Option<&str>;
    fn completion(&self) -> Option<&str>;
    fn completion_close(&self) -> Option<&str>;
    fn enter_insert_mode(&self) -> Option<&str>;
    fn enter_normal_mode(&self) -> Option<&str>;
}

impl KeymapEntries for KeymapConfig {
    fn save(&self) -> Option<&str> {
        self.save.as_deref()
    }
    fn open(&self) -> Option<&str> {
        self.open.as_deref()
    }
    fn find(&self) -> Option<&str> {
        self.find.as_deref()
    }
    fn find_next(&self) -> Option<&str> {
        self.find_next.as_deref()
    }
    fn find_previous(&self) -> Option<&str> {
        self.find_previous.as_deref()
    }
    fn completion(&self) -> Option<&str> {
        self.completion.as_deref()
    }
    fn completion_close(&self) -> Option<&str> {
        self.completion_close.as_deref()
    }
    fn enter_insert_mode(&self) -> Option<&str> {
        self.enter_insert_mode.as_deref()
    }
    fn enter_normal_mode(&self) -> Option<&str> {
        self.enter_normal_mode.as_deref()
    }
}

impl KeymapEntries for KeymapModeConfig {
    fn save(&self) -> Option<&str> {
        self.save.as_deref()
    }
    fn open(&self) -> Option<&str> {
        self.open.as_deref()
    }
    fn find(&self) -> Option<&str> {
        self.find.as_deref()
    }
    fn find_next(&self) -> Option<&str> {
        self.find_next.as_deref()
    }
    fn find_previous(&self) -> Option<&str> {
        self.find_previous.as_deref()
    }
    fn completion(&self) -> Option<&str> {
        self.completion.as_deref()
    }
    fn completion_close(&self) -> Option<&str> {
        self.completion_close.as_deref()
    }
    fn enter_insert_mode(&self) -> Option<&str> {
        self.enter_insert_mode.as_deref()
    }
    fn enter_normal_mode(&self) -> Option<&str> {
        self.enter_normal_mode.as_deref()
    }
}

#[derive(Debug, Clone, Copy)]
struct KeymapEntry<'a> {
    action: KeyAction,
    shortcut: &'a str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct KeyBinding {
    action: KeyAction,
    combo: KeyCombo,
}

impl KeyBinding {
    fn new(action: KeyAction, combo: KeyCombo) -> Self {
        Self { action, combo }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct KeyCombo {
    modifiers: Modifiers,
    key: KeySpec,
}

impl KeyCombo {
    fn parse(input: &str) -> Result<Self, String> {
        let mut modifiers = Modifiers::default();
        let mut key = None;

        for part in input.split('+') {
            let part = part.trim().to_lowercase();
            match part.as_str() {
                "cmd" | "command" | "meta" => modifiers.command = true,
                "ctrl" | "control" => modifiers.control = true,
                "shift" => modifiers.shift = true,
                "alt" | "option" => modifiers.alt = true,
                "" => {}
                _ => {
                    if key.is_some() {
                        return Err(format!("raccourci invalide '{input}'"));
                    }
                    key = Some(KeySpec::parse(&part)?);
                }
            }
        }

        let key = key.ok_or_else(|| format!("raccourci invalide '{input}'"))?;
        Ok(Self { modifiers, key })
    }

    fn from_event(key: &keyboard::Key, modifiers: keyboard::Modifiers) -> Option<Self> {
        let key = KeySpec::from_key(key)?;
        Some(Self {
            modifiers: Modifiers::from_event(modifiers),
            key,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Modifiers {
    command: bool,
    control: bool,
    shift: bool,
    alt: bool,
}

impl Default for Modifiers {
    fn default() -> Self {
        Self {
            command: false,
            control: false,
            shift: false,
            alt: false,
        }
    }
}

impl Modifiers {
    fn from_event(modifiers: keyboard::Modifiers) -> Self {
        Self {
            command: modifiers.command(),
            control: modifiers.control(),
            shift: modifiers.shift(),
            alt: modifiers.alt(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KeySpec {
    Character(char),
    Named(keyboard::key::Named),
}

impl KeySpec {
    fn parse(input: &str) -> Result<Self, String> {
        let input = input.trim();
        if input.is_empty() {
            return Err("touche manquante".to_string());
        }
        match input {
            "space" => Ok(Self::Named(keyboard::key::Named::Space)),
            "escape" | "esc" => Ok(Self::Named(keyboard::key::Named::Escape)),
            "f3" => Ok(Self::Named(keyboard::key::Named::F3)),
            _ => {
                if input.len() == 1 {
                    Ok(Self::Character(input.chars().next().unwrap()))
                } else {
                    Err(format!("touche inconnue '{input}'"))
                }
            }
        }
    }

    fn from_key(key: &keyboard::Key) -> Option<Self> {
        match key {
            keyboard::Key::Character(value) => value
                .chars()
                .next()
                .map(|ch| Self::Character(ch.to_ascii_lowercase())),
            keyboard::Key::Named(named) => Some(Self::Named(*named)),
            _ => None,
        }
    }
}
