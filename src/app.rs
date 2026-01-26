use crate::config;
use crate::editor::highlight::MatchPosition;
use crate::editor::{Position, TextBuffer, ViewportCache, highlight};
use crate::keymap::{KeyAction, Keymap, KeymapMode};
use crate::plugins::PluginManager;
use crate::theme::{ThemeConfig, ThemePalette};
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, overlay, renderer, widget};
use iced::alignment::{Horizontal, Vertical};
use iced::theme;
use iced::widget::button;
use iced::widget::text_editor::{
    Action as EditorAction, Content as EditorContent, Edit as EditorEdit, Motion,
};
use iced::widget::{
    Button, Column, Container, Scrollable, Space, TextInput, column, container, row, text,
    text_editor,
};
use iced::{
    Alignment, Application, Background, Color, Command, Element, Font, Length, Point, Rectangle,
    Renderer, Settings, Size, Subscription, Theme, Vector, clipboard, event, executor, keyboard,
    mouse, window,
};
use iced::widget::text::LineHeight;
use regex::{Regex, RegexBuilder};
use std::borrow::Cow;
use std::collections::HashSet;
use std::fs::OpenOptions;
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
#[cfg(test)]
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use walkdir::{DirEntry, WalkDir};

const DEFAULT_VIEWPORT_HEIGHT: usize = 24;
const EDITOR_CONTAINER_ID: &str = "roxanne-editor-area";
const EDITOR_VERTICAL_PADDING: f32 = 12.0;
const EDITOR_VERTICAL_PADDING_TOTAL: f32 = EDITOR_VERTICAL_PADDING * 2.0;
const EDITOR_LINE_HEIGHT: f32 = 16.0;

fn editor_container_id() -> container::Id {
    container::Id::new(EDITOR_CONTAINER_ID)
}

#[derive(Debug)]
pub struct RoxanneApp {
    filename: String,
    content: EditorContent,
    buffer: TextBuffer,
    viewport_cache: ViewportCache,
    viewport_height: usize,
    last_saved_text: String,
    search_query: String,
    search_matches: Vec<MatchPosition>,
    current_match_index: Option<usize>,
    search_case_sensitive: bool,
    search_regex: bool,
    search_include_hidden: bool,
    search_panel_open: bool,
    goto_line_input: String,
    goto_panel_open: bool,
    search_scope: SearchScope,
    search_results: Vec<SearchResult>,
    highlight_settings: highlight::Settings,
    multi_cursors: Vec<Position>,
    diagnostics: Vec<Diagnostic>,
    diagnostics_panel_open: bool,
    completion_items: Vec<CompletionItem>,
    completion_prefix: String,
    completion_panel_open: bool,
    status_message: Option<String>,
    file_is_lossy: bool,
    lossy_save_acknowledged: bool,
    theme: ThemePalette,
    keymap: Keymap,
    mode: KeymapMode,
    plugins: PluginManager,
    active_menu: Option<Menu>,
    suppress_undo_snapshot: bool,
    performance: PerformanceMetrics,
    perf_file_open_started: Option<Instant>,
    perf_file_save_started: Option<Instant>,
    perf_search_files_started: Option<Instant>,
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
        let mut parts = Vec::new();
        if let Some(duration) = self.startup {
            parts.push(format!("Démarrage {}", format_duration(duration)));
        }
        if let Some(duration) = self.last_viewport_refresh {
            parts.push(format!("Rendu {}", format_duration(duration)));
        }
        if let Some(duration) = self.last_search {
            parts.push(format!("Recherche {}", format_duration(duration)));
        }
        if let Some(duration) = self.last_search_files {
            parts.push(format!("Recherche fichiers {}", format_duration(duration)));
        }
        if let Some(duration) = self.last_file_open {
            parts.push(format!("Ouverture {}", format_duration(duration)));
        }
        if let Some(duration) = self.last_file_save {
            parts.push(format!("Sauvegarde {}", format_duration(duration)));
        }
        if parts.is_empty() {
            "Performance: aucune mesure.".to_string()
        } else {
            format!("Performance: {}.", parts.join(" | "))
        }
    }
}

