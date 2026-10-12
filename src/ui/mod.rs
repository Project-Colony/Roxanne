//! The editor window: its state, the messages it handles, and the dispatch
//! of each message to the part of the interface it belongs to.

mod editing;
mod editor_view;
mod file_tree;
mod files;
mod keys;
mod menu;
mod panels;
mod preferences;
mod search_panel;
mod status_bar;
mod style;
mod tabs;
#[cfg(test)]
mod tests;

use crate::config::{self, AppConfig};
use crate::i18n::{self, t, t_fmt};
use crate::keymap::{KeyAction, KeymapMode};
use crate::lsp;
use crate::plugins::PluginManager;
use crate::search::{SearchResult, SearchResultsSummary, SearchScope};
use crate::watch;
use colony_ui::{Palette, Typography};
use file_tree::FileTreeEntry;
use iced::widget::text_editor::Action as EditorAction;
use iced::widget::{column, container, row, stack};
use iced::{Element, Length, Size, Subscription, Task, Theme, keyboard};
use menu::{Menu, MenuAction};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tabs::Tab;

/// Starts the editor: copies settings from before the move to
/// `Colony/Roxanne` once, loads the config, puts the preferences into effect,
/// registers the Colony fonts and opens the window.
pub fn run() -> iced::Result {
    let migrated = config::migrate_legacy_config();
    let config = AppConfig::load();
    config.preferences.apply();
    i18n::set_locale(i18n::resolve(config.language.as_deref()));
    colony_ui::fonts::BYTES
        .into_iter()
        .fold(
            iced::application(
                move || RoxanneApp::new(config.clone(), migrated.clone()),
                RoxanneApp::update,
                RoxanneApp::view,
            ),
            |app, font| app.font(font),
        )
        .title("Roxanne")
        .subscription(RoxanneApp::subscription)
        .theme(RoxanneApp::theme)
        .default_font(colony_ui::ui_font())
        .run()
}

const DEFAULT_VIEWPORT_HEIGHT: usize = 24;
/// The name of a tab that has no file yet.
const UNTITLED: &str = "untitled.txt";

#[derive(Debug)]
pub struct RoxanneApp {
    tabs: Vec<Tab>,
    active_tab: usize,
    viewport_height: usize,
    /// The editor's size, as it last reported it.
    editor_size: Size,
    search_query: String,
    replace_text: String,
    search_case_sensitive: bool,
    search_regex: bool,
    search_include_hidden: bool,
    search_panel_open: bool,
    goto_line_input: String,
    goto_panel_open: bool,
    search_scope: SearchScope,
    search_results: Vec<SearchResult>,
    diagnostics_panel_open: bool,
    completion_panel_open: bool,
    command_palette_open: bool,
    command_palette_query: String,
    file_tree_open: bool,
    file_tree: Option<FileTreeEntry>,
    file_tree_filter: String,
    status_message: Option<String>,
    /// The settings in effect: as loaded, then as changed in Preferences.
    config: AppConfig,
    /// The config files the hot reload watches, found at startup.
    config_watch_paths: Vec<PathBuf>,
    /// The iced theme, rebuilt from the Colony palette when it changes.
    theme: Theme,
    mode: KeymapMode,
    plugins: PluginManager,
    active_menu: Option<Menu>,
    performance: PerformanceMetrics,
    perf_file_open_started: Option<Instant>,
    perf_file_save_started: Option<Instant>,
    perf_search_files_started: Option<Instant>,
    save_as_open: bool,
    save_as_input: String,
    /// The existing file the user was asked about; a second Save on the same
    /// name replaces it.
    save_as_overwrite: Option<String>,
    file_changed_externally: Option<String>,
    undo_group_timer: Option<Instant>,
    /// rust-analyzer for the workspace of the last LSP request, started on
    /// the first one.
    lsp: Option<lsp::LspSession>,
    preferences: preferences::Page,
}

