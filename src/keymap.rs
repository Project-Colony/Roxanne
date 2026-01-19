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
}

#[derive(Debug, Clone)]
pub struct Keymap {
    bindings: Vec<KeyBinding>,
}

impl Keymap {
    pub fn default() -> Self {
        let mut bindings = Vec::new();
        bindings.push(KeyBinding::new(KeyAction::Save, KeyCombo::parse("cmd+s").unwrap()));
        bindings.push(KeyBinding::new(KeyAction::Open, KeyCombo::parse("cmd+o").unwrap()));
        bindings.push(KeyBinding::new(KeyAction::Find, KeyCombo::parse("cmd+f").unwrap()));
        bindings.push(KeyBinding::new(KeyAction::FindNext, KeyCombo::parse("f3").unwrap()));
        bindings.push(KeyBinding::new(
            KeyAction::FindPrevious,
            KeyCombo::parse("shift+f3").unwrap(),
        ));
        bindings.push(KeyBinding::new(
            KeyAction::Completion,
            KeyCombo::parse("ctrl+space").unwrap(),
        ));
        bindings.push(KeyBinding::new(
            KeyAction::CompletionClose,
            KeyCombo::parse("escape").unwrap(),
        ));
        Self { bindings }
    }

    pub fn apply_config(&mut self, config: &KeymapConfig) -> Vec<String> {
        let mut warnings = Vec::new();
        for entry in config.entries() {
            match KeyCombo::parse(entry.shortcut) {
                Ok(combo) => self.set_binding(entry.action, combo),
                Err(err) => warnings.push(format!(
                    "Keymap: action {:?}: {err}",
                    entry.action
                )),
            }
        }
        warnings
    }

    fn set_binding(&mut self, action: KeyAction, combo: KeyCombo) {
        if let Some(binding) = self
            .bindings
            .iter_mut()
            .find(|binding| binding.action == action)
        {
            binding.combo = combo;
        } else {
            self.bindings.push(KeyBinding::new(action, combo));
        }
    }

    pub fn match_event(
        &self,
        key: &keyboard::Key,
        modifiers: keyboard::Modifiers,
    ) -> Option<KeyAction> {
        let combo = KeyCombo::from_event(key, modifiers)?;
        self.bindings
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
}

impl KeymapConfig {
    fn entries(&self) -> Vec<KeymapEntry<'_>> {
        let mut entries = Vec::new();
        if let Some(value) = self.save.as_deref() {
            entries.push(KeymapEntry {
                action: KeyAction::Save,
                shortcut: value,
            });
        }
        if let Some(value) = self.open.as_deref() {
            entries.push(KeymapEntry {
                action: KeyAction::Open,
                shortcut: value,
            });
        }
        if let Some(value) = self.find.as_deref() {
            entries.push(KeymapEntry {
                action: KeyAction::Find,
                shortcut: value,
            });
        }
        if let Some(value) = self.find_next.as_deref() {
            entries.push(KeymapEntry {
                action: KeyAction::FindNext,
                shortcut: value,
            });
        }
        if let Some(value) = self.find_previous.as_deref() {
            entries.push(KeymapEntry {
                action: KeyAction::FindPrevious,
                shortcut: value,
            });
        }
        if let Some(value) = self.completion.as_deref() {
            entries.push(KeymapEntry {
                action: KeyAction::Completion,
                shortcut: value,
            });
        }
        if let Some(value) = self.completion_close.as_deref() {
            entries.push(KeymapEntry {
                action: KeyAction::CompletionClose,
                shortcut: value,
            });
        }
        entries
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