fn format_duration(duration: Duration) -> String {
    let millis = duration.as_secs_f64() * 1000.0;
    format!("{millis:.1} ms")
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum Message {
    Edit(EditorAction),
    SearchChanged(String),
    SearchNext,
    SearchPrevious,
    SearchResultSelected(usize),
    SearchToggleCaseSensitive,
    SearchToggleRegex,
    SearchToggleIncludeHidden,
    SearchScopeSelected(SearchScope),
    SearchPanelToggled,
    GotoLineChanged(String),
    GotoLineSubmit,
    GotoLineClosed,
    SearchInFiles,
    SearchResultsLoaded(Result<SearchResultsSummary, String>),
    SearchResultOpened(Result<SearchResultLoadResult, String>),
    SearchResultsCleared,
    DiagnosticsToggled,
    CompletionRequested,
    CompletionSelected(usize),
    CompletionClosed,
    PasteFromClipboard(Option<String>),
    Event(event::Event),
    KeyAction(KeyAction),
    FilenameChanged(String),
    OpenPressed,
    SavePressed,
    MenuSelected(Menu),
    MenuAction(MenuAction),
    FileLoaded(Result<FileLoadResult, String>),
    FileSaved(Result<(), String>),
    ThemeExported(Result<PathBuf, String>),
    ThemeImported(Result<ThemeConfig, String>),
    EditorBoundsChanged(Option<Rectangle>),
}

#[derive(Debug, Clone)]
pub struct FileLoadResult {
    text: String,
    lossy: bool,
}

#[derive(Debug, Clone)]
pub struct SearchResultLoadResult {
    text: String,
    result: SearchResult,
    lossy: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Menu {
    File,
    Edit,
    Selection,
    View,
    Goto,
    Tools,
    Project,
    Preferences,
    Help,
}

#[derive(Debug, Clone, Copy)]
pub enum MenuAction {
    Open,
    Save,
    ExportTheme,
    ImportTheme,
    Find,
    FindNext,
    FindPrevious,
    FindInFiles,
    SelectAll,
    AddCursorNextMatch,
    AddCursorsAllMatches,
    ClearMultiCursors,
    ToggleDiagnosticsPanel,
    ToggleStatusBar,
    ToggleSearchPanel,
    GoToLine,
    ReloadConfig,
    PerformanceReport,
    About,
    Placeholder(&'static str),
}

#[derive(Debug, Clone, Copy)]
enum MenuEntry {
    Action(&'static str, MenuAction),
    Separator,
}

impl MenuEntry {
    fn action(label: &'static str, action: MenuAction) -> Self {
        Self::Action(label, action)
    }

    fn placeholder(label: &'static str) -> Self {
        Self::Action(label, MenuAction::Placeholder(label))
    }
}

const MENU_BAR_PADDING_X: f32 = 16.0;
const MENU_BUTTON_PADDING_X: u16 = 6;

struct MenuOverlay<'a> {
    content: Element<'a, Message>,
    overlay: Option<Element<'a, Message>>,
    dismiss_message: Option<Message>,
    active_menu_index: Option<usize>,
}

impl<'a> MenuOverlay<'a> {
    fn new(
        content: impl Into<Element<'a, Message>>,
        overlay: Option<Element<'a, Message>>,
        dismiss_message: Option<Message>,
        active_menu_index: Option<usize>,
    ) -> Self {
        Self {
            content: content.into(),
            overlay,
            dismiss_message,
            active_menu_index,
        }
    }
}

impl<'a> Widget<Message, Theme, Renderer> for MenuOverlay<'a> {
    fn children(&self) -> Vec<widget::Tree> {
        let mut children = vec![widget::Tree::new(&self.content)];
        if let Some(overlay) = &self.overlay {
            children.push(widget::Tree::new(overlay));
        }
        children
    }

    fn diff(&self, tree: &mut widget::Tree) {
        let mut children = vec![self.content.as_widget()];
        if let Some(overlay) = &self.overlay {
            children.push(overlay.as_widget());
        }
        tree.diff_children(&children);
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
    }

    fn layout(
        &self,
        tree: &mut widget::Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content
            .as_widget()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn on_event(
        &mut self,
        tree: &mut widget::Tree,
        event: event::Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) -> event::Status {
        self.content.as_widget_mut().on_event(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        )
    }

    fn mouse_interaction(
        &self,
        tree: &widget::Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn draw(
        &self,
        tree: &widget::Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        renderer_style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            renderer_style,
            layout,
            cursor,
            viewport,
        );
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut widget::Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        let mut children = tree.children.iter_mut();
        let content_overlay = self.content.as_widget_mut().overlay(
            children.next().unwrap(),
            layout,
            renderer,
            translation,
        );

        let active_button_x = self
            .active_menu_index
            .and_then(|index| active_menu_label_x(layout, index));

        let overlay_element = match (&mut self.overlay, children.next()) {
            (Some(overlay), Some(state)) => {
                Some(overlay::Element::new(Box::new(MenuOverlayLayer {
                    position: layout.position() + translation,
                    bounds: layout.bounds(),
                    overlay,
                    state,
                    dismiss_message: self.dismiss_message.clone(),
                    active_button_x,
                })))
            }
            _ => None,
        };

        match (content_overlay, overlay_element) {
            (Some(content_overlay), Some(overlay_element)) => Some(
                overlay::Group::with_children(vec![content_overlay, overlay_element]).overlay(),
            ),
            (Some(content_overlay), None) => Some(content_overlay),
            (None, Some(overlay_element)) => Some(overlay_element),
            (None, None) => None,
        }
    }
}

fn active_menu_label_x(layout: Layout<'_>, index: usize) -> Option<f32> {
    let mut children = layout.children();
    let row_layout = children.next()?;
    let button_layout = row_layout.children().nth(index)?;
    Some(button_layout.bounds().x + f32::from(MENU_BUTTON_PADDING_X))
}

fn menu_index(menu: Menu) -> usize {
    match menu {
        Menu::File => 0,
        Menu::Edit => 1,
        Menu::Selection => 2,
        Menu::View => 3,
        Menu::Goto => 4,
        Menu::Tools => 5,
        Menu::Project => 6,
        Menu::Preferences => 7,
        Menu::Help => 8,
    }
}

impl<'a> From<MenuOverlay<'a>> for Element<'a, Message> {
    fn from(overlay: MenuOverlay<'a>) -> Self {
        Element::new(overlay)
    }
}

struct MenuOverlayLayer<'a, 'b> {
    position: Point,
    bounds: Rectangle,
    overlay: &'b mut Element<'a, Message>,
    state: &'b mut widget::Tree,
    dismiss_message: Option<Message>,
    active_button_x: Option<f32>,
}

impl<'a, 'b> overlay::Overlay<Message, Theme, Renderer> for MenuOverlayLayer<'a, 'b> {
    fn layout(&mut self, renderer: &Renderer, bounds: Size) -> layout::Node {
        let overlay_top = self.bounds.y + self.bounds.height;
        let overlay_height = (bounds.height - overlay_top).max(0.0);
        let overlay_bounds = Rectangle {
            x: 0.0,
            y: overlay_top,
            width: bounds.width,
            height: overlay_height,
        };

        let overlay_layout = self.overlay.as_widget().layout(
            self.state,
            renderer,
            &layout::Limits::new(Size::ZERO, overlay_bounds.size()),
        );

        let submenu_offset_x = self
            .active_button_x
            .unwrap_or(self.position.x + MENU_BAR_PADDING_X);
        let submenu_offset = Vector::new(submenu_offset_x, 0.0);

        layout::Node::with_children(
            overlay_bounds.size(),
            vec![overlay_layout.translate(submenu_offset)],
        )
        .translate(Vector::new(overlay_bounds.x, overlay_bounds.y))
    }

    fn draw(
        &self,
        renderer: &mut Renderer,
        theme: &Theme,
        renderer_style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
    ) {
        if let Some(child) = layout.children().next() {
            self.overlay.as_widget().draw(
                self.state,
                renderer,
                theme,
                renderer_style,
                child,
                cursor,
                &Rectangle::with_size(Size::INFINITY),
            );
        }
    }

    fn on_event(
        &mut self,
        event: event::Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
    ) -> event::Status {
        if let Some(child) = layout.children().next() {
            if cursor.is_over(child.bounds()) {
                return self.overlay.as_widget_mut().on_event(
                    self.state,
                    event,
                    child,
                    cursor,
                    renderer,
                    clipboard,
                    shell,
                    &Rectangle::with_size(Size::INFINITY),
                );
            }
        }

        if let event::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) = event {
            if let Some(message) = self.dismiss_message.clone() {
                shell.publish(message);
                return event::Status::Captured;
            }
        }

        event::Status::Ignored
    }

    fn mouse_interaction(
        &self,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        if let Some(child) = layout.children().next() {
            return self
                .overlay
                .as_widget()
                .mouse_interaction(self.state, child, cursor, viewport, renderer);
        }
        mouse::Interaction::Idle
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchScope {
    CurrentFile,
    Workspace,
}

impl SearchScope {
    fn label(self) -> &'static str {
        match self {
            SearchScope::CurrentFile => "Fichier",
            SearchScope::Workspace => "Workspace",
        }
    }
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    path: PathBuf,
    line: usize,
    column: usize,
    preview: String,
}

#[derive(Debug, Clone)]
pub struct SearchResultsSummary {
    results: Vec<SearchResult>,
    skipped_read_errors: usize,
    skipped_too_large: usize,
    skipped_invalid_utf8: usize,
    truncated: bool,
}

#[derive(Debug, Clone)]
pub struct Diagnostic {
    line: usize,
    column: usize,
    message: String,
    severity: DiagnosticSeverity,
}

#[derive(Debug, Clone, Copy)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
}

#[derive(Debug, Clone)]
pub struct CompletionItem {
    label: String,
    detail: String,
}

#[derive(Debug, Clone, Copy)]
struct SearchOptions {
    regex: bool,
    case_sensitive: bool,
    include_hidden: bool,
}

enum SearchMatcher {
    Plain {
        needle: String,
        case_sensitive: bool,
    },
    Regex(Regex),
}

impl RoxanneApp {
    pub fn run_with_config(config: config::AppConfig) -> iced::Result {
        RoxanneApp::run_with(Settings {
            flags: config,
            ..Settings::default()
        })
    }

    pub fn run_with(settings: Settings<config::AppConfig>) -> iced::Result {
        <Self as Application>::run(settings)
    }
}

impl Application for RoxanneApp {
    type Executor = executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = config::AppConfig;

    fn new(flags: config::AppConfig) -> (Self, Command<Message>) {
        let startup_start = Instant::now();
        let initial_text = "Roxanne – éditeur en mode Iced\n\n\
            Objectif: MVP inspiré de Sublime Text\n\
            - Menu bar, tabs, status bar\n\
            - Zone d'édition monospace";
        let buffer = TextBuffer::from(initial_text);
        let mut viewport_cache = ViewportCache::new();
        viewport_cache.update(&buffer, 0, DEFAULT_VIEWPORT_HEIGHT);
        let diagnostics = analyze_diagnostics(&buffer);
        let (mut plugins, plugin_warnings) = PluginManager::new(&flags.plugins);
        plugins.on_text_changed(initial_text, "untitled.txt");
        let mut warnings = flags.load_warnings.clone();
        warnings.extend(plugin_warnings);
        let mut status_message = if warnings.is_empty() {
            None
        } else {
            Some(format!(
                "Config: {} alerte(s) lors du chargement.",
                warnings.len()
            ))
        };
        let startup_duration = startup_start.elapsed();
        let mut performance = PerformanceMetrics::default();
        performance.startup = Some(startup_duration);
        if status_message.is_none() {
            status_message = Some(format!("Démarrage: {}.", format_duration(startup_duration)));
        }
        highlight::set_syntax_palette(flags.theme.syntax);
        (
            Self {
                filename: "untitled.txt".to_string(),
                content: EditorContent::with_text(initial_text),
                buffer,
                viewport_cache,
                viewport_height: DEFAULT_VIEWPORT_HEIGHT,
                last_saved_text: initial_text.to_string(),
                search_query: String::new(),
                search_matches: Vec::new(),
                current_match_index: None,
                search_case_sensitive: false,
                search_regex: false,
                search_include_hidden: false,
                search_panel_open: false,
                goto_line_input: String::new(),
                goto_panel_open: false,
                search_scope: SearchScope::CurrentFile,
                search_results: Vec::new(),
                highlight_settings: highlight::Settings {
                    buffer_text: initial_text.into(),
                    ..highlight::Settings::default()
                },
                multi_cursors: Vec::new(),
                diagnostics,
                diagnostics_panel_open: false,
                completion_items: Vec::new(),
                completion_prefix: String::new(),
                completion_panel_open: false,
                status_message,
                file_is_lossy: false,
                lossy_save_acknowledged: false,
                theme: flags.theme,
                keymap: flags.keymap,
                mode: KeymapMode::Insert,
                plugins,
                active_menu: None,
                suppress_undo_snapshot: false,
                performance,
                perf_file_open_started: None,
                perf_file_save_started: None,
                perf_search_files_started: None,
            },
            Self::editor_bounds_command(),
        )
    }

    fn title(&self) -> String {
        "Roxanne".to_string()
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::Edit(action) => {
                if action.is_edit() {
                    self.record_undo_snapshot();
                }
                if action.is_edit() && !self.multi_cursors.is_empty() {
                    if let EditorAction::Edit(edit) = action {
                        self.apply_multi_cursor_edit(&edit);
                    }
                } else {
                    self.content.perform(action);
                    self.buffer.replace(&self.content.text());
                }
                self.sync_highlight_buffer();
                self.refresh_search_matches(true, true);
                self.refresh_diagnostics();
                self.plugins
                    .on_text_changed(&self.content.text(), &self.filename);
                self.refresh_viewport_cache();
                Command::none()
            }
            Message::SearchChanged(value) => {
                self.search_query = value;
                let update_status = self.search_scope == SearchScope::CurrentFile;
                self.refresh_search_matches(false, update_status);
                Command::none()
            }
            Message::SearchNext => self.find_next_match(true),
            Message::SearchPrevious => self.find_next_match(false),
            Message::SearchResultSelected(index) => self.open_search_result(index),
            Message::SearchToggleCaseSensitive => {
                self.search_case_sensitive = !self.search_case_sensitive;
                if self.search_scope == SearchScope::CurrentFile {
                    self.refresh_search_matches(false, true);
                }
                Command::none()
            }
            Message::SearchToggleRegex => {
                self.search_regex = !self.search_regex;
                if self.search_scope == SearchScope::CurrentFile {
                    self.refresh_search_matches(false, true);
                }
                Command::none()
            }
            Message::SearchToggleIncludeHidden => {
                self.search_include_hidden = !self.search_include_hidden;
                Command::none()
            }
            Message::SearchScopeSelected(scope) => {
                self.search_scope = scope;
                if self.search_scope == SearchScope::CurrentFile {
                    self.refresh_search_matches(false, true);
                }
                Command::none()
            }
            Message::SearchPanelToggled => {
                self.search_panel_open = !self.search_panel_open;
                Self::editor_bounds_command()
            }
            Message::GotoLineChanged(value) => {
                self.goto_line_input = value;
                Command::none()
            }
            Message::GotoLineSubmit => {
                self.handle_goto_line();
                Command::none()
            }
            Message::GotoLineClosed => {
                self.goto_panel_open = false;
                Self::editor_bounds_command()
            }
            Message::DiagnosticsToggled => {
                self.diagnostics_panel_open = !self.diagnostics_panel_open;
                Self::editor_bounds_command()
            }
            Message::CompletionRequested => {
                self.refresh_completions();
                self.completion_panel_open = !self.completion_items.is_empty();
                Self::editor_bounds_command()
            }
            Message::CompletionSelected(index) => {
                self.apply_completion(index);
                Command::none()
            }
            Message::CompletionClosed => {
                self.completion_panel_open = false;
                self.completion_items.clear();
                Self::editor_bounds_command()
            }
            Message::EditorBoundsChanged(bounds) => {
                if let Some(bounds) = bounds {
                    self.update_viewport_height(bounds);
                }
                Command::none()
            }
            Message::PasteFromClipboard(text) => {
                if let Some(text) = text {
                    if text.is_empty() {
                        return Command::none();
                    }
                    let insert = EditorEdit::Paste(std::sync::Arc::new(text));
                    if self.multi_cursors.is_empty() {
                        self.record_undo_snapshot();
                        self.content.perform(EditorAction::Edit(insert));
                        self.buffer.replace(&self.content.text());
                    } else {
                        self.record_undo_snapshot();
                        self.apply_multi_cursor_edit(&insert);
                    }
                    self.sync_highlight_buffer();
                    self.refresh_search_matches(true, true);
                    self.refresh_diagnostics();
                    self.plugins
                        .on_text_changed(&self.content.text(), &self.filename);
                    self.refresh_viewport_cache();
                }
                Command::none()
            }
            Message::Event(event) => {
                match event {
                    event::Event::Keyboard(keyboard::Event::KeyPressed {
                        key, modifiers, ..
                    }) => {
                        if let Some(action) = self.keymap.match_event(&key, modifiers, self.mode) {
                            if matches!(action, KeyAction::Copy | KeyAction::Cut | KeyAction::Paste)
                            {
                                if let keyboard::Key::Character(value) = &key {
                                    let key_char = value.to_ascii_lowercase();
                                    if modifiers.command()
                                        && matches!(key_char.as_str(), "c" | "x" | "v")
                                    {
                                        return Command::none();
                                    }
                                }
                            }
                            return self.handle_key_action(action);
                        }
                    }
                    event::Event::Window(
                        _,
                        window::Event::Resized { .. } | window::Event::Opened { .. },
                    ) => {
                        return Self::editor_bounds_command();
                    }
                    _ => {}
                }
                Command::none()
            }
            Message::KeyAction(action) => self.handle_key_action(action),
            Message::SearchInFiles => self.search_in_files(),
            Message::SearchResultsLoaded(result) => {
                if let Some(started) = self.perf_search_files_started.take() {
                    self.performance.last_search_files = Some(started.elapsed());
                }
                match result {
                    Ok(summary) => {
                        self.search_results = summary.results;
                        let duration = self
                            .performance
                            .last_search_files
                            .map(format_duration)
                            .unwrap_or_else(|| "-".to_string());
                        let skipped_total = summary.skipped_read_errors
                            + summary.skipped_too_large
                            + summary.skipped_invalid_utf8;
                        let skipped_note = if skipped_total > 0 {
                            let mut details = Vec::new();
                            if summary.skipped_too_large > 0 {
                                details.push(format!(
                                    "{} trop volumineux",
                                    summary.skipped_too_large
                                ));
                            }
                            if summary.skipped_invalid_utf8 > 0 {
                                details.push(format!(
                                    "{} non UTF-8",
                                    summary.skipped_invalid_utf8
                                ));
                            }
                            if summary.skipped_read_errors > 0 {
                                details.push(format!(
                                    "{} erreur(s) de lecture",
                                    summary.skipped_read_errors
                                ));
                            }
                            format!(
                                ", {skipped_total} fichier(s) ignoré(s) : {}",
                                details.join(", ")
                            )
                        } else {
                            String::new()
                        };
                        let truncated_note = if summary.truncated {
                            " résultats partiels / limite atteinte"
                        } else {
                            ""
                        };
                        self.status_message = Some(format!(
                            "Recherche fichiers: {} résultat(s) ({duration}){skipped_note}{truncated_note}.",
                            self.search_results.len()
                        ));
                    }
                    Err(message) => {
                        self.status_message = Some(format!("Recherche fichiers: {message}"));
                    }
                }
                Command::none()
            }
            Message::SearchResultOpened(result) => {
                match result {
                    Ok(load) => {
                        let nav = load.result;
                        let text = load.text;
                        self.file_is_lossy = load.lossy;
                        self.lossy_save_acknowledged = false;
                        self.filename = nav.path.display().to_string();
                        self.content = EditorContent::with_text(&text);
                        self.buffer.replace(&text);
                        self.buffer.clear_history();
                        self.multi_cursors.clear();
                        self.completion_panel_open = false;
                        self.completion_items.clear();
                        self.suppress_undo_snapshot = false;
                        self.last_saved_text = text;
                        self.sync_highlight_buffer();
                        self.refresh_search_matches(false, true);
                        self.refresh_diagnostics();
                        self.plugins
                            .on_text_changed(&self.content.text(), &self.filename);
                        self.refresh_viewport_cache();
                        self.jump_to_position(nav.line, nav.column);
                        let opened_message = format!(
                            "Recherche: ouvert {} (ligne {}, colonne {})",
                            nav.path.display(),
                            nav.line + 1,
                            nav.column + 1
                        );
                        if load.lossy {
                            self.status_message = Some(format!(
                                "{opened_message} (caractères invalides remplacés). Sauvegarde \
bloquée tant qu'une confirmation explicite n'est pas donnée."
                            ));
                        } else {
                            self.status_message = Some(format!("{opened_message}."));
                        }
                    }
                    Err(message) => {
                        self.status_message = Some(format!("Recherche fichiers: {message}"));
                    }
                }
                Command::none()
            }
            Message::SearchResultsCleared => {
                self.search_results.clear();
                self.status_message = Some("Recherche fichiers: résultats effacés.".to_string());
                Command::none()
            }
            Message::FilenameChanged(value) => {
                self.filename = value;
                Command::none()
            }
            Message::OpenPressed => self.open_file(),
            Message::SavePressed => self.save_file(),
            Message::MenuSelected(menu) => {
                if self.active_menu == Some(menu) {
                    self.active_menu = None;
                } else {
                    self.active_menu = Some(menu);
                }
                Command::none()
            }
            Message::MenuAction(action) => {
                self.active_menu = None;
                match action {
                    MenuAction::Open => self.open_file(),
                    MenuAction::Save => self.save_file(),
                    MenuAction::ExportTheme => self.export_theme(),
                    MenuAction::ImportTheme => self.import_theme(),
                    MenuAction::Find => {
                        self.status_message = Some(
                            "Recherche: utilisez le champ de recherche dans la barre d'état."
                                .to_string(),
                        );
                        Command::none()
                    }
                    MenuAction::FindNext => self.find_next_match(true),
                    MenuAction::FindPrevious => self.find_next_match(false),
                    MenuAction::FindInFiles => {
                        self.search_panel_open = true;
                        self.search_scope = SearchScope::Workspace;
                        Command::batch([Self::editor_bounds_command(), self.search_in_files()])
                    }
                    MenuAction::SelectAll => {
                        self.status_message =
                            Some("Sélection: Ctrl+A ou Cmd+A pour tout sélectionner.".to_string());
                        Command::none()
                    }
                    MenuAction::AddCursorNextMatch => {
                        self.add_cursor_next_match();
                        Command::none()
                    }
                    MenuAction::AddCursorsAllMatches => {
                        self.add_cursors_all_matches();
                        Command::none()
                    }
                    MenuAction::ClearMultiCursors => {
                        self.multi_cursors.clear();
                        self.status_message =
                            Some("Multi-curseurs: positions effacées.".to_string());
                        Command::none()
                    }
                    MenuAction::ToggleDiagnosticsPanel => {
                        self.diagnostics_panel_open = !self.diagnostics_panel_open;
                        Self::editor_bounds_command()
                    }
                    MenuAction::ToggleStatusBar => {
                        self.status_message =
                            Some("Affichage: options avancées à venir.".to_string());
                        Command::none()
                    }
                    MenuAction::ToggleSearchPanel => {
                        self.search_panel_open = !self.search_panel_open;
                        Self::editor_bounds_command()
                    }
                    MenuAction::GoToLine => {
                        let (line, _) = self.content.cursor_position();
                        self.goto_line_input = format!("{}", line + 1);
                        self.goto_panel_open = true;
                        Self::editor_bounds_command()
                    }
                    MenuAction::ReloadConfig => self.reload_config(),
                    MenuAction::PerformanceReport => {
                        self.status_message = Some(self.performance.summary());
                        Command::none()
                    }
                    MenuAction::About => {
                        self.status_message =
                            Some("Roxanne MVP: éditeur inspiré de Sublime Text.".to_string());
                        Command::none()
                    }
                    MenuAction::Placeholder(label) => {
                        self.status_message = Some(format!("Non implémenté : {label}."));
                        Command::none()
                    }
                }
            }
            Message::FileLoaded(result) => {
                if let Some(started) = self.perf_file_open_started.take() {
                    self.performance.last_file_open = Some(started.elapsed());
                }
                match result {
                    Ok(load) => {
                        let text = load.text;
                        self.file_is_lossy = load.lossy;
                        self.lossy_save_acknowledged = false;
                        self.content = EditorContent::with_text(&text);
                        self.buffer.replace(&text);
                        self.last_saved_text = text;
                        self.buffer.clear_history();
                        self.multi_cursors.clear();
                        self.completion_panel_open = false;
                        self.completion_items.clear();
                        self.suppress_undo_snapshot = false;
                        self.sync_highlight_buffer();
                        self.refresh_search_matches(false, true);
                        self.refresh_diagnostics();
                        self.plugins
                            .on_file_opened(&self.content.text(), &self.filename);
                        self.plugins
                            .on_text_changed(&self.content.text(), &self.filename);
                        self.refresh_viewport_cache();
                        let duration = self
                            .performance
                            .last_file_open
                            .map(format_duration)
                            .unwrap_or_else(|| "-".to_string());
                        if load.lossy {
                            self.status_message = Some(format!(
                                "Fichier chargé ({duration}, caractères invalides remplacés). \
Sauvegarde bloquée tant qu'une confirmation explicite n'est pas donnée."
                            ));
                        } else {
                            self.status_message = Some(format!("Fichier chargé ({duration})."));
                        }
                    }
                    Err(message) => {
                        self.status_message = Some(format!("Erreur d'ouverture: {message}"));
                    }
                }
                Command::none()
            }
            Message::FileSaved(result) => {
                if let Some(started) = self.perf_file_save_started.take() {
                    self.performance.last_file_save = Some(started.elapsed());
                }
                match result {
                    Ok(()) => {
                        self.last_saved_text = self.content.text().to_string();
                        self.plugins
                            .on_file_saved(&self.last_saved_text, &self.filename);
                        let duration = self
                            .performance
                            .last_file_save
                            .map(format_duration)
                            .unwrap_or_else(|| "-".to_string());
                        self.status_message = Some(format!("Fichier sauvegardé ({duration})."));
                    }
                    Err(message) => {
                        self.status_message = Some(format!("Erreur de sauvegarde: {message}"));
                    }
                }
                Command::none()
            }
            Message::ThemeExported(result) => {
                match result {
                    Ok(path) => {
                        self.status_message =
                            Some(format!("Thème exporté vers {}.", path.display()));
                    }
                    Err(message) => {
                        self.status_message = Some(format!("Export thème: {message}"));
                    }
                }
                Command::none()
            }
            Message::ThemeImported(result) => {
                match result {
                    Ok(theme_config) => {
                        let mut palette = self.theme;
                        let warnings = theme_config.apply_to(&mut palette);
                        highlight::set_syntax_palette(palette.syntax);
                        self.theme = palette;
                        self.status_message = Some(if warnings.is_empty() {
                            "Thème importé.".to_string()
                        } else {
                            format!("Thème importé avec {} alerte(s).", warnings.len())
                        });
                    }
                    Err(message) => {
                        self.status_message = Some(format!("Import thème: {message}"));
                    }
                }
                Command::none()
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let menu_bar = self.menu_bar();
        let tab_bar = self.tab_bar();
        let search_panel = self.search_panel();
        let goto_panel = self.goto_panel();
        let diagnostics_panel = self.diagnostics_panel();
        let completion_panel = self.completion_panel();
        let editor = self.editor_area();
        let status_bar = self.status_bar();

        let mut content = column![menu_bar, tab_bar];
        if let Some(panel) = search_panel {
            content = content.push(panel);
        }
        if let Some(panel) = goto_panel {
            content = content.push(panel);
        }
        if let Some(panel) = diagnostics_panel {
            content = content.push(panel);
        }
        if let Some(panel) = completion_panel {
            content = content.push(panel);
        }
        let content = content
            .push(editor)
            .push(status_bar)
            .spacing(0)
            .width(Length::Fill)
            .height(Length::Fill);

        Container::new(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(theme::Container::Custom(Box::new(AppBackground {
                palette: self.theme,
            })))
            .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        event::listen().map(Message::Event)
    }
}

impl RoxanneApp {
    fn editor_bounds_command() -> Command<Message> {
        container::visible_bounds(editor_container_id()).map(Message::EditorBoundsChanged)
    }

    fn update_viewport_height(&mut self, bounds: Rectangle) {
        let available_height = (bounds.height - EDITOR_VERTICAL_PADDING_TOTAL).max(0.0);
        let line_height = EDITOR_LINE_HEIGHT.max(1.0);
        let visible_lines = (available_height / line_height).ceil().max(1.0) as usize;
        if visible_lines != self.viewport_height {
            self.viewport_height = visible_lines;
            self.refresh_viewport_cache();
        }
    }

    fn open_file(&mut self) -> Command<Message> {
        if self.filename.trim().is_empty() {
            self.status_message = Some("Nom de fichier manquant.".to_string());
            return Command::none();
        }
        let filename = self.filename.clone();
        self.perf_file_open_started = Some(Instant::now());
        Command::perform(
            async move {
                let metadata = std::fs::metadata(&filename).map_err(|err| err.to_string())?;
                if metadata.len() > MAX_OPEN_FILE_SIZE {
                    return Err(format!(
                        "fichier trop volumineux ({} octets, limite {} octets)",
                        metadata.len(),
                        MAX_OPEN_FILE_SIZE
                    ));
                }
                let bytes = std::fs::read(&filename).map_err(|err| err.to_string())?;
                let lossy_text = String::from_utf8_lossy(&bytes);
                let lossy = matches!(lossy_text, Cow::Owned(_));
                Ok(FileLoadResult {
                    text: lossy_text.into_owned(),
                    lossy,
                })
            },
            Message::FileLoaded,
        )
    }

    fn save_file(&mut self) -> Command<Message> {
        if self.filename.trim().is_empty() {
            self.status_message = Some("Nom de fichier manquant.".to_string());
            return Command::none();
        }
        if self.file_is_lossy && !self.lossy_save_acknowledged {
            self.lossy_save_acknowledged = true;
            self.status_message = Some(
                "Sauvegarde bloquée: le fichier contient des caractères invalides remplacés. \
Relancez “Enregistrer” pour confirmer l’écriture."
                    .to_string(),
            );
            return Command::none();
        }
        if self.file_is_lossy {
            self.lossy_save_acknowledged = false;
        }
        let filename = self.filename.clone();
        self.buffer.replace(&self.content.text());
        let text = self.buffer.text();
        self.perf_file_save_started = Some(Instant::now());
        Command::perform(
            async move { atomic_write(&filename, &text).map_err(|err| err.to_string()) },
            Message::FileSaved,
        )
    }

    fn export_theme(&mut self) -> Command<Message> {
        let path = match Self::theme_file_path() {
            Ok(path) => path,
            Err(message) => {
                self.status_message = Some(format!("Export thème: {message}"));
                return Command::none();
            }
        };
        let theme = self.theme.to_config();
        Command::perform(
            async move {
                let contents = toml::to_string_pretty(&theme).map_err(|err| err.to_string())?;
                std::fs::write(&path, contents).map_err(|err| err.to_string())?;
                Ok(path)
            },
            Message::ThemeExported,
        )
    }

    fn import_theme(&mut self) -> Command<Message> {
        let path = match Self::theme_file_path() {
            Ok(path) => path,
            Err(message) => {
                self.status_message = Some(format!("Import thème: {message}"));
                return Command::none();
            }
        };
        Command::perform(
            async move {
                let contents = std::fs::read_to_string(&path).map_err(|err| err.to_string())?;
                let theme =
                    toml::from_str::<ThemeConfig>(&contents).map_err(|err| err.to_string())?;
                Ok(theme)
            },
            Message::ThemeImported,
        )
    }

    fn theme_file_path() -> Result<PathBuf, String> {
        std::env::current_dir()
            .map(|dir| dir.join(".roxanne-theme.toml"))
            .map_err(|err| err.to_string())
    }

    fn handle_key_action(&mut self, action: KeyAction) -> Command<Message> {
        match action {
            KeyAction::Save => self.save_file(),
            KeyAction::Open => self.open_file(),
            KeyAction::Find => {
                self.status_message = Some(
                    "Recherche: utilisez le champ de recherche dans la barre d'état.".to_string(),
                );
                Command::none()
            }
            KeyAction::FindNext => self.find_next_match(true),
            KeyAction::FindPrevious => self.find_next_match(false),
            KeyAction::SelectAll => {
                self.content
                    .perform(EditorAction::Move(Motion::DocumentStart));
                self.content
                    .perform(EditorAction::Select(Motion::DocumentEnd));
                self.refresh_viewport_cache();
                Command::none()
            }
            KeyAction::Copy => {
                if let Some(selection) = self.content.selection() {
                    if !selection.is_empty() {
                        return clipboard::write(selection);
                    }
                }
                Command::none()
            }
            KeyAction::Cut => {
                if let Some(selection) = self.content.selection() {
                    if selection.is_empty() {
                        return Command::none();
                    }
                    self.record_undo_snapshot();
                    self.content
                        .perform(EditorAction::Edit(EditorEdit::Backspace));
                    self.buffer.replace(&self.content.text());
                    self.sync_highlight_buffer();
                    self.refresh_search_matches(true, true);
                    self.refresh_diagnostics();
                    self.plugins
                        .on_text_changed(&self.content.text(), &self.filename);
                    self.refresh_viewport_cache();
                    return clipboard::write(selection);
                }
                Command::none()
            }
            KeyAction::Paste => clipboard::read(Message::PasteFromClipboard),
            KeyAction::Undo => {
                self.apply_undo();
                Command::none()
            }
            KeyAction::Redo => {
                self.apply_redo();
                Command::none()
            }
            KeyAction::Completion => {
                self.refresh_completions();
                self.completion_panel_open = !self.completion_items.is_empty();
                Command::none()
            }
            KeyAction::CompletionClose => {
                self.completion_panel_open = false;
                self.completion_items.clear();
                Command::none()
            }
            KeyAction::EnterInsertMode => {
                self.mode = KeymapMode::Insert;
                self.status_message = Some("Mode insertion.".to_string());
                Command::none()
            }
            KeyAction::EnterNormalMode => {
                self.mode = KeymapMode::Normal;
                self.status_message = Some("Mode normal.".to_string());
                Command::none()
            }
        }
    }

    fn menu_bar(&self) -> Element<'_, Message> {
        let menu_items = row![
            self.menu_button("File", Menu::File),
            self.menu_button("Edit", Menu::Edit),
            self.menu_button("Selection", Menu::Selection),
            self.menu_button("View", Menu::View),
            self.menu_button("Goto", Menu::Goto),
            self.menu_button("Tools", Menu::Tools),
            self.menu_button("Project", Menu::Project),
            self.menu_button("Preferences", Menu::Preferences),
            self.menu_button("Help", Menu::Help),
        ]
        .spacing(16)
        .align_items(Alignment::Center);

        let top_row = Container::new(menu_items)
            .width(Length::Fill)
            .padding([6, 16])
            .style(theme::Container::Custom(Box::new(MenuBarStyle {
                palette: self.theme,
            })));

        let dismiss_message = self.active_menu.map(Message::MenuSelected);
        let active_menu_index = self.active_menu.map(menu_index);
        MenuOverlay::new(top_row, self.submenu(), dismiss_message, active_menu_index).into()
    }

    fn menu_button(&self, label: &str, menu: Menu) -> Element<'_, Message> {
        let is_active = self.active_menu == Some(menu);
        Button::new(
            text(label)
                .size(14)
                .style(Color::from_rgb8(220, 220, 220))
                .font(Font::MONOSPACE),
        )
        .padding([2, MENU_BUTTON_PADDING_X])
        .style(theme::Button::Custom(Box::new(MenuButtonStyle {
            active: is_active,
            palette: self.theme,
        })))
        .on_press(Message::MenuSelected(menu))
        .into()
    }

    fn submenu(&self) -> Option<Element<'_, Message>> {
        let (label, actions) = match self.active_menu? {
            Menu::File => (
                "File",
                vec![
                    MenuEntry::placeholder("New File — Ctrl+N"),
                    MenuEntry::placeholder("New Window — Ctrl+Shift+N"),
                    MenuEntry::action("Open File… — Ctrl+O", MenuAction::Open),
                    MenuEntry::placeholder("Open Folder… — Ctrl+Shift+O"),
                    MenuEntry::placeholder("Open Recent ▸"),
                    MenuEntry::placeholder("Reopen Closed File — Ctrl+Shift+T"),
                    MenuEntry::Separator,
                    MenuEntry::placeholder("Close File — Ctrl+W"),
                    MenuEntry::placeholder("Close Window — Ctrl+Shift+W"),
                    MenuEntry::Separator,
                    MenuEntry::action("Save — Ctrl+S", MenuAction::Save),
                    MenuEntry::placeholder("Save As… — Ctrl+Shift+S"),
                    MenuEntry::placeholder("Save All — Ctrl+Alt+S"),
                    MenuEntry::placeholder("Save with Encoding ▸"),
                    MenuEntry::placeholder("Save with Line Endings ▸"),
                    MenuEntry::Separator,
                    MenuEntry::action("Export Theme", MenuAction::ExportTheme),
                    MenuEntry::action("Import Theme", MenuAction::ImportTheme),
                    MenuEntry::Separator,
                    MenuEntry::placeholder("Print… — Ctrl+P"),
                    MenuEntry::placeholder("Exit — Ctrl+Q"),
                ],
            ),
            Menu::Edit => (
                "Edit",
                vec![
                    MenuEntry::placeholder("Undo — Ctrl+Z"),
                    MenuEntry::placeholder("Redo — Ctrl+Y"),
                    MenuEntry::placeholder("Undo Selection — Ctrl+U"),
                    MenuEntry::placeholder("Soft Undo — Alt+Backspace"),
                    MenuEntry::placeholder("Soft Redo — Alt+Shift+Backspace"),
                    MenuEntry::Separator,
                    MenuEntry::placeholder("Cut — Ctrl+X"),
                    MenuEntry::placeholder("Copy — Ctrl+C"),
                    MenuEntry::placeholder("Copy as RTF"),
                    MenuEntry::placeholder("Paste — Ctrl+V"),
                    MenuEntry::placeholder("Paste and Indent — Ctrl+Shift+V"),
                    MenuEntry::placeholder("Paste from History ▸"),
                    MenuEntry::placeholder("Paste Special ▸"),
                    MenuEntry::Separator,
                    MenuEntry::action("Find — Ctrl+F", MenuAction::Find),
                    MenuEntry::action("Find Next — F3", MenuAction::FindNext),
                    MenuEntry::action("Find Previous — Shift+F3", MenuAction::FindPrevious),
                    MenuEntry::action("Find in Files… — Ctrl+Shift+F", MenuAction::FindInFiles),
                    MenuEntry::placeholder("Replace… — Ctrl+H"),
                    MenuEntry::Separator,
                    MenuEntry::action("Search Panel", MenuAction::ToggleSearchPanel),
                    MenuEntry::action("Diagnostics Panel", MenuAction::ToggleDiagnosticsPanel),
                ],
            ),
            Menu::Selection => (
                "Selection",
                vec![
                    MenuEntry::placeholder("Single Selection — Esc"),
                    MenuEntry::action("Select All — Ctrl+A", MenuAction::SelectAll),
                    MenuEntry::placeholder("Expand Selection to Line — Ctrl+L"),
                    MenuEntry::placeholder("Split Selection into Lines — Ctrl+Shift+L"),
                    MenuEntry::action(
                        "Add Cursor to Next Match — Ctrl+D",
                        MenuAction::AddCursorNextMatch,
                    ),
                    MenuEntry::action(
                        "Add Cursors to All Matches — Alt+F3",
                        MenuAction::AddCursorsAllMatches,
                    ),
                    MenuEntry::action("Clear Cursors — Esc", MenuAction::ClearMultiCursors),
                    MenuEntry::Separator,
                    MenuEntry::placeholder("Expand Selection to Word — Ctrl+D"),
                    MenuEntry::placeholder("Expand Selection to Paragraph — Ctrl+Shift+Space"),
                    MenuEntry::placeholder("Expand Selection to Brackets — Ctrl+Shift+M"),
                    MenuEntry::placeholder("Expand Selection to Tag — Ctrl+Shift+A"),
                ],
            ),
            Menu::View => (
                "View",
                vec![
                    MenuEntry::placeholder("Side Bar"),
                    MenuEntry::placeholder("Show Minimap"),
                    MenuEntry::placeholder("Show Tabs"),
                    MenuEntry::action("Show Status Bar", MenuAction::ToggleStatusBar),
                    MenuEntry::placeholder("Show Menu"),
                    MenuEntry::Separator,
                    MenuEntry::placeholder("Enter Full Screen — F11"),
                    MenuEntry::placeholder("Distraction Free Mode — Shift+F11"),
                    MenuEntry::Separator,
                    MenuEntry::placeholder("Layout ▸"),
                    MenuEntry::placeholder("Groups ▸"),
                    MenuEntry::placeholder("Move File to Group ▸"),
                    MenuEntry::placeholder("Focus Group ▸"),
                ],
            ),
            Menu::Goto => (
                "Goto",
                vec![
                    MenuEntry::placeholder("Goto Anything… — Ctrl+P"),
                    MenuEntry::placeholder("Goto Symbol… — Ctrl+R"),
                    MenuEntry::placeholder("Goto Symbol in Project… — Ctrl+Shift+R"),
                    MenuEntry::placeholder("Goto Definition — F12"),
                    MenuEntry::placeholder("Goto Reference — Shift+F12"),
                    MenuEntry::Separator,
                    MenuEntry::action("Go to Line… — Ctrl+G", MenuAction::GoToLine),
                    MenuEntry::placeholder("Go to Word… — Ctrl+;"),
                    MenuEntry::placeholder("Go to Section… — Ctrl+Shift+;"),
                ],
            ),
            Menu::Tools => (
                "Tools",
                vec![
                    MenuEntry::placeholder("Command Palette… — Ctrl+Shift+P"),
                    MenuEntry::Separator,
                    MenuEntry::placeholder("Build — Ctrl+B"),
                    MenuEntry::placeholder("Build With… — Ctrl+Shift+B"),
                    MenuEntry::placeholder("Cancel Build"),
                    MenuEntry::Separator,
                    MenuEntry::placeholder("Record Macro"),
                    MenuEntry::placeholder("Stop Recording Macro"),
                    MenuEntry::placeholder("Playback Macro — Ctrl+Shift+Q"),
                    MenuEntry::placeholder("Save Macro…"),
                    MenuEntry::Separator,
                    MenuEntry::action("Reload Config", MenuAction::ReloadConfig),
                    MenuEntry::action("Performance Report", MenuAction::PerformanceReport),
                ],
            ),
            Menu::Project => (
                "Project",
                vec![
                    MenuEntry::placeholder("Open Project…"),
                    MenuEntry::placeholder("Switch Project ▸"),
                    MenuEntry::placeholder("Quick Switch Project… — Ctrl+Alt+P"),
                    MenuEntry::Separator,
                    MenuEntry::placeholder("Save Project As…"),
                    MenuEntry::placeholder("Close Project"),
                    MenuEntry::placeholder("Edit Project"),
                    MenuEntry::Separator,
                    MenuEntry::placeholder("Add Folder to Project…"),
                    MenuEntry::placeholder("Remove Folder from Project"),
                ],
            ),
            Menu::Preferences => (
                "Preferences",
                vec![
                    MenuEntry::placeholder("Browse Packages…"),
                    MenuEntry::placeholder("Browse Cache…"),
                    MenuEntry::Separator,
                    MenuEntry::placeholder("Settings"),
                    MenuEntry::placeholder("Settings — Syntax Specific"),
                    MenuEntry::placeholder("Settings — Distraction Free"),
                    MenuEntry::Separator,
                    MenuEntry::placeholder("Key Bindings"),
                    MenuEntry::placeholder("Mouse Bindings"),
                    MenuEntry::placeholder("Menu"),
                    MenuEntry::placeholder("Macros"),
                    MenuEntry::placeholder("Commands"),
                    MenuEntry::Separator,
                    MenuEntry::placeholder("Package Settings ▸"),
                    MenuEntry::placeholder("Select Color Scheme ▸"),
                    MenuEntry::placeholder("Select Theme ▸"),
                ],
            ),
            Menu::Help => (
                "Help",
                vec![
                    MenuEntry::placeholder("Documentation"),
                    MenuEntry::placeholder("Report a Bug"),
                    MenuEntry::placeholder("Support Roxanne"),
                    MenuEntry::Separator,
                    MenuEntry::placeholder("License"),
                    MenuEntry::action("About", MenuAction::About),
                ],
            ),
        };

        let row = column![
            text(label)
                .size(12)
                .font(Font::MONOSPACE)
                .style(Color::from_rgb8(180, 180, 180)),
            Column::with_children(
                actions
                    .into_iter()
                    .map(|entry| match entry {
                        MenuEntry::Action(name, action) => Button::new(
                            text(name).size(12).font(Font::MONOSPACE),
                        )
                        .width(Length::Shrink)
                        .padding([2, 8])
                        .style(theme::Button::Custom(Box::new(SubmenuButtonStyle {
                            palette: self.theme,
                        })))
                        .on_press(Message::MenuAction(action))
                        .into(),
                        MenuEntry::Separator => self.submenu_separator(),
                    })
                    .collect::<Vec<Element<Message>>>(),
            )
            .spacing(6)
            .width(Length::Shrink),
        ]
        .spacing(8)
        .align_items(Alignment::Start)
        .padding([8, 16]);

        Some(
            Container::new(row)
                .width(Length::Shrink)
                .style(theme::Container::Custom(Box::new(SubmenuStyle {
                    palette: self.theme,
                })))
                .into(),
        )
    }

    fn submenu_separator(&self) -> Element<'_, Message> {
        Container::new(Space::with_height(Length::Fixed(1.0)))
            .width(Length::Shrink)
            .style(theme::Container::Custom(Box::new(SubmenuSeparatorStyle {
                palette: self.theme,
            })))
            .into()
    }

    fn tab_bar(&self) -> Element<'_, Message> {
        let is_modified = self.content.text() != self.last_saved_text;
        let filename = if self.filename.trim().is_empty() {
            "untitled.txt"
        } else {
            self.filename.as_str()
        };
        let tab = row![
            text(filename)
                .size(13)
                .font(Font::MONOSPACE)
                .style(Color::from_rgb8(230, 230, 230)),
            text(if is_modified { "●" } else { "×" })
                .size(12)
                .font(Font::MONOSPACE)
                .style(Color::from_rgb8(180, 180, 180)),
        ]
        .spacing(8)
        .padding([6, 12])
        .align_items(Alignment::Center);

        let row = row![
            Container::new(tab)
                .style(theme::Container::Custom(Box::new(ActiveTabStyle {
                    palette: self.theme,
                })))
                .height(Length::Fill)
        ]
        .spacing(4)
        .padding([0, 8])
        .align_items(Alignment::Center);

        Container::new(row)
            .width(Length::Fill)
            .height(Length::Fixed(32.0))
            .style(theme::Container::Custom(Box::new(TabBarStyle {
                palette: self.theme,
            })))
            .into()
    }

    fn editor_area(&self) -> Element<'_, Message> {
        let line_height = EDITOR_LINE_HEIGHT.max(1.0);
        let vertical_padding = EDITOR_VERTICAL_PADDING;
        let line_number_size = 14;
        let line_count = self.buffer.line_count().max(1);
        let gutter_digits = line_count.to_string().len();
        let gutter_width = (gutter_digits as f32 * (line_number_size as f32 * 0.6)) + 24.0;
        let (start_line, _) = self.viewport_cache.range().unwrap_or((0, 0));
        let visible_lines = self.viewport_height.max(1);
        let end_line = (start_line + visible_lines).min(line_count);
        let mut gutter_children = (start_line..end_line)
            .map(|line_index| {
                text(format!("{:>width$}", line_index + 1, width = gutter_digits))
                    .size(line_number_size)
                    .font(Font::MONOSPACE)
                    .style(Color::from_rgb8(140, 140, 140))
                    .line_height(LineHeight::Absolute(line_height.into()))
                    .horizontal_alignment(Horizontal::Right)
                    .into()
            })
            .collect::<Vec<Element<Message>>>();
        gutter_children.push(Space::with_height(Length::Fill).into());

        let gutter_lines = Column::with_children(gutter_children)
            .spacing(0)
            .align_items(Alignment::End)
            .height(Length::Fill);

        let gutter = Container::new(gutter_lines)
            .width(Length::Fixed(gutter_width))
            .height(Length::Fill)
            .padding([vertical_padding, 8.0])
            .style(theme::Container::Custom(Box::new(GutterStyle {
                palette: self.theme,
            })));

        let editor = text_editor(&self.content)
            .on_action(Message::Edit)
            .font(Font::MONOSPACE)
            .line_height(LineHeight::Absolute(line_height.into()))
            .padding([vertical_padding, 16.0])
            .height(Length::Fill)
            .highlight::<highlight::RoxanneHighlighter>(
                self.highlight_settings.clone(),
                highlight::highlight_format,
            );

        let editor = Container::new(editor)
            .width(Length::Fill)
            .height(Length::Fill)
            .id(editor_container_id())
            .style(theme::Container::Custom(Box::new(EditorStyle {
                palette: self.theme,
            })));

        row![gutter, editor].height(Length::Fill).into()
    }

    fn search_panel(&self) -> Option<Element<'_, Message>> {
        if !self.search_panel_open {
            return None;
        }

        let header = row![
            text("Recherche avancée")
                .size(12)
                .font(Font::MONOSPACE)
                .style(Color::from_rgb8(220, 220, 220)),
            text(format!("{} résultat(s)", self.search_results.len()))
                .size(12)
                .font(Font::MONOSPACE)
                .style(Color::from_rgb8(160, 160, 160)),
        ]
        .spacing(12)
        .align_items(Alignment::Center);

        let query_row = row![
            TextInput::new("Recherche globale…", &self.search_query)
                .on_input(Message::SearchChanged)
                .padding([4, 8])
                .size(12),
            self.toggle_button(
                "Aa",
                self.search_case_sensitive,
                Message::SearchToggleCaseSensitive
            ),
            self.toggle_button(".*", self.search_regex, Message::SearchToggleRegex),
            self.toggle_button(
                "Cachés",
                self.search_include_hidden,
                Message::SearchToggleIncludeHidden
            ),
        ]
        .spacing(8)
        .align_items(Alignment::Center);

        let scope_row = row![
            self.search_scope_button(SearchScope::CurrentFile),
            self.search_scope_button(SearchScope::Workspace),
        ]
        .spacing(8)
        .align_items(Alignment::Center);

        let action_row = row![
            Button::new(text("Rechercher fichiers").size(12).font(Font::MONOSPACE))
                .padding([4, 10])
                .style(theme::Button::Custom(Box::new(SubmenuButtonStyle {
                    palette: self.theme,
                })))
                .on_press(Message::SearchInFiles),
            Button::new(text("Effacer résultats").size(12).font(Font::MONOSPACE))
                .padding([4, 10])
                .style(theme::Button::Custom(Box::new(SubmenuButtonStyle {
                    palette: self.theme,
                })))
                .on_press(Message::SearchResultsCleared),
        ]
        .spacing(8)
        .align_items(Alignment::Center);

        let results: Element<Message> = if self.search_results.is_empty() {
            text("Aucun résultat. Lancez une recherche pour le workspace.")
                .size(12)
                .font(Font::MONOSPACE)
                .style(theme::Text::Color(Color::from_rgb8(150, 150, 150)))
                .into()
        } else {
            let entries = self
                .search_results
                .iter()
                .enumerate()
                .map(|(index, result)| {
                    let title = text(format!(
                        "{}:{}:{}",
                        result.path.display(),
                        result.line + 1,
                        result.column + 1
                    ))
                    .size(12)
                    .font(Font::MONOSPACE)
                    .style(Color::from_rgb8(200, 200, 200));

                    let preview = text(result.preview.trim())
                        .size(12)
                        .font(Font::MONOSPACE)
                        .style(Color::from_rgb8(160, 160, 160));

                    Button::new(
                        Container::new(column![title, preview].spacing(2))
                            .padding([4, 12])
                            .style(theme::Container::Custom(Box::new(SearchResultStyle {
                                palette: self.theme,
                            }))),
                    )
                    .style(theme::Button::Custom(Box::new(SearchResultButtonStyle {
                        palette: self.theme,
                    })))
                    .on_press(Message::SearchResultSelected(index))
                    .into()
                })
                .collect::<Vec<Element<Message>>>();

            Scrollable::new(column(entries).spacing(4))
                .height(Length::Fixed(180.0))
                .into()
        };

        let panel = column![header, query_row, scope_row, action_row, results]
            .spacing(10)
            .padding([8, 16]);

        Some(
            Container::new(panel)
                .width(Length::Fill)
                .style(theme::Container::Custom(Box::new(SearchPanelStyle {
                    palette: self.theme,
                })))
                .into(),
        )
    }

    fn goto_panel(&self) -> Option<Element<'_, Message>> {
        if !self.goto_panel_open {
            return None;
        }

        let header = row![
            text("Aller à")
                .size(12)
                .font(Font::MONOSPACE)
                .style(Color::from_rgb8(220, 220, 220)),
            text("Ligne[:Colonne]")
                .size(12)
                .font(Font::MONOSPACE)
                .style(Color::from_rgb8(160, 160, 160)),
        ]
        .spacing(12)
        .align_items(Alignment::Center);

        let input = TextInput::new("ex: 42:5", &self.goto_line_input)
            .on_input(Message::GotoLineChanged)
            .on_submit(Message::GotoLineSubmit)
            .padding([4, 8])
            .size(12);

        let actions = row![
            Button::new(text("Aller").size(12).font(Font::MONOSPACE))
                .padding([4, 10])
                .style(theme::Button::Custom(Box::new(SubmenuButtonStyle {
                    palette: self.theme,
                })))
                .on_press(Message::GotoLineSubmit),
            Button::new(text("Fermer").size(12).font(Font::MONOSPACE))
                .padding([4, 10])
                .style(theme::Button::Custom(Box::new(SubmenuButtonStyle {
                    palette: self.theme,
                })))
                .on_press(Message::GotoLineClosed),
        ]
        .spacing(8)
        .align_items(Alignment::Center);

        let panel = column![header, input, actions].spacing(10).padding([8, 16]);

        Some(
            Container::new(panel)
                .width(Length::Fill)
                .style(theme::Container::Custom(Box::new(SearchPanelStyle {
                    palette: self.theme,
                })))
                .into(),
        )
    }

    fn diagnostics_panel(&self) -> Option<Element<'_, Message>> {
        if !self.diagnostics_panel_open {
            return None;
        }

        let header = row![
            text("Diagnostics LSP")
                .size(12)
                .font(Font::MONOSPACE)
                .style(Color::from_rgb8(220, 220, 220)),
            text(format!("{} élément(s)", self.diagnostics.len()))
                .size(12)
                .font(Font::MONOSPACE)
                .style(Color::from_rgb8(160, 160, 160)),
        ]
        .spacing(12)
        .align_items(Alignment::Center);

        let content: Element<Message> = if self.diagnostics.is_empty() {
            text("Aucun diagnostic.")
                .size(12)
                .font(Font::MONOSPACE)
                .style(theme::Text::Color(Color::from_rgb8(150, 150, 150)))
                .into()
        } else {
            let entries = self
                .diagnostics
                .iter()
                .map(|diagnostic| {
                    let severity_color = match diagnostic.severity {
                        DiagnosticSeverity::Error => Color::from_rgb8(242, 94, 92),
                        DiagnosticSeverity::Warning => Color::from_rgb8(240, 200, 120),
                    };
                    let title = text(format!("{}:{}", diagnostic.line + 1, diagnostic.column + 1))
                        .size(12)
                        .font(Font::MONOSPACE)
                        .style(severity_color);

                    let message = text(&diagnostic.message)
                        .size(12)
                        .font(Font::MONOSPACE)
                        .style(Color::from_rgb8(180, 180, 180));

                    Container::new(column![title, message].spacing(2))
                        .padding([4, 12])
                        .style(theme::Container::Custom(Box::new(SearchResultStyle {
                            palette: self.theme,
                        })))
                        .into()
                })
                .collect::<Vec<Element<Message>>>();

            Scrollable::new(column(entries).spacing(4))
                .height(Length::Fixed(160.0))
                .into()
        };

        let panel = column![header, content].spacing(10).padding([8, 16]);

        Some(
            Container::new(panel)
                .width(Length::Fill)
                .style(theme::Container::Custom(Box::new(SearchPanelStyle {
                    palette: self.theme,
                })))
                .into(),
        )
    }

    fn completion_panel(&self) -> Option<Element<'_, Message>> {
        if !self.completion_panel_open {
            return None;
        }

        let header = row![
            text("Complétions LSP")
                .size(12)
                .font(Font::MONOSPACE)
                .style(Color::from_rgb8(220, 220, 220)),
            text(format!("Préfixe: {}", self.completion_prefix))
                .size(12)
                .font(Font::MONOSPACE)
                .style(Color::from_rgb8(160, 160, 160)),
        ]
        .spacing(12)
        .align_items(Alignment::Center);

        let list: Element<Message> = if self.completion_items.is_empty() {
            text("Aucune suggestion.")
                .size(12)
                .font(Font::MONOSPACE)
                .style(theme::Text::Color(Color::from_rgb8(150, 150, 150)))
                .into()
        } else {
            let entries = self
                .completion_items
                .iter()
                .enumerate()
                .map(|(index, item)| {
                    let label = text(&item.label)
                        .size(12)
                        .font(Font::MONOSPACE)
                        .style(Color::from_rgb8(200, 200, 200));
                    let detail = text(&item.detail)
                        .size(12)
                        .font(Font::MONOSPACE)
                        .style(Color::from_rgb8(150, 150, 150));

                    Button::new(
                        Container::new(column![label, detail].spacing(2))
                            .padding([4, 12])
                            .style(theme::Container::Custom(Box::new(SearchResultStyle {
                                palette: self.theme,
                            }))),
                    )
                    .style(theme::Button::Custom(Box::new(SearchResultButtonStyle {
                        palette: self.theme,
                    })))
                    .on_press(Message::CompletionSelected(index))
                    .into()
                })
                .collect::<Vec<Element<Message>>>();

            Scrollable::new(column(entries).spacing(4))
                .height(Length::Fixed(160.0))
                .into()
        };

        let panel = column![header, list].spacing(10).padding([8, 16]);

        Some(
            Container::new(panel)
                .width(Length::Fill)
                .style(theme::Container::Custom(Box::new(SearchPanelStyle {
                    palette: self.theme,
                })))
                .into(),
        )
    }

    fn status_bar(&self) -> Element<'_, Message> {
        let is_modified = self.content.text() != self.last_saved_text;
        let has_matches = !self.search_matches.is_empty();
        let cursor_position = self.content.cursor_position();
        let cursor_count = self.multi_cursors.len() + 1;
        let diagnostics_count = self.diagnostics.len();
        let left = row![
            text(format!(
                "{}{}",
                self.filename,
                if is_modified { " ●" } else { "" }
            ))
            .size(12)
            .font(Font::MONOSPACE)
            .style(Color::from_rgb8(200, 200, 200)),
            TextInput::new("search…", &self.search_query)
                .on_input(Message::SearchChanged)
                .padding([2, 8])
                .size(12),
            self.toggle_button(
                "Aa",
                self.search_case_sensitive,
                Message::SearchToggleCaseSensitive
            ),
            self.toggle_button(".*", self.search_regex, Message::SearchToggleRegex),
            Button::new(text("◀").size(12).font(Font::MONOSPACE))
                .padding([2, 6])
                .style(theme::Button::Custom(Box::new(SubmenuButtonStyle {
                    palette: self.theme,
                })))
                .on_press_maybe(has_matches.then_some(Message::SearchPrevious)),
            Button::new(text("▶").size(12).font(Font::MONOSPACE))
                .padding([2, 6])
                .style(theme::Button::Custom(Box::new(SubmenuButtonStyle {
                    palette: self.theme,
                })))
                .on_press_maybe(has_matches.then_some(Message::SearchNext)),
            TextInput::new("fichier…", &self.filename)
                .on_input(Message::FilenameChanged)
                .padding([2, 8])
                .size(12),
        ]
        .spacing(12)
        .align_items(Alignment::Center);

        let matches = self.search_matches.len();
        let status_text = self
            .status_message
            .clone()
            .unwrap_or_else(|| "Prêt.".to_string());
        let plugin_text = self
            .plugins
            .statuses()
            .into_iter()
            .map(|status| format!("{}: {}", status.label, status.value))
            .collect::<Vec<_>>()
            .join("   ");
        let plugin_segment = if plugin_text.is_empty() {
            String::new()
        } else {
            format!("   {plugin_text}")
        };
        let viewport_label = self.viewport_label();

        let right = text(format!(
            "{}   Mode: {}   Occurrences: {}   Cursors: {}   Diagnostics: {}   Ln {}, Col {}   UTF-8   LF   {}{}",
            viewport_label,
            self.mode.label(),
            matches,
            cursor_count,
            diagnostics_count,
            cursor_position.0 + 1,
            cursor_position.1 + 1,
            status_text,
            plugin_segment
        ))
        .size(12)
        .font(Font::MONOSPACE)
        .style(Color::from_rgb8(200, 200, 200))
        .horizontal_alignment(Horizontal::Right)
        .vertical_alignment(Vertical::Center);

        let row = row![left, right]
            .spacing(16)
            .padding([6, 16])
            .width(Length::Fill)
            .align_items(Alignment::Center);

        Container::new(row)
            .width(Length::Fill)
            .style(theme::Container::Custom(Box::new(StatusBarStyle {
                palette: self.theme,
            })))
            .into()
    }

    fn toggle_button(&self, label: &str, active: bool, message: Message) -> Element<'_, Message> {
        Button::new(text(label).size(12).font(Font::MONOSPACE))
            .padding([2, 6])
            .style(theme::Button::Custom(Box::new(ToggleButtonStyle {
                active,
                palette: self.theme,
            })))
            .on_press(message)
            .into()
    }

    fn search_scope_button(&self, scope: SearchScope) -> Element<'_, Message> {
        let active = self.search_scope == scope;
        Button::new(
            text(scope.label())
                .size(12)
                .font(Font::MONOSPACE)
                .style(Color::from_rgb8(220, 220, 220)),
        )
        .padding([2, 8])
        .style(theme::Button::Custom(Box::new(ToggleButtonStyle {
            active,
            palette: self.theme,
        })))
        .on_press(Message::SearchScopeSelected(scope))
        .into()
    }

    fn reload_config(&mut self) -> Command<Message> {
        let config = config::AppConfig::load();
        let (mut plugins, plugin_warnings) = PluginManager::new(&config.plugins);
        let mut warnings = config.load_warnings.clone();
        warnings.extend(plugin_warnings);

        highlight::set_syntax_palette(config.theme.syntax);
        self.theme = config.theme;
        self.keymap = config.keymap;
        plugins.on_text_changed(&self.content.text(), &self.filename);
        self.plugins = plugins;

        self.status_message = Some(if warnings.is_empty() {
            "Config: rechargée.".to_string()
        } else {
            format!("Config: rechargée avec {} alerte(s).", warnings.len())
        });

        Command::none()
    }

    fn search_options(&self) -> SearchOptions {
        SearchOptions {
            regex: self.search_regex,
            case_sensitive: self.search_case_sensitive,
            include_hidden: self.search_include_hidden,
        }
    }

    fn resolve_workspace_root(&self) -> Result<PathBuf, String> {
        if let Some(root) = Self::workspace_root_from_filename(&self.filename) {
            return Ok(root);
        }
        std::env::current_dir().map_err(|err| err.to_string())
    }

    fn workspace_root_from_filename(filename: &str) -> Option<PathBuf> {
        let trimmed = filename.trim();
        if trimmed.is_empty() {
            return None;
        }

        let path = Path::new(trimmed);
        let base_dir = if path.is_file() {
            path.parent()?
        } else if path.is_dir() {
            path
        } else {
            path.parent()?
        };

        for ancestor in base_dir.ancestors() {
            let candidate = ancestor.join(".roxanne.toml");
            if candidate.is_file() {
                return Some(ancestor.to_path_buf());
            }
        }

        Some(base_dir.to_path_buf())
    }

    fn search_in_files(&mut self) -> Command<Message> {
        if self.search_query.trim().is_empty() {
            self.status_message = Some("Recherche fichiers: saisissez un terme.".to_string());
            return Command::none();
        }

        if self.search_scope == SearchScope::CurrentFile {
            let started = Instant::now();
            self.refresh_search_matches(false, true);
            self.performance.last_search = Some(started.elapsed());
            let duration = self
                .performance
                .last_search
                .map(format_duration)
                .unwrap_or_else(|| "-".to_string());
            self.status_message = Some(format!(
                "Recherche fichier: {} occurrence(s) ({duration}).",
                self.search_matches.len(),
            ));
            return Command::none();
        }

        let query = self.search_query.clone();
        let options = self.search_options();
        let root = match self.resolve_workspace_root() {
            Ok(path) => path,
            Err(err) => {
                self.status_message = Some(format!("Recherche fichiers: {err}"));
                return Command::none();
            }
        };

        self.perf_search_files_started = Some(Instant::now());
        Command::perform(
            search_in_workspace(root, query, options),
            Message::SearchResultsLoaded,
        )
    }

    fn refresh_search_matches(&mut self, preserve_index: bool, update_status: bool) {
        let options = self.search_options();
        self.search_matches = match find_matches(&self.buffer, &self.search_query, options) {
            Ok(matches) => matches,
            Err(message) => {
                self.current_match_index = None;
                self.search_matches.clear();
                self.highlight_settings.search_matches.clear();
                if update_status && !self.search_query.is_empty() {
                    self.status_message = Some(format!("Recherche: {message}"));
                }
                return;
            }
        };
        self.highlight_settings.search_matches = self.search_matches.clone();
        if self.search_matches.is_empty() {
            self.current_match_index = None;
            if update_status && !self.search_query.is_empty() {
                self.status_message = Some("Recherche: aucune occurrence.".to_string());
            }
            return;
        }

        self.current_match_index = if preserve_index {
            self.current_match_index
                .filter(|index| *index < self.search_matches.len())
        } else {
            None
        };

        if update_status && !self.search_query.is_empty() {
            self.status_message = Some(format!(
                "Recherche: {} occurrence(s).",
                self.search_matches.len()
            ));
        }
    }

    fn find_next_match(&mut self, forward: bool) -> Command<Message> {
        if self.search_query.trim().is_empty() {
            self.status_message = Some("Recherche: saisissez un terme.".to_string());
            return Command::none();
        }

        if self.search_matches.is_empty() {
            self.status_message = Some("Recherche: aucune occurrence.".to_string());
            return Command::none();
        }

        let total = self.search_matches.len();
        let next_index = match self.current_match_index {
            Some(index) => {
                if forward {
                    (index + 1) % total
                } else {
                    (index + total - 1) % total
                }
            }
            None => {
                if forward {
                    0
                } else {
                    total - 1
                }
            }
        };

        self.current_match_index = Some(next_index);
        self.jump_to_match(next_index);
        Command::none()
    }

    fn jump_to_match(&mut self, index: usize) {
        if let Some(match_position) = self.search_matches.get(index) {
            let line = match_position.line;
            let column = match_position.column;
            self.jump_to_position(line, column);
            self.status_message = Some(format!(
                "Recherche: {}/{} (ligne {}, colonne {}).",
                index + 1,
                self.search_matches.len(),
                line + 1,
                column + 1
            ));
        }
    }

    fn open_search_result(&mut self, index: usize) -> Command<Message> {
        let Some(result) = self.search_results.get(index).cloned() else {
            return Command::none();
        };

        let path = result.path.clone();
        Command::perform(
            async move {
                let metadata = std::fs::metadata(&path).map_err(|err| err.to_string())?;
                if metadata.len() > MAX_OPEN_FILE_SIZE {
                    return Err(format!(
                        "fichier trop volumineux ({} octets, limite {} octets)",
                        metadata.len(),
                        MAX_OPEN_FILE_SIZE
                    ));
                }
                let bytes = std::fs::read(&path).map_err(|err| err.to_string())?;
                let lossy_text = String::from_utf8_lossy(&bytes);
                let lossy = matches!(lossy_text, Cow::Owned(_));
                Ok(SearchResultLoadResult {
                    text: lossy_text.into_owned(),
                    result,
                    lossy,
                })
            },
            Message::SearchResultOpened,
        )
    }

    fn jump_to_position(&mut self, line: usize, column: usize) {
        let max_line = self.buffer.line_count().saturating_sub(1);
        let clamped_line = line.min(max_line);
        let clamped_column = self
            .buffer
            .line(clamped_line)
            .map(|line| line.chars().count())
            .unwrap_or(0)
            .min(column);
        self.move_cursor_to(clamped_line, clamped_column);
    }

    fn move_cursor_to(&mut self, line: usize, column: usize) {
        self.content
            .perform(EditorAction::Move(Motion::DocumentStart));
        for _ in 0..line {
            self.content.perform(EditorAction::Move(Motion::Down));
        }
        for _ in 0..column {
            self.content.perform(EditorAction::Move(Motion::Right));
        }
        self.refresh_viewport_cache();
    }

    fn handle_goto_line(&mut self) {
        let input = self.goto_line_input.trim();
        if input.is_empty() {
            self.status_message = Some("Aller à: saisissez une ligne.".to_string());
            return;
        }

        match parse_goto_input(input) {
            Ok((line, column)) => {
                self.jump_to_position(line, column);
                self.status_message = Some(format!(
                    "Aller à: ligne {}, colonne {}.",
                    line + 1,
                    column + 1
                ));
                self.goto_panel_open = false;
            }
            Err(message) => {
                self.status_message = Some(format!("Aller à: {message}"));
            }
        }
    }

    fn add_cursor_next_match(&mut self) {
        if self.search_matches.is_empty() {
            self.status_message = Some("Multi-curseurs: aucune occurrence.".to_string());
            return;
        }

        let (line, column) = self.content.cursor_position();
        let next_match = self
            .search_matches
            .iter()
            .find(|match_position| {
                match_position.line > line
                    || (match_position.line == line && match_position.column > column)
            })
            .or_else(|| self.search_matches.first());

        if let Some(match_position) = next_match {
            let position = Position::new(match_position.line, match_position.column);
            if !self.multi_cursors.contains(&position) {
                self.multi_cursors.push(position);
            }
            self.status_message = Some("Multi-curseurs: ajout d'une position.".to_string());
        }
    }

    fn add_cursors_all_matches(&mut self) {
        if self.search_matches.is_empty() {
            self.status_message = Some("Multi-curseurs: aucune occurrence.".to_string());
            return;
        }
        let (line, column) = self.content.cursor_position();
        let primary = Position::new(line, column);
        self.multi_cursors = self
            .search_matches
            .iter()
            .map(|match_position| Position::new(match_position.line, match_position.column))
            .filter(|position| *position != primary)
            .collect();
        self.status_message = Some(format!(
            "Multi-curseurs: {} position(s).",
            self.multi_cursors.len()
        ));
    }

    fn apply_multi_cursor_edit(&mut self, edit: &EditorEdit) {
        let (line, column) = self.content.cursor_position();
        let mut cursor_positions = Vec::with_capacity(self.multi_cursors.len() + 1);
        cursor_positions.push(Position::new(line, column));
        cursor_positions.extend(self.multi_cursors.iter().copied());

        let mut seen_indices = HashSet::new();
        let mut cursor_slots = Vec::new();
        for (id, position) in cursor_positions.iter().copied().enumerate() {
            let index = self.buffer.index_from_position(position);
            if seen_indices.insert(index) {
                cursor_slots.push(CursorSlot {
                    id,
                    _position: position,
                    index,
                });
            }
        }

        cursor_slots.sort_by_key(|slot| slot.index);
        let text = self.buffer.text();

        let mut operations = Vec::new();
        for slot in &cursor_slots {
            if let Some(operation) = EditOperation::from_edit(slot, edit, &text) {
                operations.push(operation);
            }
        }

        operations
            .sort_by(|left, right| left.start.cmp(&right.start).then(left.end.cmp(&right.end)));

        let mut filtered = Vec::new();
        let mut ignored_operations = 0;
        let mut last_end = 0;
        for operation in operations {
            if operation.start < last_end {
                ignored_operations += 1;
                continue;
            }
            last_end = operation.end;
            filtered.push(operation);
        }

        let new_text = apply_operations(&text, &filtered);
        self.buffer.replace(&new_text);
        self.content = EditorContent::with_text(&self.buffer.text());
        self.sync_highlight_buffer();

        let new_indices = compute_new_indices(&cursor_slots, &filtered);
        let mut new_positions = Vec::new();
        for (index, position) in cursor_positions.iter().enumerate() {
            let new_index = new_indices
                .get(&index)
                .copied()
                .unwrap_or_else(|| self.buffer.index_from_position(*position));
            new_positions.push(self.buffer.position_from_index(new_index));
        }

        if let Some(primary) = new_positions.first() {
            self.move_cursor_to(primary.line, primary.column);
        }
        self.multi_cursors = new_positions.into_iter().skip(1).collect();
        let applied_cursors = self.multi_cursors.len() + 1;
        if ignored_operations > 0 {
            self.status_message = Some(format!(
                "Multi-curseurs: édition sur {applied_cursors} curseur(s). \
{ignored_operations} curseur(s) ignoré(s) car leurs edits se chevauchent."
            ));
        } else {
            self.status_message = Some(format!(
                "Multi-curseurs: édition sur {applied_cursors} curseur(s)."
            ));
        }
    }

    fn record_undo_snapshot(&mut self) {
        if self.suppress_undo_snapshot {
            return;
        }
        self.buffer.record_snapshot();
    }

    fn apply_undo(&mut self) {
        if !self.buffer.undo() {
            return;
        };
        let text = self.buffer.text();
        self.apply_snapshot(text);
    }

    fn apply_redo(&mut self) {
        if !self.buffer.redo() {
            return;
        };
        let text = self.buffer.text();
        self.apply_snapshot(text);
    }

    fn apply_snapshot(&mut self, text: String) {
        self.suppress_undo_snapshot = true;
        self.content = EditorContent::with_text(&text);
        self.buffer.replace(&text);
        self.sync_highlight_buffer();
        self.refresh_search_matches(true, true);
        self.refresh_diagnostics();
        self.plugins
            .on_text_changed(&self.content.text(), &self.filename);
        self.refresh_viewport_cache();
        self.suppress_undo_snapshot = false;
    }

    fn sync_highlight_buffer(&mut self) {
        self.highlight_settings.buffer_text = self.content.text().into();
    }

    fn refresh_viewport_cache(&mut self) {
        let (line, _) = self.content.cursor_position();
        let start_line = line.saturating_sub(self.viewport_height / 2);
        let started = Instant::now();
        self.viewport_cache
            .update(&self.buffer, start_line, self.viewport_height);
        self.performance.last_viewport_refresh = Some(started.elapsed());
    }

    fn viewport_label(&self) -> String {
        match self.viewport_cache.range() {
            Some((start, end)) => format!("Viewport: {}-{}", start + 1, end),
            None => "Viewport: -".to_string(),
        }
    }

    fn refresh_diagnostics(&mut self) {
        self.diagnostics = analyze_diagnostics(&self.buffer);
    }

    fn refresh_completions(&mut self) {
        let (line, column) = self.content.cursor_position();
        let line_text = self.buffer.line(line).unwrap_or("");
        let column = column.min(line_text.chars().count());
        let prefix = extract_prefix(line_text, column);
        self.completion_prefix = prefix.clone();
        self.completion_items = build_completion_items(&prefix);
        if self.completion_items.is_empty() {
            self.status_message = Some("Complétions: aucune suggestion.".to_string());
        } else {
            self.status_message = Some(format!(
                "Complétions: {} suggestion(s).",
                self.completion_items.len()
            ));
        }
    }

    fn apply_completion(&mut self, index: usize) {
        let Some(item) = self.completion_items.get(index) else {
            return;
        };

        let remainder = item
            .label
            .strip_prefix(&self.completion_prefix)
            .unwrap_or(&item.label);
        if remainder.is_empty() {
            return;
        }

        let insert = EditorEdit::Paste(std::sync::Arc::new(remainder.to_string()));
        if self.multi_cursors.is_empty() {
            self.record_undo_snapshot();
            self.content.perform(EditorAction::Edit(insert.clone()));
            self.buffer.replace(&self.content.text());
        } else {
            self.record_undo_snapshot();
            self.apply_multi_cursor_edit(&insert);
        }
        self.sync_highlight_buffer();
        self.refresh_search_matches(true, true);
        self.refresh_diagnostics();
        self.plugins
            .on_text_changed(&self.content.text(), &self.filename);
        self.refresh_viewport_cache();
        self.completion_panel_open = false;
        self.completion_items.clear();
    }
}

