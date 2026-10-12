//! The Preferences page, opened by the Roxanne button.
//!
//! It replaces the editor below the menu bar. General, Appearance and
//! Accessibility come from the colony-ui kit, then Roxanne's own Editor, LSP
//! and Plugins, and About last. There is no Save button: every change applies
//! at once and is written to the user's file that sets it (see
//! [`store`](crate::config::store)). A setting a project file sets is not
//! saved, and its row says so.

use super::RoxanneApp;
use super::style;
use crate::config::store;
use crate::i18n::{self, t, t_fmt};
use crate::plugins::{BUILT_IN_PLUGINS, PluginManager, built_in_label};
use colony_ui::preferences::Change;
use colony_ui::widgets::{
    self, ABOUT_CATEGORY, PreferencesView, STANDARD_CATEGORIES, category_heading,
    collapsible_section, dropdown, functional_toggle, section_description, setting_row,
};
use colony_ui::{Palette, Typography};
use iced::widget::{Column, column, text_input};
use iced::{Element, Length, Task};
use std::collections::{BTreeMap, HashSet};
use std::path::Path;

/// Roxanne's own categories, between Accessibility and About.
const OWN_CATEGORIES: [&str; 3] = [
    "preferences_cat_editor",
    "preferences_cat_lsp",
    "preferences_cat_plugins",
];

/// Every category, in the order the page lists them.
fn categories() -> impl Iterator<Item = &'static str> {
    STANDARD_CATEGORIES
        .into_iter()
        .chain(OWN_CATEGORIES)
        .chain([ABOUT_CATEGORY])
}

/// The position of the category `key` in the list.
pub fn category_index(key: &str) -> usize {
    categories()
        .position(|category| category == key)
        .unwrap_or(0)
}

/// The page's own state, kept while the window is open.
#[derive(Debug, Default)]
pub struct Page {
    pub open: bool,
    category: usize,
    /// The sections the user opened.
    expanded: HashSet<String>,
    /// `theme_applied`, shown after a theme change until dismissed.
    pub toast: Option<&'static str>,
}