#[derive(Debug, Clone)]
pub enum Message {
    Edit(EditorAction),
    EditorResized(Size),
    PasteFromClipboard(Option<String>),
    Key(keyboard::Event),
    KeyAction(KeyAction),
    TabSelected(usize),
    TabClosed(usize),
    NewTab,
    NextTab,
    PrevTab,
    FilenameChanged(String),
    FileLoaded(Result<FileLoadResult, String>),
    /// The tab's id, the path and the text written to it.
    FileSaved(Result<(u64, String, String), String>),
    SaveAsChanged(String),
    SaveAsSubmit,
    SaveAsClosed,
    FileChangedExternally(String),
    FileChangedReload,
    FileChangedIgnore,
    SearchChanged(String),
    SearchNext,
    SearchPrevious,
    SearchResultSelected(usize),
    SearchToggleCaseSensitive,
    SearchToggleRegex,
    SearchToggleIncludeHidden,
    SearchScopeSelected(SearchScope),
    SearchInFiles,
    SearchResultsLoaded(Result<SearchResultsSummary, String>),
    SearchResultOpened(Result<SearchResultLoadResult, String>),
    SearchResultsCleared,
    ReplaceChanged(String),
    ReplaceCurrent,
    ReplaceAll,
    GotoLineChanged(String),
    GotoLineSubmit,
    GotoLineClosed,
    CompletionSelected(usize),
    MenuSelected(Menu),
    MenuAction(MenuAction),
    CommandPaletteChanged(String),
    CommandPaletteSelected(usize),
    FileTreeToggle,
    FileTreeToggleDir(PathBuf),
    FileTreeFileClicked(PathBuf),
    FileTreeFilterChanged(String),
    ConfigFileChanged,
    LspHoverResult(Result<Option<String>, String>),
    LspDefinitionResult(Result<Option<lsp::LspLocation>, String>),
    Preferences(preferences::Message),
}

#[derive(Debug, Clone)]
pub struct FileLoadResult {
    tab_id: u64,
    path: String,
    text: String,
    lossy: bool,
}

#[derive(Debug, Clone)]
pub struct SearchResultLoadResult {
    text: String,
    result: SearchResult,
    lossy: bool,
}

#[derive(Debug, Default)]
struct PerformanceMetrics {
    startup: Option<Duration>,
    last_viewport_refresh: Option<Duration>,
    last_search: Option<Duration>,
    last_search_files: Option<Duration>,
    last_file_open: Option<Duration>,
    last_file_save: Option<Duration>,
}

impl PerformanceMetrics {
    fn summary(&self) -> String {
        let parts: Vec<String> = [
            (self.startup, "perf_startup"),
            (self.last_viewport_refresh, "perf_render"),
            (self.last_search, "perf_search"),
            (self.last_search_files, "perf_search_files"),
            (self.last_file_open, "perf_open"),
            (self.last_file_save, "perf_save"),
        ]
        .into_iter()
        .filter_map(|(duration, key)| {
            Some(t_fmt(key, &[("duration", &format_duration(duration?))]))
        })
        .collect();
        if parts.is_empty() {
            t("perf_none").to_string()
        } else {
            t_fmt("perf_summary", &[("measures", &parts.join(" | "))])
        }
    }
}

/// `path` resolved through symlinks when it exists, else made absolute
/// against the working directory, so two spellings of one file compare equal.
fn comparable_path(path: &str) -> PathBuf {
    std::fs::canonicalize(path)
        .or_else(|_| std::path::absolute(path))
        .unwrap_or_else(|_| PathBuf::from(path))
}

fn kept_edits_message(path: &str) -> String {
    t_fmt("status_kept_edits", &[("path", &path)])
}

fn format_duration(duration: Duration) -> String {
    let millis = duration.as_secs_f64() * 1000.0;
    format!("{millis:.1} ms")
}