struct AppBackground {
    palette: ThemePalette,
}

impl container::StyleSheet for AppBackground {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(self.palette.app_background)),
            text_color: None,
            border: Default::default(),
            shadow: Default::default(),
        }
    }
}

struct MenuBarStyle {
    palette: ThemePalette,
}

impl container::StyleSheet for MenuBarStyle {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(self.palette.menu_bar)),
            text_color: None,
            border: Default::default(),
            shadow: Default::default(),
        }
    }
}

struct MenuButtonStyle {
    active: bool,
    palette: ThemePalette,
}

impl button::StyleSheet for MenuButtonStyle {
    type Style = Theme;

    fn active(&self, _style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: self
                .active
                .then(|| Background::Color(self.palette.menu_button_active)),
            text_color: Color::from_rgb8(220, 220, 220),
            border: Default::default(),
            shadow_offset: Default::default(),
            shadow: Default::default(),
        }
    }

    fn hovered(&self, style: &Self::Style) -> button::Appearance {
        let mut appearance = self.active(style);
        appearance.background = Some(Background::Color(self.palette.menu_button_hover));
        appearance
    }
}

struct SubmenuStyle {
    palette: ThemePalette,
}

impl container::StyleSheet for SubmenuStyle {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(self.palette.submenu_bar)),
            text_color: None,
            border: Default::default(),
            shadow: Default::default(),
        }
    }
}