impl Page {
    /// Opens the page on the category at `index`.
    pub fn show(&mut self, index: usize) {
        self.open = true;
        self.category = index;
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    /// Sent by the Roxanne button and by Close: the same way in and out.
    Toggle,
    Category(usize),
    ToggleSection(&'static str),
    Change(Change),
    DismissToast,
    /// `None` follows the system language.
    Language(Option<&'static str>),
    LineNumbers(bool),
    WordWrap(bool),
    Minimap(bool),
    KeymapProfile(Option<String>),
    LspEnabled(bool),
    LspCommand(String),
    Plugin(&'static str, bool),
}

fn message(message: Message) -> super::Message {
    super::Message::Preferences(message)
}

fn set_by_project(path: &Path) -> String {
    t_fmt("preferences_set_by_project", &[("path", &path.display())])
}

impl RoxanneApp {
    pub(super) fn preferences_update(&mut self, update: Message) -> Task<super::Message> {
        let before = self.config.page_settings();
        match update {
            Message::Toggle => {
                let page = &mut self.preferences;
                page.open = !page.open;
                page.category = 0;
                page.toast = None;
                self.active_menu = None;
            }
            Message::Category(index) => self.preferences.category = index,
            Message::ToggleSection(key) => {
                let expanded = &mut self.preferences.expanded;
                if !expanded.remove(key) {
                    expanded.insert(key.to_string());
                }
            }
            Message::Change(change) => {
                self.config.preferences.update(change);
                if matches!(change, Change::Theme { .. }) {
                    self.preferences.toast = Some(t("theme_applied"));
                }
                if matches!(change, Change::FontSize(_) | Change::TextSize(_))
                    && self.editor_size != iced::Size::ZERO
                {
                    // Lines are taller or shorter: fewer or more fit.
                    self.update_viewport_height(self.editor_size);
                }
                self.palette_changed();
            }
            Message::DismissToast => self.preferences.toast = None,
            Message::Language(language) => {
                self.config.language = language.map(str::to_string);
                i18n::set_locale(i18n::resolve(language));
            }
            Message::LineNumbers(on) => self.config.editor.line_numbers = on,
            Message::WordWrap(on) => self.config.editor.word_wrap = on,
            Message::Minimap(on) => self.config.editor.minimap = on,
            Message::KeymapProfile(profile) => {
                // The bindings come from the files: saved, then read again.
                // A project's choice cannot be saved, so it stays.
                if let Some(path) = self.project_source("keymap_profile") {
                    self.status_message = Some(set_by_project(path));
                    return Task::none();
                }
                self.config.keymap_profile = profile;
                if self.save(&before) {
                    self.reload_config(false);
                }
                return Task::none();
            }
            Message::LspEnabled(on) => {
                self.config.lsp.enabled = on;
                // A server that is turned off stops now.
                self.lsp = None;
            }
            Message::LspCommand(command) => {
                self.config.lsp.rust_analyzer = command;
                // The next request starts the new program.
                self.lsp = None;
            }
            Message::Plugin(name, on) => {
                let enabled = &mut self.config.plugins.enabled;
                enabled.retain(|plugin| plugin != name);
                if on {
                    enabled.push(name.to_string());
                }
                let (mut plugins, _) = PluginManager::new(&self.config.plugins);
                let text = self.tab().content.text();
                plugins.on_text_changed(&text, &self.tab().filename);
                self.plugins = plugins;
            }
        }
        self.save(&before);
        Task::none()
    }

    /// Writes the settings that differ from `before`. The change is already
    /// in effect; a failed write, or a setting a project file sets, is
    /// reported, never retried behind the user's back. Returns whether every
    /// change was written.
    fn save(&mut self, before: &BTreeMap<String, toml::Value>) -> bool {
        let result = crate::config::user_config_path()
            .ok_or_else(|| t("preferences_about_no_config_dir").to_string())
            .and_then(|user| store::save_changes(&self.config, before, &user));
        match result {
            Ok(skipped) => match skipped.first() {
                Some(path) => {
                    self.status_message = Some(set_by_project(path));
                    false
                }
                None => true,
            },
            Err(error) => {
                self.status_message =
                    Some(t_fmt("status_preferences_not_saved", &[("error", &error)]));
                false
            }
        }
    }

    /// The project file that sets `key`, if one does.
    fn project_source(&self, key: &str) -> Option<&Path> {
        self.config
            .sources
            .get(key)
            .filter(|source| source.project)
            .map(|source| source.path.as_path())
    }

    /// `description`, or the project file that sets `key`.
    fn description(&self, key: &str, description: &str) -> String {
        self.project_source(key)
            .map_or_else(|| description.to_string(), set_by_project)
    }

    pub(super) fn preferences_page(&self, typo: &Typography) -> Element<'_, super::Message> {
        let kit = PreferencesView::new(
            *typo,
            &self.config.preferences,
            &self.preferences.expanded,
            |change| message(Message::Change(change)),
            |key| message(Message::ToggleSection(key)),
        );
        let content: Element<'_, super::Message> = match categories().nth(self.preferences.category)
        {
            Some("settings_cat_general") => kit.general().push(self.language_section(typo)).into(),
            Some("settings_cat_appearance") => kit.appearance().into(),
            Some("settings_cat_accessibility") => kit.accessibility().into(),
            Some("preferences_cat_editor") => self.editor_category(typo).into(),
            Some("preferences_cat_lsp") => self.lsp_category(typo).into(),
            Some("preferences_cat_plugins") => self.plugins_category(typo).into(),
            _ => self.about_category(typo).into(),
        };
        widgets::preferences_page(
            typo,
            widgets::category_list(
                typo,
                categories().map(t),
                self.preferences.category,
                |index| message(Message::Category(index)),
            ),
            content,
            message(Message::Toggle),
        )
    }

    /// A collapsible section, open when the user opened it.
    fn section<'a>(
        &self,
        typo: &Typography,
        key: &'static str,
        title: &str,
        content: Column<'a, super::Message>,
    ) -> Element<'a, super::Message> {
        collapsible_section(
            typo,
            title,
            self.preferences.expanded.contains(key),
            message(Message::ToggleSection(key)),
            content.into(),
        )
    }

    fn language_section(&self, typo: &Typography) -> Element<'_, super::Message> {
        let selected = match self.config.language.as_deref() {
            Some("en") => Some("en"),
            Some("fr") => Some("fr"),
            _ => None,
        };
        let options = [
            (None, t("preferences_language_system").to_string()),
            (Some("en"), t("preferences_language_en").to_string()),
            (Some("fr"), t("preferences_language_fr").to_string()),
        ];
        self.section(
            typo,
            "language",
            t("preferences_section_language"),
            column![
                section_description(typo, t("preferences_language_desc")),
                setting_row(
                    typo,
                    t("preferences_language"),
                    t("preferences_language_setting_desc"),
                    dropdown(typo, options, &selected, |language| {
                        message(Message::Language(language))
                    }),
                ),
            ],
        )
    }

    fn editor_category(&self, typo: &Typography) -> Column<'_, super::Message> {
        let editor = &self.config.editor;
        let display = column![
            section_description(typo, t("preferences_display_desc")),
            functional_toggle(
                typo,
                t("preferences_line_numbers"),
                &self.description("editor.line_numbers", t("preferences_line_numbers_desc")),
                editor.line_numbers,
                message(Message::LineNumbers(!editor.line_numbers)),
            ),
            functional_toggle(
                typo,
                t("preferences_word_wrap"),
                &self.description("editor.word_wrap", t("preferences_word_wrap_desc")),
                editor.word_wrap,
                message(Message::WordWrap(!editor.word_wrap)),
            ),
            functional_toggle(
                typo,
                t("preferences_minimap"),
                &self.description("editor.minimap", t("preferences_minimap_desc")),
                editor.minimap,
                message(Message::Minimap(!editor.minimap)),
            ),
        ];

