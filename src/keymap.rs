use iced::keyboard;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyAction {
    Save,
    Open,
    Find,
    FindNext,
    FindPrevious,
    SelectAll,
    Copy,
    Cut,
    Paste,
    Undo,
    Redo,
    Completion,
    CompletionClose,
    EnterInsertMode,
    EnterNormalMode,
    NewTab,
    CloseTab,
    NextTab,
    PrevTab,
    CommandPalette,
    ToggleFileTree,
    // Normal mode vim-like motions
    MoveLeft,
    MoveDown,
    MoveUp,
    MoveRight,
    MoveWordForward,
    MoveWordBackward,
    MoveLineStart,
    MoveLineEnd,
    MoveFileTop,
    MoveFileBottom,
    DeleteChar,
    DeleteLine,
    InsertAfter,
    InsertLineBelow,
    InsertLineAbove,
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
        let mut keymap = Self {
            insert: Vec::new(),
            normal: Vec::new(),
        };

        let default_bindings = [
            (KeyAction::Save, "cmd+s"),
            (KeyAction::Open, "cmd+o"),
            (KeyAction::Find, "cmd+f"),
            (KeyAction::FindNext, "f3"),
            (KeyAction::FindPrevious, "shift+f3"),
            (KeyAction::SelectAll, "cmd+a"),
            (KeyAction::SelectAll, "ctrl+a"),
            (KeyAction::Copy, "cmd+c"),
            (KeyAction::Copy, "ctrl+c"),
            (KeyAction::Cut, "cmd+x"),
            (KeyAction::Cut, "ctrl+x"),
            (KeyAction::Paste, "cmd+v"),
            (KeyAction::Paste, "ctrl+v"),
            (KeyAction::Undo, "cmd+z"),
            (KeyAction::Undo, "ctrl+z"),
            (KeyAction::Undo, "ctrl+w"),
            (KeyAction::Redo, "cmd+shift+z"),
            (KeyAction::Redo, "ctrl+shift+z"),
            (KeyAction::Redo, "ctrl+y"),
            (KeyAction::NewTab, "cmd+n"),
            (KeyAction::NewTab, "ctrl+n"),
            (KeyAction::CloseTab, "cmd+w"),
            (KeyAction::NextTab, "ctrl+tab"),
            (KeyAction::PrevTab, "ctrl+shift+tab"),
            (KeyAction::CommandPalette, "ctrl+shift+p"),
            (KeyAction::ToggleFileTree, "ctrl+b"),
        ];

        for (action, combo) in default_bindings {
            let combo = KeyCombo::parse(combo).unwrap();
            keymap.add_binding(KeymapMode::Insert, action, combo);
            keymap.add_binding(KeymapMode::Normal, action, combo);
        }

        keymap.add_binding(
            KeymapMode::Insert,
            KeyAction::Completion,
            KeyCombo::parse("ctrl+space").unwrap(),
        );
        keymap.add_binding(
            KeymapMode::Insert,
            KeyAction::CompletionClose,
            KeyCombo::parse("escape").unwrap(),
        );
        keymap.add_binding(
            KeymapMode::Insert,
            KeyAction::EnterNormalMode,
            KeyCombo::parse("ctrl+[").unwrap(),
        );

        // Normal mode: vim-like bindings
        let normal_bindings = [
            (KeyAction::EnterInsertMode, "i"),
            (KeyAction::InsertAfter, "a"),
            (KeyAction::InsertLineBelow, "o"),
            (KeyAction::InsertLineAbove, "shift+o"),
            (KeyAction::MoveLeft, "h"),
            (KeyAction::MoveDown, "j"),
            (KeyAction::MoveUp, "k"),
            (KeyAction::MoveRight, "l"),
            (KeyAction::MoveWordForward, "w"),
            (KeyAction::MoveWordBackward, "b"),
            (KeyAction::MoveLineStart, "0"),
            (KeyAction::MoveLineEnd, "shift+4"),
            (KeyAction::MoveFileTop, "g"),
            (KeyAction::MoveFileBottom, "shift+g"),
            (KeyAction::DeleteChar, "x"),
            (KeyAction::DeleteLine, "d"),
        ];
        for (action, combo) in normal_bindings {
            keymap.add_binding(
                KeymapMode::Normal,
                action,
                KeyCombo::parse(combo).unwrap(),
            );
        }

        keymap
    }

    pub fn apply_config(&mut self, config: &KeymapConfig) -> Vec<String> {
        let mut warnings = Vec::new();
        let (insert_entries, insert_warnings) = config.entries_for_mode(KeymapMode::Insert);
        warnings.extend(insert_warnings);
        for entry in insert_entries {
            match KeyCombo::parse(entry.shortcut) {
                Ok(combo) => self.set_binding(KeymapMode::Insert, entry.action, combo),
                Err(err) => warnings.push(format!(
                    "Keymap: action {:?}: {err}",
                    entry.action
                )),
            }
        }
        let (normal_entries, normal_warnings) = config.entries_for_mode(KeymapMode::Normal);
        warnings.extend(normal_warnings);
        for entry in normal_entries {
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
        bindings.retain(|binding| binding.action != action);
        bindings.push(KeyBinding::new(action, combo));
    }

    fn add_binding(&mut self, mode: KeymapMode, action: KeyAction, combo: KeyCombo) {
        let bindings = match mode {
            KeymapMode::Insert => &mut self.insert,
            KeymapMode::Normal => &mut self.normal,
        };
        bindings.push(KeyBinding::new(action, combo));
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

macro_rules! keymap_fields {
    ($struct_name:ident $(, $extra_field:ident : $extra_type:ty)*) => {
        #[derive(Debug, Clone, Serialize, Deserialize, Default)]
        pub struct $struct_name {
            pub save: Option<String>,
            pub open: Option<String>,
            pub find: Option<String>,
            pub find_next: Option<String>,
            pub find_previous: Option<String>,
            pub select_all: Option<String>,
            pub copy: Option<String>,
            pub cut: Option<String>,
            pub paste: Option<String>,
            pub undo: Option<String>,
            pub redo: Option<String>,
            pub completion: Option<String>,
            pub completion_close: Option<String>,
            pub enter_insert_mode: Option<String>,
            pub enter_normal_mode: Option<String>,
            $(
                #[serde(default)]
                pub $extra_field: $extra_type,
            )*
        }
    };
}

keymap_fields!(KeymapConfig,
    insert: Option<KeymapModeConfig>,
    normal: Option<KeymapModeConfig>
);

keymap_fields!(KeymapModeConfig);

const KEYMAP_ACTIONS: &[(&str, KeyAction)] = &[
    ("save", KeyAction::Save),
    ("open", KeyAction::Open),
    ("find", KeyAction::Find),
    ("find_next", KeyAction::FindNext),
    ("find_previous", KeyAction::FindPrevious),
    ("select_all", KeyAction::SelectAll),
    ("copy", KeyAction::Copy),
    ("cut", KeyAction::Cut),
    ("paste", KeyAction::Paste),
    ("undo", KeyAction::Undo),
    ("redo", KeyAction::Redo),
    ("completion", KeyAction::Completion),
    ("completion_close", KeyAction::CompletionClose),
    ("enter_insert_mode", KeyAction::EnterInsertMode),
    ("enter_normal_mode", KeyAction::EnterNormalMode),
    ("new_tab", KeyAction::NewTab),
    ("close_tab", KeyAction::CloseTab),
    ("next_tab", KeyAction::NextTab),
    ("prev_tab", KeyAction::PrevTab),
    ("command_palette", KeyAction::CommandPalette),
    ("toggle_file_tree", KeyAction::ToggleFileTree),
    ("move_left", KeyAction::MoveLeft),
    ("move_down", KeyAction::MoveDown),
    ("move_up", KeyAction::MoveUp),
    ("move_right", KeyAction::MoveRight),
    ("move_word_forward", KeyAction::MoveWordForward),
    ("move_word_backward", KeyAction::MoveWordBackward),
    ("move_line_start", KeyAction::MoveLineStart),
    ("move_line_end", KeyAction::MoveLineEnd),
    ("move_file_top", KeyAction::MoveFileTop),
    ("move_file_bottom", KeyAction::MoveFileBottom),
    ("delete_char", KeyAction::DeleteChar),
    ("delete_line", KeyAction::DeleteLine),
    ("insert_after", KeyAction::InsertAfter),
    ("insert_line_below", KeyAction::InsertLineBelow),
    ("insert_line_above", KeyAction::InsertLineAbove),
];

trait KeymapEntries {
    fn field(&self, name: &str) -> Option<&str>;
}

macro_rules! impl_keymap_entries {
    ($struct_name:ty) => {
        impl KeymapEntries for $struct_name {
            fn field(&self, name: &str) -> Option<&str> {
                match name {
                    "save" => self.save.as_deref(),
                    "open" => self.open.as_deref(),
                    "find" => self.find.as_deref(),
                    "find_next" => self.find_next.as_deref(),
                    "find_previous" => self.find_previous.as_deref(),
                    "select_all" => self.select_all.as_deref(),
                    "copy" => self.copy.as_deref(),
                    "cut" => self.cut.as_deref(),
                    "paste" => self.paste.as_deref(),
                    "undo" => self.undo.as_deref(),
                    "redo" => self.redo.as_deref(),
                    "completion" => self.completion.as_deref(),
                    "completion_close" => self.completion_close.as_deref(),
                    "enter_insert_mode" => self.enter_insert_mode.as_deref(),
                    "enter_normal_mode" => self.enter_normal_mode.as_deref(),
                    _ => None,
                }
            }
        }
    };
}

impl_keymap_entries!(KeymapConfig);
impl_keymap_entries!(KeymapModeConfig);

impl KeymapConfig {
    fn entries_for_mode(&self, mode: KeymapMode) -> (Vec<KeymapEntry<'_>>, Vec<String>) {
        let mut entries = Vec::new();
        let mut warnings = Vec::new();
        let mode_config = match mode {
            KeymapMode::Insert => self.insert.as_ref(),
            KeymapMode::Normal => self.normal.as_ref(),
        };

        entries.extend(collect_entries(self));
        if let Some(mode_config) = mode_config {
            entries.extend(collect_entries(mode_config));
        }

        entries.retain(|entry| {
            if action_allowed_in_mode(entry.action, mode) {
                true
            } else {
                warnings.push(format!(
                    "Keymap: action {:?} non autorisée en mode {}",
                    entry.action,
                    mode.label()
                ));
                false
            }
        });

        (entries, warnings)
    }
}

fn collect_entries<'a>(config: &'a impl KeymapEntries) -> Vec<KeymapEntry<'a>> {
    KEYMAP_ACTIONS
        .iter()
        .filter_map(|(name, action)| {
            config.field(name).map(|shortcut| KeymapEntry {
                action: *action,
                shortcut,
            })
        })
        .collect()
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
        let normalized = input.replace(['-', '_'], "");
        let named_key = match normalized.as_str() {
            "space" => Some(keyboard::key::Named::Space),
            "escape" | "esc" => Some(keyboard::key::Named::Escape),
            "enter" | "return" => Some(keyboard::key::Named::Enter),
            "tab" => Some(keyboard::key::Named::Tab),
            "backspace" => Some(keyboard::key::Named::Backspace),
            "delete" | "del" => Some(keyboard::key::Named::Delete),
            "home" => Some(keyboard::key::Named::Home),
            "end" => Some(keyboard::key::Named::End),
            "pageup" => Some(keyboard::key::Named::PageUp),
            "pagedown" => Some(keyboard::key::Named::PageDown),
            "insert" => Some(keyboard::key::Named::Insert),
            "left" => Some(keyboard::key::Named::ArrowLeft),
            "right" => Some(keyboard::key::Named::ArrowRight),
            "up" => Some(keyboard::key::Named::ArrowUp),
            "down" => Some(keyboard::key::Named::ArrowDown),
            "f1" => Some(keyboard::key::Named::F1),
            "f2" => Some(keyboard::key::Named::F2),
            "f3" => Some(keyboard::key::Named::F3),
            "f4" => Some(keyboard::key::Named::F4),
            "f5" => Some(keyboard::key::Named::F5),
            "f6" => Some(keyboard::key::Named::F6),
            "f7" => Some(keyboard::key::Named::F7),
            "f8" => Some(keyboard::key::Named::F8),
            "f9" => Some(keyboard::key::Named::F9),
            "f10" => Some(keyboard::key::Named::F10),
            "f11" => Some(keyboard::key::Named::F11),
            "f12" => Some(keyboard::key::Named::F12),
            _ => None,
        };

        if let Some(named) = named_key {
            return Ok(Self::Named(named));
        }

        if input.chars().count() == 1 {
            Ok(Self::Character(input.chars().next().unwrap()))
        } else {
            Err(format!("touche inconnue '{input}'"))
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

fn action_allowed_in_mode(action: KeyAction, mode: KeymapMode) -> bool {
    match mode {
        KeymapMode::Insert => action != KeyAction::EnterInsertMode,
        KeymapMode::Normal => {
            action != KeyAction::Completion && action != KeyAction::EnterNormalMode
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_multibyte_character() {
        let key = KeySpec::parse("é").expect("expected multibyte character to parse");
        assert_eq!(key, KeySpec::Character('é'));
    }
}