struct SubmenuSeparatorStyle {
    palette: ThemePalette,
}

impl container::StyleSheet for SubmenuSeparatorStyle {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(self.palette.menu_button_hover)),
            text_color: None,
            border: Default::default(),
            shadow: Default::default(),
        }
    }
}

struct SubmenuButtonStyle {
    palette: ThemePalette,
}

impl button::StyleSheet for SubmenuButtonStyle {
    type Style = Theme;

    fn active(&self, _style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: None,
            text_color: Color::from_rgb8(230, 230, 230),
            border: Default::default(),
            shadow_offset: Default::default(),
            shadow: Default::default(),
        }
    }

    fn hovered(&self, style: &Self::Style) -> button::Appearance {
        let mut appearance = self.active(style);
        appearance.background = Some(Background::Color(self.palette.button_hover));
        appearance
    }
}

struct ToggleButtonStyle {
    active: bool,
    palette: ThemePalette,
}

impl button::StyleSheet for ToggleButtonStyle {
    type Style = Theme;

    fn active(&self, _style: &Self::Style) -> button::Appearance {
        let background = if self.active {
            Background::Color(self.palette.toggle_active)
        } else {
            Background::Color(self.palette.toggle_inactive)
        };

        button::Appearance {
            background: Some(background),
            text_color: Color::from_rgb8(230, 230, 230),
            border: Default::default(),
            shadow_offset: Default::default(),
            shadow: Default::default(),
        }
    }