        let mut keymap = column![section_description(typo, t("preferences_keymap_desc"))];
        if self.config.keymap_profiles.is_empty() {
            keymap = keymap.push(style::label(
                typo,
                t("preferences_keymap_none"),
                12,
                Palette::TEXT_DIMMER(),
            ));
        } else {
            let options = std::iter::once((None, t("preferences_keymap_default").to_string()))
                .chain(
                    self.config
                        .keymap_profiles
                        .iter()
                        .map(|name| (Some(name.clone()), name.clone())),
                );
            keymap = keymap.push(setting_row(
                typo,
                t("preferences_keymap_profile"),
                &self.description("keymap_profile", t("preferences_keymap_profile_desc")),
                dropdown(typo, options, &self.config.keymap_profile, |profile| {
                    message(Message::KeymapProfile(profile))
                }),
            ));
        }

        category_heading(
            typo,
            t("preferences_cat_editor"),
            t("preferences_editor_desc"),
        )
        .push(self.section(typo, "display", t("preferences_section_display"), display))
        .push(self.section(typo, "keymap", t("preferences_section_keymap"), keymap))
    }

    fn lsp_category(&self, typo: &Typography) -> Column<'_, super::Message> {
        let lsp = &self.config.lsp;
        let server = column![
            section_description(typo, t("preferences_rust_analyzer_desc")),
            functional_toggle(
                typo,
                t("preferences_lsp_enabled"),
                t("preferences_lsp_enabled_desc"),
                lsp.enabled,
                message(Message::LspEnabled(!lsp.enabled)),
            ),
            setting_row(
                typo,
                t("preferences_lsp_command"),
                t("preferences_lsp_command_desc"),
                text_input("rust-analyzer", &lsp.rust_analyzer)
                    .on_input(|command| message(Message::LspCommand(command)))
                    .padding([4, 8])
                    .size(typo.sz(12))
                    .font(typo.regular)
                    .width(Length::Fixed(240.0 * typo.scale))
                    .style(style::input),
            ),
        ];
        category_heading(typo, t("preferences_cat_lsp"), t("preferences_lsp_desc")).push(
            self.section(
                typo,
                "rust_analyzer",
                t("preferences_section_rust_analyzer"),
                server,
            ),
        )
    }

    fn plugins_category(&self, typo: &Typography) -> Column<'_, super::Message> {
        let enabled = &self.config.plugins.enabled;
        let description = self.description("plugins.enabled", t("preferences_plugin_status_bar"));
        let built_in = BUILT_IN_PLUGINS.iter().fold(
            column![section_description(typo, t("preferences_built_in_desc"))],
            |list, &name| {
                let on = enabled.iter().any(|plugin| plugin == name);
                list.push(functional_toggle(
                    typo,
                    built_in_label(name),
                    &description,
                    on,
                    message(Message::Plugin(name, !on)),
                ))
            },
        );

        let mut native = column![section_description(typo, t("preferences_native_desc"))];
        if self.config.plugins.dynamic.is_empty() {
            native = native.push(style::label(
                typo,
                t("preferences_native_none"),
                12,
                Palette::TEXT_DIMMER(),
            ));
        }
        for path in &self.config.plugins.dynamic {
            native = native.push(
                style::label(typo, path.as_str(), 12, Palette::TEXT_SECONDARY())
                    .font(style::code_font()),
            );
        }

        category_heading(
            typo,
            t("preferences_cat_plugins"),
            t("preferences_plugins_desc"),
        )
        .push(self.section(
            typo,
            "built_in",
            t("preferences_section_built_in"),
            built_in,
        ))
        .push(self.section(typo, "native", t("preferences_section_native"), native))
    }

    fn about_category(&self, typo: &Typography) -> Column<'_, super::Message> {
        let config_dir = crate::config::user_config_dir().map_or_else(
            || t("preferences_about_no_config_dir").to_string(),
            |dir| dir.display().to_string(),
        );
        let row = |title: &str, value: String| {
            setting_row(
                typo,
                title,
                "",
                style::label(typo, value, 12, Palette::TEXT_SECONDARY()),
            )
        };
        category_heading(
            typo,
            t(ABOUT_CATEGORY),
            &t_fmt(
                "preferences_about_desc",
                &[("version", &env!("CARGO_PKG_VERSION"))],
            ),
        )
        .push(row(
            t("preferences_about_license"),
            env!("CARGO_PKG_LICENSE").to_string(),
        ))
        .push(row(t("preferences_about_config"), config_dir))
        .push(row(
            t("preferences_about_source"),
            env!("CARGO_PKG_REPOSITORY").to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn categories_follow_the_colony_order() {
        let all: Vec<&str> = categories().collect();
        assert_eq!(all[..3], STANDARD_CATEGORIES);
        assert_eq!(all.last(), Some(&ABOUT_CATEGORY));
        assert_eq!(category_index(ABOUT_CATEGORY), all.len() - 1);
        assert_eq!(category_index("no-such-category"), 0);
    }

    #[test]
    fn reduced_motion_turns_the_effects_off() {
        let mut preferences = colony_ui::preferences::StandardPreferences::default();
        preferences.update(Change::ReduceMotion(true));
        assert!(!colony_ui::effects_enabled());
        preferences.update(Change::ReduceMotion(false));
        assert!(colony_ui::effects_enabled());
    }
}
