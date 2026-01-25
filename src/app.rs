use crate::config;
use crate::editor::highlight::MatchPosition;
use crate::editor::{Position, TextBuffer, ViewportCache, highlight};
use crate::keymap::{KeyAction, Keymap, KeymapMode};
use crate::plugins::PluginManager;
use crate::theme::{ThemeConfig, ThemePalette};
use iced::alignment::{Horizontal, Vertical};
use iced::theme;
use iced::widget::button;
use iced::widget::text_editor::{
    Action as EditorAction, Content as EditorContent, Edit as EditorEdit, Motion,
};
use iced::widget::{
    Button, Column, Container, Scrollable, TextInput, column, container, row, text, text_editor,
};
use iced::advanced::{layout, overlay, renderer, widget, Clipboard, Layout, Shell, Widget};
use iced::{
    Alignment, Application, Background, Color, Command, Element, Font, Length, Settings,
    Point, Rectangle, Renderer, Size, Subscription, Theme, Vector, clipboard, event,
    executor, keyboard, mouse,
};
use regex::{Regex, RegexBuilder};
use std::collections::HashSet;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use walkdir::{DirEntry, WalkDir};

const DEFAULT_VIEWPORT_HEIGHT: usize = 24;

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
    SearchScopeSelected(SearchScope),
    SearchPanelToggled,
    GotoLineChanged(String),
    GotoLineSubmit,
    GotoLineClosed,
    SearchInFiles,
    SearchResultsLoaded(Result<Vec<SearchResult>, String>),
    SearchResultOpened(Result<(String, SearchResult), String>),
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
    FileLoaded(Result<String, String>),
    FileSaved(Result<(), String>),
    ThemeExported(Result<PathBuf, String>),
    ThemeImported(Result<ThemeConfig, String>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Menu {
    File,
    Edit,
    Selection,
    View,
    Goto,
    Tools,
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
}

const MENU_BAR_PADDING_X: f32 = 16.0;

struct MenuOverlay<'a> {
    content: Element<'a, Message>,
    overlay: Option<Element<'a, Message>>,
    dismiss_message: Option<Message>,
}