    fn hovered(&self, style: &Self::Style) -> button::Appearance {
        let mut appearance = self.active(style);
        appearance.background = Some(Background::Color(self.palette.button_hover));
        appearance
    }
}

struct TabBarStyle {
    palette: ThemePalette,
}

impl container::StyleSheet for TabBarStyle {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(self.palette.tab_bar)),
            text_color: None,
            border: Default::default(),
            shadow: Default::default(),
        }
    }
}

struct ActiveTabStyle {
    palette: ThemePalette,
}

impl container::StyleSheet for ActiveTabStyle {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(self.palette.tab_active)),
            text_color: None,
            border: Default::default(),
            shadow: Default::default(),
        }
    }
}

struct EditorStyle {
    palette: ThemePalette,
}

impl container::StyleSheet for EditorStyle {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(self.palette.editor_background)),
            text_color: None,
            border: Default::default(),
            shadow: Default::default(),
        }
    }
}

struct GutterStyle {
    palette: ThemePalette,
}

impl container::StyleSheet for GutterStyle {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(self.palette.panel_background)),
            text_color: None,
            border: Default::default(),
            shadow: Default::default(),
        }
    }
}

struct StatusBarStyle {
    palette: ThemePalette,
}

impl container::StyleSheet for StatusBarStyle {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(self.palette.status_bar)),
            text_color: None,
            border: Default::default(),
            shadow: Default::default(),
        }
    }
}