impl RoxanneApp {
    fn new(config: AppConfig, migrated: Option<String>) -> (Self, Task<Message>) {
        let startup_start = Instant::now();
        let tab = Tab::new(UNTITLED, "");
        let (mut plugins, plugin_warnings) = PluginManager::new(&config.plugins);
        plugins.on_text_changed("", UNTITLED);
        let warnings = config.load_warnings.len() + plugin_warnings.len();
        let startup_duration = startup_start.elapsed();
        let status_message = migrated.or_else(|| {
            Some(if warnings > 0 {
                t_fmt("status_config_warnings", &[("count", &warnings)])
            } else {
                t_fmt(
                    "status_started",
                    &[("duration", &format_duration(startup_duration))],
                )
            })
        });
        let app = Self {
            tabs: vec![tab],
            active_tab: 0,
            viewport_height: DEFAULT_VIEWPORT_HEIGHT,
            editor_size: Size::ZERO,
            search_query: String::new(),
            replace_text: String::new(),
            search_case_sensitive: false,
            search_regex: false,
            search_include_hidden: false,
            search_panel_open: false,
            goto_line_input: String::new(),
            goto_panel_open: false,
            search_scope: SearchScope::CurrentFile,
            search_results: Vec::new(),
            diagnostics_panel_open: false,
            completion_panel_open: false,
            command_palette_open: false,
            command_palette_query: String::new(),
            file_tree_open: false,
            file_tree: None,
            file_tree_filter: String::new(),
            status_message,
            config,
            config_watch_paths: config::watch_paths(),
            theme: style::iced_theme(),
            mode: KeymapMode::Insert,
            plugins,
            active_menu: None,
            performance: PerformanceMetrics {
                startup: Some(startup_duration),
                ..PerformanceMetrics::default()
            },
            perf_file_open_started: None,
            perf_file_save_started: None,
            perf_search_files_started: None,
            save_as_open: false,
            save_as_input: String::new(),
            save_as_overwrite: None,
            file_changed_externally: None,
            undo_group_timer: None,
            lsp: None,
            preferences: preferences::Page::default(),
        };
        (app, Task::none())
    }

    fn tab(&self) -> &Tab {
        &self.tabs[self.active_tab]
    }

    fn tab_mut(&mut self) -> &mut Tab {
        &mut self.tabs[self.active_tab]
    }