impl<'a> MenuOverlay<'a> {
    fn new(
        content: impl Into<Element<'a, Message>>,
        overlay: Option<Element<'a, Message>>,
        dismiss_message: Option<Message>,
    ) -> Self {
        Self {
            content: content.into(),
            overlay,
            dismiss_message,
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

        let overlay_element = match (&mut self.overlay, children.next()) {
            (Some(overlay), Some(state)) => Some(overlay::Element::new(Box::new(
                MenuOverlayLayer {
                    position: layout.position() + translation,
                    bounds: layout.bounds(),
                    overlay,
                    state,
                    dismiss_message: self.dismiss_message.clone(),
                },
            ))),
            _ => None,
        };

        match (content_overlay, overlay_element) {
            (Some(content_overlay), Some(overlay_element)) => {
                Some(overlay::Group::with_children(vec![
                    content_overlay,
                    overlay_element,
                ])
                .overlay())
            }
            (Some(content_overlay), None) => Some(content_overlay),
            (None, Some(overlay_element)) => Some(overlay_element),
            (None, None) => None,
        }
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

        let submenu_offset = Vector::new(self.position.x + MENU_BAR_PADDING_X, 0.0);

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
            return self.overlay.as_widget().mouse_interaction(
                self.state,
                child,
                cursor,
                viewport,
                renderer,
            );
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
    path: String,
    line: usize,
    column: usize,
    preview: String,
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
            status_message = Some(format!(
                "Démarrage: {}.",
                format_duration(startup_duration)
            ));
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
                search_panel_open: false,
                goto_line_input: String::new(),
                goto_panel_open: false,
                search_scope: SearchScope::CurrentFile,
                search_results: Vec::new(),
                highlight_settings: highlight::Settings::default(),
                multi_cursors: Vec::new(),
                diagnostics,
                diagnostics_panel_open: false,
                completion_items: Vec::new(),
                completion_prefix: String::new(),
                completion_panel_open: false,
                status_message,
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
            Command::none(),
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
                self.refresh_search_matches(true);
                self.refresh_diagnostics();
                self.plugins
                    .on_text_changed(&self.content.text(), &self.filename);
                self.refresh_viewport_cache();
                Command::none()
            }
            Message::SearchChanged(value) => {
                self.search_query = value;
                if self.search_scope == SearchScope::CurrentFile {
                    self.refresh_search_matches(false);
                }
                Command::none()
            }
            Message::SearchNext => self.find_next_match(true),
            Message::SearchPrevious => self.find_next_match(false),
            Message::SearchResultSelected(index) => self.open_search_result(index),
            Message::SearchToggleCaseSensitive => {
                self.search_case_sensitive = !self.search_case_sensitive;
                if self.search_scope == SearchScope::CurrentFile {
                    self.refresh_search_matches(false);
                }
                Command::none()
            }
            Message::SearchToggleRegex => {
                self.search_regex = !self.search_regex;
                if self.search_scope == SearchScope::CurrentFile {
                    self.refresh_search_matches(false);
                }
                Command::none()
            }
            Message::SearchScopeSelected(scope) => {
                self.search_scope = scope;
                if self.search_scope == SearchScope::CurrentFile {
                    self.refresh_search_matches(false);
                }
                Command::none()
            }
            Message::SearchPanelToggled => {
                self.search_panel_open = !self.search_panel_open;
                Command::none()
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
                Command::none()
            }
            Message::DiagnosticsToggled => {
                self.diagnostics_panel_open = !self.diagnostics_panel_open;
                Command::none()
            }
            Message::CompletionRequested => {
                self.refresh_completions();
                self.completion_panel_open = !self.completion_items.is_empty();
                Command::none()
            }
            Message::CompletionSelected(index) => {
                self.apply_completion(index);
                Command::none()
            }
            Message::CompletionClosed => {
                self.completion_panel_open = false;
                self.completion_items.clear();
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
                    self.refresh_search_matches(true);
                    self.refresh_diagnostics();
                    self.plugins
                        .on_text_changed(&self.content.text(), &self.filename);
                    self.refresh_viewport_cache();
                }
                Command::none()
            }
            Message::Event(event) => {
                if let event::Event::Keyboard(keyboard::Event::KeyPressed {
                    key, modifiers, ..
                }) = event
                {
                    if let Some(action) = self.keymap.match_event(&key, modifiers, self.mode) {
                        if matches!(action, KeyAction::Copy | KeyAction::Cut | KeyAction::Paste) {
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
                Command::none()
            }
            Message::KeyAction(action) => self.handle_key_action(action),
            Message::SearchInFiles => self.search_in_files(),
            Message::SearchResultsLoaded(result) => {
                if let Some(started) = self.perf_search_files_started.take() {
                    self.performance.last_search_files = Some(started.elapsed());
                }
                match result {
                    Ok(results) => {
                        self.search_results = results;
                        let duration = self
                            .performance
                            .last_search_files
                            .map(format_duration)
                            .unwrap_or_else(|| "-".to_string());
                        self.status_message = Some(format!(
                            "Recherche fichiers: {} résultat(s) ({duration}).",
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
                    Ok((text, nav)) => {
                        self.filename = nav.path.clone();
                        self.content = EditorContent::with_text(&text);
                        self.buffer.replace(&text);
                        self.last_saved_text = text;
                        self.refresh_search_matches(false);
                        self.refresh_diagnostics();
                        self.plugins.on_text_changed(&self.content.text(), &self.filename);
                        self.refresh_viewport_cache();
                        self.jump_to_position(nav.line, nav.column);
                        self.status_message = Some(format!(
                            "Recherche: ouvert {} (ligne {}, colonne {}).",
                            nav.path,
                            nav.line + 1,
                            nav.column + 1
                        ));
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
                        self.search_in_files()
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
                        Command::none()
                    }
                    MenuAction::ToggleStatusBar => {
                        self.status_message =
                            Some("Affichage: options avancées à venir.".to_string());
                        Command::none()
                    }
                    MenuAction::ToggleSearchPanel => {
                        self.search_panel_open = !self.search_panel_open;
                        Command::none()
                    }
                    MenuAction::GoToLine => {
                        let (line, _) = self.content.cursor_position();
                        self.goto_line_input = format!("{}", line + 1);
                        self.goto_panel_open = true;
                        Command::none()
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
                }
            }
            Message::FileLoaded(result) => {
                if let Some(started) = self.perf_file_open_started.take() {
                    self.performance.last_file_open = Some(started.elapsed());
                }
                match result {
                    Ok(text) => {
                        self.content = EditorContent::with_text(&text);
                        self.buffer.replace(&text);
                        self.last_saved_text = text;
                        self.buffer.clear_history();
                        self.suppress_undo_snapshot = false;
                        self.refresh_search_matches(false);
                        self.refresh_diagnostics();
                        self.plugins.on_file_opened(&self.content.text(), &self.filename);
                        self.plugins.on_text_changed(&self.content.text(), &self.filename);
                        self.refresh_viewport_cache();
                        let duration = self
                            .performance
                            .last_file_open
                            .map(format_duration)
                            .unwrap_or_else(|| "-".to_string());
                        self.status_message = Some(format!("Fichier chargé ({duration})."));
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
                        self.plugins.on_file_saved(&self.last_saved_text, &self.filename);
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
    fn open_file(&mut self) -> Command<Message> {
        if self.filename.trim().is_empty() {
            self.status_message = Some("Nom de fichier manquant.".to_string());
            return Command::none();
        }
        let filename = self.filename.clone();
        self.perf_file_open_started = Some(Instant::now());
        Command::perform(
            async move { std::fs::read_to_string(&filename).map_err(|err| err.to_string()) },
            Message::FileLoaded,
        )
    }

    fn save_file(&mut self) -> Command<Message> {
        if self.filename.trim().is_empty() {
            self.status_message = Some("Nom de fichier manquant.".to_string());
            return Command::none();
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
                let theme = toml::from_str::<ThemeConfig>(&contents)
                    .map_err(|err| err.to_string())?;
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
                    "Recherche: utilisez le champ de recherche dans la barre d'état."
                        .to_string(),
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
                    self.refresh_search_matches(true);
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
        MenuOverlay::new(top_row, self.submenu(), dismiss_message).into()
    }

    fn menu_button(&self, label: &str, menu: Menu) -> Element<'_, Message> {
        let is_active = self.active_menu == Some(menu);
        Button::new(
            text(label)
                .size(14)
                .style(Color::from_rgb8(220, 220, 220))
                .font(Font::MONOSPACE),
        )
        .padding([2, 6])
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
                    ("Open", MenuAction::Open),
                    ("Save", MenuAction::Save),
                    ("Export Theme", MenuAction::ExportTheme),
                    ("Import Theme", MenuAction::ImportTheme),
                ],
            ),
            Menu::Edit => (
                "Edit",
                vec![
                    ("Find", MenuAction::Find),
                    ("Find Next", MenuAction::FindNext),
                    ("Find Previous", MenuAction::FindPrevious),
                    ("Find in Files", MenuAction::FindInFiles),
                    ("Search Panel", MenuAction::ToggleSearchPanel),
                    ("Diagnostics Panel", MenuAction::ToggleDiagnosticsPanel),
                ],
            ),
            Menu::Selection => (
                "Selection",
                vec![
                    ("Select All", MenuAction::SelectAll),
                    ("Add Cursor Next", MenuAction::AddCursorNextMatch),
                    ("Add Cursors All", MenuAction::AddCursorsAllMatches),
                    ("Clear Cursors", MenuAction::ClearMultiCursors),
                ],
            ),
            Menu::View => (
                "View",
                vec![("Status Bar", MenuAction::ToggleStatusBar)],
            ),
            Menu::Goto => ("Goto", vec![("Go to Line", MenuAction::GoToLine)]),
            Menu::Tools => (
                "Tools",
                vec![
                    ("Reload Config", MenuAction::ReloadConfig),
                    ("Performance Report", MenuAction::PerformanceReport),
                ],
            ),
            Menu::Help => ("Help", vec![("About", MenuAction::About)]),
        };

        let row = column![
            text(label)
                .size(12)
                .font(Font::MONOSPACE)
                .style(Color::from_rgb8(180, 180, 180)),
            Column::with_children(
                actions
                    .into_iter()
                    .map(|(name, action)| {
                        Button::new(text(name).size(12).font(Font::MONOSPACE))
                            .padding([2, 8])
                            .style(theme::Button::Custom(Box::new(SubmenuButtonStyle {
                                palette: self.theme,
                            })))
                            .on_press(Message::MenuAction(action))
                            .into()
                    })
                    .collect::<Vec<Element<Message>>>(),
            )
            .spacing(6),
        ]
        .spacing(8)
        .align_items(Alignment::Start)
        .padding([8, 16]);

        Some(
            Container::new(row)
                .style(theme::Container::Custom(Box::new(SubmenuStyle {
                    palette: self.theme,
                })))
                .into(),
        )
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
        let editor = text_editor(&self.content)
            .on_action(Message::Edit)
            .font(Font::MONOSPACE)
            .padding([12, 16])
            .highlight::<highlight::RoxanneHighlighter>(
                self.highlight_settings.clone(),
                highlight::highlight_format,
            );

        Container::new(editor)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(theme::Container::Custom(Box::new(EditorStyle {
                palette: self.theme,
            })))
            .into()
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
                        result.path,
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

        let panel = column![header, input, actions]
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

    fn toggle_button(
        &self,
        label: &str,
        active: bool,
        message: Message,
    ) -> Element<'_, Message> {
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
        }
    }

    fn search_in_files(&mut self) -> Command<Message> {
        if self.search_query.trim().is_empty() {
            self.status_message = Some("Recherche fichiers: saisissez un terme.".to_string());
            return Command::none();
        }

        if self.search_scope == SearchScope::CurrentFile {
            let started = Instant::now();
            self.refresh_search_matches(false);
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
        let root = match std::env::current_dir() {
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

    fn refresh_search_matches(&mut self, preserve_index: bool) {
        let options = self.search_options();
        self.search_matches = match find_matches(&self.buffer, &self.search_query, options) {
            Ok(matches) => matches,
            Err(message) => {
                self.current_match_index = None;
                self.search_matches.clear();
                self.highlight_settings.search_matches.clear();
                if !self.search_query.is_empty() {
                    self.status_message = Some(format!("Recherche: {message}"));
                }
                return;
            }
        };
        self.highlight_settings.search_matches = self.search_matches.clone();
        if self.search_matches.is_empty() {
            self.current_match_index = None;
            if !self.search_query.is_empty() {
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

        if !self.search_query.is_empty() {
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
                std::fs::read_to_string(&path)
                    .map(|text| (text, result))
                    .map_err(|err| err.to_string())
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
            .map(|line| line.len())
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
        let mut last_end = 0;
        for operation in operations {
            if operation.start < last_end {
                continue;
            }
            last_end = operation.end;
            filtered.push(operation);
        }

        let new_text = apply_operations(&text, &filtered);
        self.buffer.replace(&new_text);
        self.content = EditorContent::with_text(&self.buffer.text());

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
        self.status_message = Some(format!(
            "Multi-curseurs: édition sur {} curseur(s).",
            self.multi_cursors.len() + 1
        ));
    }

    fn record_undo_snapshot(&mut self) {
        if self.suppress_undo_snapshot {
            return;
        }
        self.buffer.record_snapshot();
    }

    fn apply_undo(&mut self) {
        let Some(previous) = self.buffer.undo() else {
            return;
        };
        self.apply_snapshot(previous);
    }

    fn apply_redo(&mut self) {
        let Some(next) = self.buffer.redo() else {
            return;
        };
        self.apply_snapshot(next);
    }

    fn apply_snapshot(&mut self, text: String) {
        self.suppress_undo_snapshot = true;
        self.content = EditorContent::with_text(&text);
        self.buffer.replace(&text);
        self.refresh_search_matches(true);
        self.refresh_diagnostics();
        self.plugins
            .on_text_changed(&self.content.text(), &self.filename);
        self.refresh_viewport_cache();
        self.suppress_undo_snapshot = false;
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
        let column = column.min(line_text.len());
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
        self.refresh_search_matches(true);
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

struct SubmenuButtonStyle {
    palette: ThemePalette,
}

impl button::StyleSheet for SubmenuButtonStyle {
    type Style = Theme;

    fn active(&self, _style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(self.palette.button_base)),
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

async fn search_in_workspace(
    root: PathBuf,
    query: String,
    options: SearchOptions,
) -> Result<Vec<SearchResult>, String> {
    if query.trim().is_empty() {
        return Ok(Vec::new());
    }

    let matcher = build_matcher(&query, options)?;
    let mut results = Vec::new();
    let mut collected = 0usize;
    let max_results = 500usize;

    for entry in WalkDir::new(&root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| !should_skip_entry(entry))
    {
        let entry = entry.map_err(|err| err.to_string())?;
        if !entry.file_type().is_file() {
            continue;
        }

        if should_skip_file(entry.path()) {
            continue;
        }

        let metadata = entry.metadata().map_err(|err| err.to_string())?;
        if metadata.len() > 1_000_000 {
            continue;
        }

        let contents = match std::fs::read_to_string(entry.path()) {
            Ok(contents) => contents,
            Err(_) => continue,
        };

        for line_match in find_matches_in_text(&contents, &matcher) {
            results.push(SearchResult {
                path: entry.path().display().to_string(),
                line: line_match.line,
                column: line_match.column,
                preview: line_match.preview,
            });
            collected += 1;
            if collected >= max_results {
                return Ok(results);
            }
        }
    }

    Ok(results)
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

fn find_matches_in_line(line: &str, matcher: &SearchMatcher) -> Vec<(usize, usize)> {
    match matcher {
        SearchMatcher::Regex(regex) => regex
            .find_iter(line)
            .map(|found| (found.start(), found.end() - found.start()))
            .collect(),
        SearchMatcher::Plain {
            needle,
            case_sensitive,
        } => {
            let mut matches = Vec::new();
            if needle.is_empty() {
                return matches;
            }

            let search_line = if *case_sensitive {
                line.to_string()
            } else {
                line.to_lowercase()
            };
            let search_needle = if *case_sensitive {
                needle.clone()
            } else {
                needle.to_lowercase()
            };

            let mut search_start = 0;
            while let Some(found) = search_line[search_start..].find(&search_needle) {
                let column = search_start + found;
                matches.push((column, needle.len()));
                search_start = column + needle.len();
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

fn should_skip_entry(entry: &DirEntry) -> bool {
    if entry.depth() == 0 {
        return false;
    }
    let name = entry.file_name().to_string_lossy();
    let skip_dirs = [".git", "target", "node_modules", "dist", "build", "out"];
    if entry.file_type().is_dir() && skip_dirs.contains(&name.as_ref()) {
        return true;
    }
    name.starts_with('.')
}

fn should_skip_file(path: &Path) -> bool {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");
    if file_name.starts_with('.') {
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

fn atomic_write(path: &str, contents: &str) -> Result<(), String> {
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
    let temp_name = format!(".{file_name}.{stamp}.tmp");
    let temp_path = if parent.as_os_str().is_empty() {
        PathBuf::from(&temp_name)
    } else {
        parent.join(&temp_name)
    };

    let mut temp_file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&temp_path)
        .map_err(|err| err.to_string())?;
    temp_file
        .write_all(contents.as_bytes())
        .and_then(|_| temp_file.sync_all())
        .map_err(|err| err.to_string())?;

    match std::fs::rename(&temp_path, path) {
        Ok(()) => Ok(()),
        Err(err) => {
            if path.exists() {
                if let Err(remove_err) = std::fs::remove_file(path) {
                    let _ = std::fs::remove_file(&temp_path);
                    return Err(format!("{err} (suppression échouée: {remove_err})"));
                }
                std::fs::rename(&temp_path, path).map_err(|err| err.to_string())
            } else {
                let _ = std::fs::remove_file(&temp_path);
                Err(err.to_string())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::atomic_write;
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
    let mut start = column;
    for (index, ch) in line.char_indices() {
        if index >= column {
            break;
        }
        if !is_word_char(ch) {
            start = index + ch.len_utf8();
        }
    }
    line[start..column].to_string()
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

    for (line_index, line) in buffer.lines().enumerate() {
        let mut quotes = 0usize;
        for (column, ch) in line.char_indices() {
            match ch {
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
                '"' => quotes += 1,
                _ => {}
            }
        }

        if quotes % 2 == 1 {
            diagnostics.push(Diagnostic {
                line: line_index,
                column: line.len().saturating_sub(1),
                message: "Chaîne non terminée.".to_string(),
                severity: DiagnosticSeverity::Warning,
            });
        }
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