struct SearchPanelStyle {
    palette: ThemePalette,
}

impl container::StyleSheet for SearchPanelStyle {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(self.palette.panel_background)),
            text_color: None,
            border: Default::default(),
            shadow: Default::default(),
        }
    }
}

struct SearchResultStyle {
    palette: ThemePalette,
}

impl container::StyleSheet for SearchResultStyle {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(self.palette.panel_item_background)),
            text_color: None,
            border: Default::default(),
            shadow: Default::default(),
        }
    }
}

struct SearchResultButtonStyle {
    palette: ThemePalette,
}

impl button::StyleSheet for SearchResultButtonStyle {
    type Style = Theme;

    fn active(&self, _style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: None,
            text_color: Color::WHITE,
            border: Default::default(),
            shadow_offset: Default::default(),
            shadow: Default::default(),
        }
    }

    fn hovered(&self, style: &Self::Style) -> button::Appearance {
        let mut appearance = self.active(style);
        appearance.background = Some(Background::Color(self.palette.panel_item_hover));
        appearance
    }
}

fn find_matches(
    buffer: &TextBuffer,
    needle: &str,
    options: SearchOptions,
) -> Result<Vec<MatchPosition>, String> {
    if needle.trim().is_empty() {
        return Ok(Vec::new());
    }

    let matcher = build_matcher(needle, options)?;
    let mut matches = Vec::new();
    for (line_index, line) in buffer.lines().enumerate() {
        for (column, length) in find_matches_in_line(line, &matcher) {
            matches.push(MatchPosition {
                line: line_index,
                column,
                length,
            });
        }
    }

    Ok(matches)
}

const MAX_OPEN_FILE_SIZE: u64 = 5 * 1024 * 1024;