    fn theme(&self) -> Theme {
        self.theme.clone()
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Edit(action) => {
                self.on_edit(action);
                Task::none()
            }
            Message::EditorResized(size) => {
                self.editor_size = size;
                self.update_viewport_height(size);
                Task::none()
            }
            Message::PasteFromClipboard(text) => {
                self.paste(text);
                Task::none()
            }
            Message::Key(event) => self.on_key(event),
            Message::KeyAction(action) => self.handle_key_action(action),
            Message::TabSelected(index) => {
                self.select_tab(index);
                Task::none()
            }
            Message::TabClosed(index) => {
                self.close_tab(index);
                Task::none()
            }
            Message::NewTab => {
                self.new_tab();
                Task::none()
            }
            Message::NextTab => {
                self.cycle_tab(true);
                Task::none()
            }
            Message::PrevTab => {
                self.cycle_tab(false);
                Task::none()
            }
            Message::FilenameChanged(value) => {
                self.tab_mut().filename = value;
                Task::none()
            }
            Message::FileLoaded(result) => {
                self.file_loaded(result);
                Task::none()
            }
            Message::FileSaved(result) => {
                self.file_saved(result);
                Task::none()
            }
            Message::SaveAsChanged(value) => {
                self.save_as_input = value;
                Task::none()
            }
            Message::SaveAsSubmit => self.submit_save_as(),
            Message::SaveAsClosed => {
                self.save_as_open = false;
                Task::none()
            }
            Message::FileChangedExternally(path) => {
                self.file_changed_externally = Some(path);
                Task::none()
            }
            Message::FileChangedReload => self.reload_changed_file(),
            Message::FileChangedIgnore => {
                self.file_changed_externally = None;
                self.status_message = Some(t("status_external_change_ignored").to_string());
                Task::none()
            }
            Message::SearchChanged(value) => {
                self.search_query = value;
                let update_status = self.search_scope == SearchScope::CurrentFile;
                self.refresh_search_matches(false, update_status);
                Task::none()
            }
            Message::SearchNext => {
                self.find_next_match(true);
                Task::none()
            }
            Message::SearchPrevious => {
                self.find_next_match(false);
                Task::none()
            }
            Message::SearchResultSelected(index) => self.open_search_result(index),
            Message::SearchToggleCaseSensitive => {
                self.search_case_sensitive = !self.search_case_sensitive;
                self.refresh_current_file_search();
                Task::none()
            }
            Message::SearchToggleRegex => {
                self.search_regex = !self.search_regex;
                self.refresh_current_file_search();
                Task::none()
            }
            Message::SearchToggleIncludeHidden => {
                self.search_include_hidden = !self.search_include_hidden;
                Task::none()
            }
            Message::SearchScopeSelected(scope) => {
                self.search_scope = scope;
                self.refresh_current_file_search();
                Task::none()
            }
            Message::SearchInFiles => self.search_in_files(),
            Message::SearchResultsLoaded(result) => {
                self.search_results_loaded(result);
                Task::none()
            }
            Message::SearchResultOpened(result) => {
                self.search_result_opened(result);
                Task::none()
            }
            Message::SearchResultsCleared => {
                self.search_results.clear();
                self.status_message = Some(t("status_search_files_cleared").to_string());
                Task::none()
            }
            Message::ReplaceChanged(value) => {
                self.replace_text = value;
                Task::none()
            }
            Message::ReplaceCurrent => {
                self.replace_current_match();
                Task::none()
            }
            Message::ReplaceAll => {
                self.replace_all_matches();
                Task::none()
            }
            Message::GotoLineChanged(value) => {
                self.goto_line_input = value;
                Task::none()
            }
            Message::GotoLineSubmit => {
                self.handle_goto_line();
                Task::none()
            }
            Message::GotoLineClosed => {
                self.goto_panel_open = false;
                Task::none()
            }
            Message::CompletionSelected(index) => {
                self.apply_completion(index);
                Task::none()
            }
            Message::MenuSelected(menu) => {
                self.active_menu = (self.active_menu != Some(menu)).then_some(menu);
                Task::none()
            }
            Message::MenuAction(action) => {
                self.active_menu = None;
                self.menu_action(action)
            }
            Message::CommandPaletteChanged(value) => {
                self.command_palette_query = value;
                Task::none()
            }
            Message::CommandPaletteSelected(index) => {
                self.command_palette_open = false;
                match self.filtered_commands().get(index) {
                    Some(&(_, action)) => self.menu_action(action),
                    None => Task::none(),
                }
            }
            Message::FileTreeToggle => {
                self.toggle_file_tree();
                Task::none()
            }
            Message::FileTreeToggleDir(path) => {
                if let Some(tree) = self.file_tree.as_mut() {
                    tree.toggle_dir(&path);
                }
                Task::none()
            }
            Message::FileTreeFileClicked(path) => self.open_path(path.display().to_string()),
            Message::FileTreeFilterChanged(value) => {
                self.file_tree_filter = value;
                Task::none()
            }
            Message::ConfigFileChanged => {
                self.reload_config(false);
                Task::none()
            }
            Message::LspHoverResult(result) => {
                self.status_message = Some(match result {
                    Ok(Some(info)) => t_fmt("status_lsp_hover", &[("info", &info)]),
                    Ok(None) => t("status_lsp_no_hover").to_string(),
                    Err(err) => t_fmt("status_lsp_error", &[("error", &err)]),
                });
                Task::none()
            }
            Message::LspDefinitionResult(result) => match result {
                Ok(Some(location)) => self.open_definition(location),
                Ok(None) => {
                    self.status_message = Some(t("status_lsp_no_definition").to_string());
                    Task::none()
                }
                Err(err) => {
                    self.status_message = Some(t_fmt("status_lsp_error", &[("error", &err)]));
                    Task::none()
                }
            },
            Message::Preferences(message) => self.preferences_update(message),
        }
    }

    fn view(&self) -> Element<'_, Message> {
        // Read once per frame: the user's sizes and, with the dyslexia font
        // on, OpenDyslexic in every weight.
        let typo = Typography::current();

        let mut content = column![self.menu_bar(&typo)];
        if self.preferences.open {
            content = content.push(self.preferences_page(&typo));
        } else {
            content = content.push(self.tab_bar(&typo));
            if let Some(palette) = self.command_palette(&typo) {
                content = content.push(container(palette).center_x(Length::Fill));
            }
            for panel in [
                self.external_change_panel(&typo),
                self.search_panel(&typo),
                self.goto_panel(&typo),
                self.diagnostics_panel(&typo),
                self.completion_panel(&typo),
                self.save_as_panel(&typo),
            ]
            .into_iter()
            .flatten()
            {
                content = content.push(panel);
            }
            let mut editor_row = row![].height(Length::Fill);
            if let Some(tree) = self.file_tree_panel(&typo) {
                editor_row = editor_row.push(tree);
            }
            editor_row = editor_row.push(self.editor_area(&typo));
            if let Some(minimap) = self.minimap_panel(&typo) {
                editor_row = editor_row.push(minimap);
            }
            content = content.push(editor_row);
        }
        content = content.push(self.status_bar(&typo));

        let window: Element<'_, Message> = container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(style::surface(Palette::BG_PRIMARY()))
            .into();
        match self.preferences.toast {
            Some(notice) => stack![
                window,
                container(colony_ui::widgets::toast(
                    &typo,
                    notice,
                    Message::Preferences(preferences::Message::DismissToast),
                ))
                .padding(16)
                .align_bottom(Length::Fill)
                .center_x(Length::Fill),
            ]
            .into(),
            None => window,
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        let mut subscriptions = vec![
            keyboard::listen().map(Message::Key),
            watch::subscription(self.config_watch_paths.clone(), false)
                .map(|_| Message::ConfigFileChanged),
        ];
        // Open files, for changes made by other programs. The paths are the
        // subscription's identity, so opening or closing a file restarts the
        // watcher.
        let mut paths: Vec<PathBuf> = self
            .tabs
            .iter()
            .filter_map(|tab| tab.disk_path.as_ref().map(PathBuf::from))
            .collect();
        if !paths.is_empty() {
            paths.sort();
            paths.dedup();
            subscriptions.push(
                watch::subscription(paths, true)
                    .map(|path| Message::FileChangedExternally(path.display().to_string())),
            );
        }
        Subscription::batch(subscriptions)
    }

    /// Reloads the config files and applies what changed. A reload that finds
    /// the settings already in effect, such as the one after Preferences saved
    /// a change, does nothing. `asked` reports the result even then.
    fn reload_config(&mut self, asked: bool) {
        let config = AppConfig::load();
        if config == self.config && !asked {
            return;
        }
        let plugins_changed = config.plugins != self.config.plugins;
        let mut warnings = config.load_warnings.len();
        if plugins_changed {
            let (mut plugins, plugin_warnings) = PluginManager::new(&config.plugins);
            warnings += plugin_warnings.len();
            let text = self.tab().content.text();
            plugins.on_text_changed(&text, &self.tab().filename);
            self.plugins = plugins;
        }
        if config.preferences != self.config.preferences {
            config.preferences.apply();
        }
        if config.language != self.config.language {
            i18n::set_locale(i18n::resolve(config.language.as_deref()));
        }
        self.config = config;
        self.palette_changed();
        self.status_message = Some(if warnings == 0 {
            t("status_config_reloaded").to_string()
        } else {
            t_fmt("status_config_reloaded_warnings", &[("count", &warnings)])
        });
    }

    /// After a theme, accent or contrast change: rebuilds the iced theme and
    /// highlights every tab again in the new colours.
    fn palette_changed(&mut self) {
        self.theme = style::iced_theme();
        let colors = crate::editor::highlight::SyntaxColors::current();
        for tab in &mut self.tabs {
            tab.highlight_settings.colors = colors;
        }
    }
}