async fn search_in_workspace(
    root: PathBuf,
    query: String,
    options: SearchOptions,
) -> Result<SearchResultsSummary, String> {
    if query.trim().is_empty() {
        return Ok(SearchResultsSummary {
            results: Vec::new(),
            skipped_read_errors: 0,
            skipped_too_large: 0,
            skipped_invalid_utf8: 0,
            truncated: false,
        });
    }

    let matcher = build_matcher(&query, options)?;
    let mut results = Vec::new();
    let mut collected = 0usize;
    let max_results = 500usize;
    let mut skipped_read_errors = 0usize;
    let mut skipped_too_large = 0usize;
    let mut skipped_invalid_utf8 = 0usize;
    let mut truncated = false;

    for entry in WalkDir::new(&root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| !should_skip_entry(entry, options.include_hidden))
    {
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => {
                skipped_read_errors += 1;
                continue;
            }
        };
        if !entry.file_type().is_file() {
            continue;
        }

        if should_skip_file(entry.path(), options.include_hidden) {
            continue;
        }

        let metadata = match entry.metadata() {
            Ok(metadata) => metadata,
            Err(_) => {
                skipped_read_errors += 1;
                continue;
            }
        };
        if metadata.len() > MAX_OPEN_FILE_SIZE {
            skipped_too_large += 1;
            continue;
        }

        let contents = match std::fs::read_to_string(entry.path()) {
            Ok(contents) => contents,
            Err(err) => {
                if err.kind() == std::io::ErrorKind::InvalidData {
                    skipped_invalid_utf8 += 1;
                } else {
                    skipped_read_errors += 1;
                }
                continue;
            }
        };

        for line_match in find_matches_in_text(&contents, &matcher) {
            results.push(SearchResult {
                path: entry.path().to_path_buf(),
                line: line_match.line,
                column: line_match.column,
                preview: line_match.preview,
            });
            collected += 1;
            if collected >= max_results {
                truncated = true;
                return Ok(SearchResultsSummary {
                    results,
                    skipped_read_errors,
                    skipped_too_large,
                    skipped_invalid_utf8,
                    truncated,
                });
            }
        }
    }

    Ok(SearchResultsSummary {
        results,
        skipped_read_errors,
        skipped_too_large,
        skipped_invalid_utf8,
        truncated,
    })
}

fn build_matcher(needle: &str, options: SearchOptions) -> Result<SearchMatcher, String> {
    if options.regex {
        RegexBuilder::new(needle)
            .case_insensitive(!options.case_sensitive)
            .build()
            .map(SearchMatcher::Regex)
            .map_err(|err| format!("regex invalide ({err})"))
    } else {
        Ok(SearchMatcher::Plain {
            needle: needle.to_string(),
            case_sensitive: options.case_sensitive,
        })
    }
}

fn normalize_casefolded(text: &str) -> String {
    text.chars().flat_map(|ch| ch.to_lowercase()).collect()
}

fn normalize_casefolded_with_mapping(text: &str) -> (String, Vec<usize>) {
    let mut normalized = String::new();
    let mut mapping = Vec::new();
    for (index, ch) in text.chars().enumerate() {
        for folded in ch.to_lowercase() {
            normalized.push(folded);
            mapping.push(index);
        }
    }
    (normalized, mapping)
}

fn find_matches_in_line(line: &str, matcher: &SearchMatcher) -> Vec<(usize, usize)> {
    match matcher {
        SearchMatcher::Regex(regex) => regex
            .find_iter(line)
            .map(|found| {
                let start = byte_index_to_char_index(line, found.start());
                let length = line[found.start()..found.end()].chars().count();
                (start, length)
            })
            .collect(),
        SearchMatcher::Plain {
            needle,
            case_sensitive,
        } => {
            if needle.is_empty() {
                return Vec::new();
            }

            if !case_sensitive {
                let normalized_needle = normalize_casefolded(needle);
                if normalized_needle.is_empty() {
                    return Vec::new();
                }
                let (normalized_line, mapping) = normalize_casefolded_with_mapping(line);
                let mut matches = Vec::new();
                let mut search_start = 0;
                while let Some(found) = normalized_line[search_start..].find(&normalized_needle) {
                    let start_byte = search_start + found;
                    let end_byte = start_byte + normalized_needle.len();
                    let start_index = byte_index_to_char_index(&normalized_line, start_byte);
                    let end_index = byte_index_to_char_index(&normalized_line, end_byte);
                    if end_index == 0 {
                        break;
                    }
                    let start_original = mapping.get(start_index).copied().unwrap_or_default();
                    let end_original = mapping
                        .get(end_index.saturating_sub(1))
                        .copied()
                        .map(|index| index + 1)
                        .unwrap_or(start_original);
                    let length = end_original.saturating_sub(start_original);
                    matches.push((start_original, length));
                    search_start = end_byte;
                }
                return matches;
            }

            let mut matches = Vec::new();
            let needle_len = needle.len();
            let needle_chars = needle.chars().count();
            let mut search_start = 0;
            while let Some(found) = line[search_start..].find(needle) {
                let byte_index = search_start + found;
                let column = byte_index_to_char_index(line, byte_index);
                matches.push((column, needle_chars));
                search_start = byte_index + needle_len;
            }

            matches
        }
    }
}

fn find_matches_in_text(text: &str, matcher: &SearchMatcher) -> Vec<SearchResultLine> {
    let mut matches = Vec::new();
    for (line_index, line) in text.lines().enumerate() {
        for (column, length) in find_matches_in_line(line, matcher) {
            matches.push(SearchResultLine {
                line: line_index,
                column,
                _length: length,
                preview: line.to_string(),
            });
        }
    }

    matches
}

fn should_skip_entry(entry: &DirEntry, include_hidden: bool) -> bool {
    if entry.depth() == 0 {
        return false;
    }
    let name = entry.file_name().to_string_lossy();
    let skip_dirs = [".git", "target", "node_modules", "dist", "build", "out"];
    if entry.file_type().is_dir() && skip_dirs.contains(&name.as_ref()) {
        return true;
    }
    if include_hidden {
        return false;
    }
    name.starts_with('.')
}

fn should_skip_file(path: &Path, include_hidden: bool) -> bool {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");
    if file_name.starts_with('.') && !include_hidden {
        return true;
    }

    let skip_extensions = [
        "png", "jpg", "jpeg", "gif", "svg", "ico", "zip", "tar", "gz", "pdf", "mp4", "mp3",
    ];
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| skip_extensions.contains(&ext))
        .unwrap_or(false)
}

#[cfg(test)]
fn rename_failure_state() -> &'static Mutex<Option<PathBuf>> {
    static RENAME_FAILURE_TARGET: OnceLock<Mutex<Option<PathBuf>>> = OnceLock::new();
    RENAME_FAILURE_TARGET.get_or_init(|| Mutex::new(None))
}

#[cfg(test)]
fn set_rename_failure_target(path: Option<PathBuf>) {
    let mut guard = rename_failure_state().lock().expect("rename failure lock");
    *guard = path;
}

fn rename_file(from: &Path, to: &Path) -> Result<(), std::io::Error> {
    #[cfg(test)]
    {
        let mut guard = rename_failure_state().lock().expect("rename failure lock");
        if let Some(target) = guard.as_ref() {
            if target.as_path() == to {
                *guard = None;
                return Err(std::io::Error::new(
                    ErrorKind::Other,
                    "simulated rename failure",
                ));
            }
        }
    }

    std::fs::rename(from, to)
}

fn atomic_write(path: &str, contents: &str) -> Result<(), String> {
    static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

    let path = Path::new(path);
    let parent = path.parent().unwrap_or(Path::new(""));
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("roxanne");
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| err.to_string())?
        .as_millis();
    let base_name = format!(".{file_name}.{stamp}");
    let mut attempts = 0_u32;
    let (temp_path, mut temp_file) = loop {
        let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let temp_name = format!("{base_name}.{counter}.tmp");
        let temp_path = if parent.as_os_str().is_empty() {
            PathBuf::from(&temp_name)
        } else {
            parent.join(&temp_name)
        };
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)
        {
            Ok(file) => break (temp_path, file),
            Err(err) if err.kind() == ErrorKind::AlreadyExists => {
                attempts += 1;
                if attempts > 1000 {
                    return Err("impossible de créer un fichier temporaire unique".to_string());
                }
            }
            Err(err) => return Err(err.to_string()),
        }
    };
    temp_file
        .write_all(contents.as_bytes())
        .and_then(|_| temp_file.sync_all())
        .map_err(|err| err.to_string())?;
    drop(temp_file);

    let backup_path = if path.exists() {
        let mut attempts = 0_u32;
        loop {
            let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
            let backup_name = format!("{base_name}.{counter}.bak");
            let candidate = if parent.as_os_str().is_empty() {
                PathBuf::from(&backup_name)
            } else {
                parent.join(&backup_name)
            };
            if !candidate.exists() {
                break Some(candidate);
            }
            attempts += 1;
            if attempts > 1000 {
                let _ = std::fs::remove_file(&temp_path);
                return Err("impossible de créer un fichier de sauvegarde unique".to_string());
            }
        }
    } else {
        None
    };

    if let Some(backup_path) = backup_path.as_ref() {
        if let Err(err) = rename_file(path, backup_path) {
            let _ = std::fs::remove_file(&temp_path);
            return Err(err.to_string());
        }
    }

    match rename_file(&temp_path, path) {
        Ok(()) => {
            if let Some(backup_path) = backup_path {
                if let Err(err) = std::fs::remove_file(&backup_path) {
                    return Err(format!("suppression sauvegarde échouée: {err}"));
                }
            }
            Ok(())
        }
        Err(err) => {
            let _ = std::fs::remove_file(&temp_path);
            if let Some(backup_path) = backup_path {
                if let Err(restore_err) = rename_file(&backup_path, path) {
                    let _ = std::fs::remove_file(&backup_path);
                    return Err(format!(
                        "{err} (restauration échouée: {restore_err})"
                    ));
                }
            }
            Err(err.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        RoxanneApp, SearchOptions, analyze_diagnostics, atomic_write, find_matches,
        set_rename_failure_target, should_skip_entry, should_skip_file,
    };
    use crate::editor::TextBuffer;
    use std::fs;
    use tempfile::tempdir;

    fn build_text(lines: usize, line_len: usize) -> String {
        let line = "a".repeat(line_len);
        std::iter::repeat(line)
            .take(lines)
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn atomic_write_writes_contents_without_temp_leftover() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("note.txt");
        atomic_write(path.to_str().expect("path"), "hello").expect("atomic write");

        let contents = fs::read_to_string(&path).expect("read file");
        assert_eq!(contents, "hello");

        let leftovers: Vec<_> = fs::read_dir(dir.path())
            .expect("read dir")
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry
                    .file_name()
                    .to_str()
                    .map(|name| name.ends_with(".tmp"))
                    .unwrap_or(false)
            })
            .collect();
        assert!(
            leftovers.is_empty(),
            "temporary files were not cleaned up: {leftovers:?}"
        );
    }

    #[test]
    fn atomic_write_handles_large_payloads() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("large.txt");
        let payload = build_text(50_000, 80);

        atomic_write(path.to_str().expect("path"), &payload).expect("atomic write");
        let contents = fs::read_to_string(&path).expect("read file");

        assert_eq!(contents.len(), payload.len());
        assert_eq!(contents, payload);
    }

    #[test]
    fn atomic_write_preserves_original_on_rename_failure() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("note.txt");
        fs::write(&path, "original").expect("write original");

        set_rename_failure_target(Some(path.clone()));
        let result = atomic_write(path.to_str().expect("path"), "updated");
        assert!(result.is_err(), "atomic write should fail");

        let contents = fs::read_to_string(&path).expect("read original");
        assert_eq!(contents, "original");

        let leftovers: Vec<_> = fs::read_dir(dir.path())
            .expect("read dir")
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry
                    .file_name()
                    .to_str()
                    .map(|name| name.ends_with(".tmp") || name.ends_with(".bak"))
                    .unwrap_or(false)
            })
            .collect();
        assert!(
            leftovers.is_empty(),
            "temporary files were not cleaned up: {leftovers:?}"
        );
    }

    #[test]
    fn atomic_write_restores_backup_after_failed_rename() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("report.txt");
        fs::write(&path, "baseline").expect("write original");

        set_rename_failure_target(Some(path.clone()));
        let result = atomic_write(path.to_str().expect("path"), "replacement");
        assert!(result.is_err(), "atomic write should fail");

        let contents = fs::read_to_string(&path).expect("read original");
        assert_eq!(contents, "baseline");

        let leftovers: Vec<_> = fs::read_dir(dir.path())
            .expect("read dir")
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry
                    .file_name()
                    .to_str()
                    .map(|name| name.ends_with(".tmp") || name.ends_with(".bak"))
                    .unwrap_or(false)
            })
            .collect();
        assert!(
            leftovers.is_empty(),
            "temporary files were not cleaned up: {leftovers:?}"
        );
    }

    #[test]
    fn search_and_diagnostics_use_character_columns() {
        let text = "aé👍b\n🙂}";
        let buffer = TextBuffer::from(text);
        let options = SearchOptions {
            regex: false,
            case_sensitive: true,
            include_hidden: false,
        };

        let matches = find_matches(&buffer, "👍", options).expect("matches");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].line, 0);
        assert_eq!(matches[0].column, 2);
        assert_eq!(matches[0].length, 1);

        let diagnostics = analyze_diagnostics(&buffer);
        let diagnostic = diagnostics
            .iter()
            .find(|item| item.message.contains("Fermeture inattendue"))
            .expect("diagnostic");
        assert_eq!(diagnostic.line, 1);
        assert_eq!(diagnostic.column, 1);
    }

    #[test]
    fn search_case_insensitive_preserves_original_columns_with_expanding_lowercase() {
        let text = "İa";
        let buffer = TextBuffer::from(text);
        let options = SearchOptions {
            regex: false,
            case_sensitive: false,
            include_hidden: false,
        };

        let matches = find_matches(&buffer, "a", options).expect("matches");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].column, 1);
        assert_eq!(matches[0].length, 1);
    }

    #[test]
    fn include_hidden_allows_hidden_files_in_filters() {
        let dir = tempdir().expect("tempdir");
        let hidden_path = dir.path().join(".secret.txt");
        fs::write(&hidden_path, "secret").expect("write hidden file");

        let hidden_entry = walkdir::WalkDir::new(dir.path())
            .min_depth(1)
            .max_depth(1)
            .into_iter()
            .filter_map(Result::ok)
            .find(|entry| entry.file_name() == ".secret.txt")
            .expect("hidden entry");

        assert!(should_skip_entry(&hidden_entry, false));
        assert!(!should_skip_entry(&hidden_entry, true));
        assert!(should_skip_file(&hidden_path, false));
        assert!(!should_skip_file(&hidden_path, true));
    }

    #[test]
    fn diagnostics_ignore_escaped_quotes() {
        let text = r#"let value = "hello\"world";"#;
        let buffer = TextBuffer::from(text);

        let diagnostics = analyze_diagnostics(&buffer);
        assert!(
            diagnostics
                .iter()
                .all(|item| item.message != "Chaîne non terminée."),
            "escaped quote should not trigger unterminated string diagnostic"
        );
    }

    #[test]
    fn diagnostics_ignore_delimiters_in_strings_and_comments() {
        let text = r#"fn main() {
    let value = "{[()]}"; // } ]) )
    let url = "http://example.com";
}"#;
        let buffer = TextBuffer::from(text);

        let diagnostics = analyze_diagnostics(&buffer);
        assert!(
            diagnostics.is_empty(),
            "string/comment delimiters should be ignored: {diagnostics:?}"
        );
    }

    #[test]
    fn diagnostics_ignore_comment_delimiters() {
        let text = "fn main() { // }\n    let value = \"{\";\n}";
        let buffer = TextBuffer::from(text);

        let diagnostics = analyze_diagnostics(&buffer);
        assert!(
            diagnostics.is_empty(),
            "comment delimiters should be ignored: {diagnostics:?}"
        );
    }

    #[test]
    fn diagnostics_ignore_block_comment_delimiters() {
        let text = "fn main() {\n    /* { [ ( */\n    let value = 1;\n}\n";
        let buffer = TextBuffer::from(text);

        let diagnostics = analyze_diagnostics(&buffer);
        assert!(
            diagnostics.is_empty(),
            "block comment delimiters should be ignored: {diagnostics:?}"
        );
    }

    #[test]
    fn diagnostics_ignore_nested_block_comment_delimiters() {
        let text = "fn main() {\n    /* outer { [ /* inner ( ) */ still ] } */\n}\n";
        let buffer = TextBuffer::from(text);

        let diagnostics = analyze_diagnostics(&buffer);
        assert!(
            diagnostics.is_empty(),
            "nested block comment delimiters should be ignored: {diagnostics:?}"
        );
    }

    #[test]
    fn diagnostics_allow_multiline_strings() {
        let text = "fn main() {\n    let value = \"multi\nline\";\n}\n";
        let buffer = TextBuffer::from(text);

        let diagnostics = analyze_diagnostics(&buffer);
        assert!(
            diagnostics
                .iter()
                .all(|item| item.message != "Chaîne non terminée."),
            "multiline strings should not trigger unterminated string diagnostic"
        );
    }

    #[test]
    fn diagnostics_ignore_char_literals_with_braces() {
        let text = "fn main() { let left = '{'; let right = '}'; }";
        let buffer = TextBuffer::from(text);

        let diagnostics = analyze_diagnostics(&buffer);
        assert!(
            diagnostics.is_empty(),
            "char literal braces should be ignored: {diagnostics:?}"
        );
    }

    #[test]
    fn diagnostics_ignore_raw_string_with_braces_and_quotes() {
        let text = r##"fn main() { let value = r#"{ "quoted" }"#; }"##;
        let buffer = TextBuffer::from(text);

        let diagnostics = analyze_diagnostics(&buffer);
        assert!(
            diagnostics.is_empty(),
            "raw string braces/quotes should be ignored: {diagnostics:?}"
        );
    }

    #[test]
    fn workspace_root_from_filename_uses_parent_for_missing_file() {
        let dir = tempdir().expect("tempdir");
        let root_marker = dir.path().join(".roxanne.toml");
        fs::write(&root_marker, "root = true").expect("write root marker");
        let nested = dir.path().join("nested");
        fs::create_dir_all(&nested).expect("create nested dir");
        let missing = nested.join("missing.txt");

        let resolved = RoxanneApp::workspace_root_from_filename(
            missing.to_str().expect("missing path"),
        )
        .expect("workspace root");

        assert_eq!(resolved, dir.path());
    }
}

struct SearchResultLine {
    line: usize,
    column: usize,
    _length: usize,
    preview: String,
}

#[derive(Debug, Clone)]
struct CursorSlot {
    id: usize,
    _position: Position,
    index: usize,
}

#[derive(Debug, Clone)]
struct EditOperation {
    id: usize,
    start: usize,
    end: usize,
    insert: String,
    base: usize,
}

impl EditOperation {
    fn from_edit(slot: &CursorSlot, edit: &EditorEdit, text: &str) -> Option<Self> {
        match edit {
            EditorEdit::Insert(ch) => {
                let insert = ch.to_string();
                Some(Self {
                    id: slot.id,
                    start: slot.index,
                    end: slot.index,
                    base: slot.index + insert.len(),
                    insert,
                })
            }
            EditorEdit::Paste(contents) => {
                let insert = contents.as_str().to_string();
                Some(Self {
                    id: slot.id,
                    start: slot.index,
                    end: slot.index,
                    base: slot.index + insert.len(),
                    insert,
                })
            }
            EditorEdit::Enter => Some(Self {
                id: slot.id,
                start: slot.index,
                end: slot.index,
                base: slot.index + 1,
                insert: "\n".to_string(),
            }),
            EditorEdit::Backspace => {
                let start = prev_char_boundary(text, slot.index)?;
                Some(Self {
                    id: slot.id,
                    start,
                    end: slot.index,
                    base: start,
                    insert: String::new(),
                })
            }
            EditorEdit::Delete => {
                let end = next_char_boundary(text, slot.index)?;
                Some(Self {
                    id: slot.id,
                    start: slot.index,
                    end,
                    base: slot.index,
                    insert: String::new(),
                })
            }
        }
    }

    fn delta(&self) -> isize {
        self.insert.len() as isize - (self.end - self.start) as isize
    }
}

fn apply_operations(text: &str, operations: &[EditOperation]) -> String {
    let mut output = String::with_capacity(text.len().saturating_add(operations.len() * 2));
    let mut cursor = 0;
    for operation in operations {
        if operation.start > text.len() || operation.end > text.len() || operation.start < cursor {
            continue;
        }
        output.push_str(&text[cursor..operation.start]);
        output.push_str(&operation.insert);
        cursor = operation.end;
    }
    output.push_str(&text[cursor..]);
    output
}

fn compute_new_indices(
    cursor_slots: &[CursorSlot],
    operations: &[EditOperation],
) -> std::collections::HashMap<usize, usize> {
    let mut new_indices = std::collections::HashMap::new();
    for slot in cursor_slots {
        let mut base = slot.index;
        if let Some(operation) = operations.iter().find(|op| op.id == slot.id) {
            base = operation.base;
        }
        let mut delta = 0isize;
        for operation in operations {
            if operation.start < slot.index {
                delta += operation.delta();
            }
        }
        let adjusted = (base as isize + delta).max(0) as usize;
        new_indices.insert(slot.id, adjusted);
    }
    new_indices
}

fn prev_char_boundary(text: &str, index: usize) -> Option<usize> {
    if index == 0 {
        return None;
    }
    text[..index].char_indices().last().map(|(i, _)| i)
}

fn next_char_boundary(text: &str, index: usize) -> Option<usize> {
    if index >= text.len() {
        return None;
    }
    let next = text[index..]
        .chars()
        .next()
        .map(|ch| index + ch.len_utf8())?;
    Some(next.min(text.len()))
}

fn parse_goto_input(input: &str) -> Result<(usize, usize), String> {
    let mut parts = input.split(|ch| ch == ':' || ch == ',');
    let line_part = parts.next().unwrap_or("").trim();
    let column_part = parts.next().map(str::trim);

    if parts.next().is_some() {
        return Err("format invalide (utilisez ligne[:colonne])".to_string());
    }

    let line = line_part
        .parse::<usize>()
        .map_err(|_| "ligne invalide".to_string())?;
    if line == 0 {
        return Err("la ligne commence à 1".to_string());
    }

    let column = match column_part {
        Some(value) if !value.is_empty() => value
            .parse::<usize>()
            .map_err(|_| "colonne invalide".to_string())?,
        _ => 1,
    };

    if column == 0 {
        return Err("la colonne commence à 1".to_string());
    }

    Ok((line - 1, column - 1))
}

fn extract_prefix(line: &str, column: usize) -> String {
    let byte_column = char_index_to_byte_index(line, column);
    let mut start = byte_column;
    for (index, ch) in line.char_indices() {
        if index >= byte_column {
            break;
        }
        if !is_word_char(ch) {
            start = index + ch.len_utf8();
        }
    }
    line[start..byte_column].to_string()
}

fn is_word_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

fn build_completion_items(prefix: &str) -> Vec<CompletionItem> {
    if prefix.is_empty() {
        return Vec::new();
    }
    let keywords = [
        "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false", "fn",
        "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref",
        "return", "self", "Self", "static", "struct", "super", "trait", "true", "type", "unsafe",
        "use", "where", "while",
    ];
    let types = [
        "bool", "char", "i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16", "u32", "u64",
        "u128", "usize", "f32", "f64", "str", "String", "Option", "Result", "Vec",
    ];
    let mut items = Vec::new();
    let mut seen = HashSet::new();
    for keyword in keywords.iter().chain(types.iter()) {
        if keyword.starts_with(prefix) && seen.insert(*keyword) {
            let detail = if types.contains(keyword) {
                "Type"
            } else {
                "Keyword"
            };
            items.push(CompletionItem {
                label: (*keyword).to_string(),
                detail: detail.to_string(),
            });
        }
    }
    items.sort_by(|a, b| a.label.cmp(&b.label));
    items
}

fn analyze_diagnostics(buffer: &TextBuffer) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut stack: Vec<(char, Position)> = Vec::new();
    let mut in_string = false;
    let mut in_char = false;
    let mut raw_string_hashes: Option<usize> = None;
    let mut block_comment_depth = 0usize;
    let mut escaped = false;
    let mut string_start: Option<Position> = None;
    let mut raw_string_start: Option<Position> = None;
    let mut char_start: Option<Position> = None;

    for (line_index, line) in buffer.lines().enumerate() {
        if in_string || in_char {
            escaped = false;
        }
        let mut chars = line.chars().enumerate().peekable();
        while let Some((column, ch)) = chars.next() {
            if block_comment_depth > 0 {
                if ch == '/' && matches!(chars.peek(), Some((_, '*'))) {
                    chars.next();
                    block_comment_depth += 1;
                } else if ch == '*' && matches!(chars.peek(), Some((_, '/'))) {
                    chars.next();
                    block_comment_depth = block_comment_depth.saturating_sub(1);
                }
                continue;
            }

            if let Some(hashes) = raw_string_hashes {
                if ch == '"' {
                    if hashes == 0 {
                        raw_string_hashes = None;
                        raw_string_start = None;
                    } else {
                        let mut lookahead = chars.clone();
                        let mut matched = 0;
                        while matched < hashes {
                            if matches!(lookahead.peek(), Some((_, '#'))) {
                                matched += 1;
                                lookahead.next();
                            } else {
                                break;
                            }
                        }
                        if matched == hashes {
                            for _ in 0..hashes {
                                chars.next();
                            }
                            raw_string_hashes = None;
                            raw_string_start = None;
                        }
                    }
                }
                continue;
            }

            if in_string {
                if escaped {
                    escaped = false;
                    continue;
                }
                match ch {
                    '\\' => {
                        escaped = true;
                    }
                    '"' => {
                        in_string = false;
                        string_start = None;
                    }
                    _ => {}
                }
                continue;
            }

            if in_char {
                if escaped {
                    escaped = false;
                    continue;
                }
                match ch {
                    '\\' => {
                        escaped = true;
                    }
                    '\'' => {
                        in_char = false;
                        char_start = None;
                    }
                    _ => {}
                }
                continue;
            }

            if ch == '/' && matches!(chars.peek(), Some((_, '*'))) {
                chars.next();
                block_comment_depth += 1;
                continue;
            }

            if ch == '/' && matches!(chars.peek(), Some((_, '/'))) {
                break;
            }

            match ch {
                'r' => {
                    let mut lookahead = chars.clone();
                    let mut hashes = 0;
                    let mut valid = false;
                    if let Some((_, next)) = lookahead.peek() {
                        if *next == '"' {
                            valid = true;
                        } else if *next == '#' {
                            while let Some((_, '#')) = lookahead.peek() {
                                hashes += 1;
                                lookahead.next();
                            }
                            if matches!(lookahead.peek(), Some((_, '"'))) {
                                valid = true;
                            }
                        }
                    }
                    if valid {
                        if hashes == 0 {
                            chars.next();
                        } else {
                            for _ in 0..hashes {
                                chars.next();
                            }
                            chars.next();
                        }
                        raw_string_hashes = Some(hashes);
                        raw_string_start = Some(Position::new(line_index, column));
                        continue;
                    }
                }
                '"' => {
                    in_string = true;
                    escaped = false;
                    string_start = Some(Position::new(line_index, column));
                }
                '\'' => {
                    in_char = true;
                    escaped = false;
                    char_start = Some(Position::new(line_index, column));
                }
                '{' | '(' | '[' => stack.push((ch, Position::new(line_index, column))),
                '}' | ')' | ']' => {
                    if let Some((open, position)) = stack.pop() {
                        if !matches!((open, ch), ('{', '}') | ('(', ')') | ('[', ']')) {
                            diagnostics.push(Diagnostic {
                                line: line_index,
                                column,
                                message: format!(
                                    "Fermeture inattendue '{ch}' (ouverture '{open}' ligne {}).",
                                    position.line + 1
                                ),
                                severity: DiagnosticSeverity::Error,
                            });
                        }
                    } else {
                        diagnostics.push(Diagnostic {
                            line: line_index,
                            column,
                            message: format!("Fermeture inattendue '{ch}'."),
                            severity: DiagnosticSeverity::Error,
                        });
                    }
                }
                _ => {}
            }
        }
    }

    if raw_string_hashes.is_some() {
        let position = raw_string_start.unwrap_or_else(|| Position::new(0, 0));
        diagnostics.push(Diagnostic {
            line: position.line,
            column: position.column,
            message: "Chaîne non terminée.".to_string(),
            severity: DiagnosticSeverity::Warning,
        });
    }

    if in_string {
        let position = string_start.unwrap_or_else(|| Position::new(0, 0));
        diagnostics.push(Diagnostic {
            line: position.line,
            column: position.column,
            message: "Chaîne non terminée.".to_string(),
            severity: DiagnosticSeverity::Warning,
        });
    }

    if in_char {
        let position = char_start.unwrap_or_else(|| Position::new(0, 0));
        diagnostics.push(Diagnostic {
            line: position.line,
            column: position.column,
            message: "Caractère non terminé.".to_string(),
            severity: DiagnosticSeverity::Warning,
        });
    }

    for (open, position) in stack {
        diagnostics.push(Diagnostic {
            line: position.line,
            column: position.column,
            message: format!("Ouverture '{open}' sans fermeture."),
            severity: DiagnosticSeverity::Warning,
        });
    }

    diagnostics
}

fn char_index_to_byte_index(line: &str, char_index: usize) -> usize {
    if char_index == 0 {
        return 0;
    }
    line.char_indices()
        .nth(char_index)
        .map(|(index, _)| index)
        .unwrap_or(line.len())
}

fn byte_index_to_char_index(line: &str, byte_index: usize) -> usize {
    let mut index = byte_index.min(line.len());
    while index > 0 && !line.is_char_boundary(index) {
        index -= 1;
    }
    line[..index].chars().count()
}
