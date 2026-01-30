use crate::completion::{self, CompletionItem};
use crate::config;
use crate::diagnostics::{self, Diagnostic, DiagnosticSeverity};
use crate::editor::highlight::MatchPosition;
use crate::editor::{Position, TextBuffer, ViewportCache, highlight};
use crate::file_ops::{self, MAX_OPEN_FILE_SIZE};
use crate::keymap::{KeyAction, Keymap, KeymapMode};
use crate::plugins::PluginManager;
use crate::search::{self, SearchOptions, SearchResult, SearchResultsSummary, SearchScope};
use crate::theme::{ThemeConfig, ThemePalette};
use crate::ui::styles;
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, overlay, renderer, widget};
use iced::alignment::{Horizontal, Vertical};
use iced::theme;
use iced::widget::text_editor::{
    Action as EditorAction, Content as EditorContent, Edit as EditorEdit, Motion,
};
use iced::widget::{
    Button, Column, Container, Scrollable, Space, TextInput, column, container, row, text,
    text_editor,
};
use iced::{
    Alignment, Application, Color, Command, Element, Font, Length, Point, Rectangle,
    Renderer, Settings, Size, Subscription, Theme, Vector, clipboard, event, executor, keyboard,
    mouse, subscription, window,
};
use iced::widget::text::LineHeight;
use std::borrow::Cow;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const DEFAULT_VIEWPORT_HEIGHT: usize = 24;
const FILE_TREE_MAX_DEPTH: usize = 8;
const FILE_TREE_MAX_ENTRIES: usize = 500;

#[derive(Debug, Clone)]
pub struct FileTreeEntry {
    name: String,
    path: PathBuf,
    is_dir: bool,
    depth: usize,
    expanded: bool,
    children: Vec<FileTreeEntry>,
}

impl FileTreeEntry {
    fn scan(path: &Path, depth: usize) -> Option<Self> {
        let name = path.file_name()?.to_string_lossy().to_string();
        if name.starts_with('.') {
            return None;
        }
        let is_dir = path.is_dir();
        let children = if is_dir && depth < FILE_TREE_MAX_DEPTH {
            let mut entries: Vec<FileTreeEntry> = std::fs::read_dir(path)
                .ok()?
                .filter_map(|e| e.ok())
                .filter_map(|e| Self::scan(&e.path(), depth + 1))
                .collect();
            entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            });
            entries
        } else {
            Vec::new()
        };
        Some(Self {
            name,
            path: path.to_path_buf(),
            is_dir,
            depth,
            expanded: depth == 0,
            children,
        })
    }

    fn toggle_dir(&mut self, target: &Path) -> bool {
        if self.path == target {
            self.expanded = !self.expanded;
            return true;
        }
        for child in &mut self.children {
            if child.toggle_dir(target) {
                return true;
            }
        }
        false
    }

    fn flatten_visible(&self) -> Vec<(usize, &FileTreeEntry)> {
        let mut result = vec![(self.depth, self)];
        if self.is_dir && self.expanded {
            for child in &self.children {
                result.extend(child.flatten_visible());
            }
        }
        result
    }
}
const EDITOR_CONTAINER_ID: &str = "roxanne-editor-area";
const EDITOR_VERTICAL_PADDING: f32 = 24.0;
const EDITOR_LINE_HEIGHT: f32 = 16.0;

fn editor_container_id() -> container::Id {
    container::Id::new(EDITOR_CONTAINER_ID)
}

#[derive(Debug)]
pub struct Tab {
    filename: String,
    content: EditorContent,
    buffer: TextBuffer,
    viewport_cache: ViewportCache,
    last_saved_text: String,
    highlight_settings: highlight::Settings,
    multi_cursors: Vec<Position>,
    diagnostics: Vec<Diagnostic>,
    completion_items: Vec<CompletionItem>,
    completion_prefix: String,
    file_is_lossy: bool,
    lossy_save_acknowledged: bool,
    suppress_undo_snapshot: bool,
    search_matches: Vec<MatchPosition>,
    current_match_index: Option<usize>,
}

impl Tab {
    fn new(filename: &str, text: &str) -> Self {
        let buffer = TextBuffer::from(text);
        let diagnostics = diagnostics::analyze(&buffer);
        let language = std::path::Path::new(filename)
            .extension()
            .and_then(|ext| ext.to_str())
            .map(highlight::Language::from_extension)
            .unwrap_or(highlight::Language::Plain);
        Self {
            filename: filename.to_string(),
            content: EditorContent::with_text(text),
            buffer,
            viewport_cache: {
                let mut vc = ViewportCache::new();
                vc.update(&TextBuffer::from(text), 0, DEFAULT_VIEWPORT_HEIGHT);
                vc
            },
            last_saved_text: text.to_string(),
            highlight_settings: highlight::Settings {
                language,
                buffer_text: text.into(),
                ..highlight::Settings::default()
            },
            multi_cursors: Vec::new(),
            diagnostics,
            completion_items: Vec::new(),
            completion_prefix: String::new(),
            file_is_lossy: false,
            lossy_save_acknowledged: false,
            suppress_undo_snapshot: false,
            search_matches: Vec::new(),
            current_match_index: None,
        }
    }

    fn is_modified(&self) -> bool {
        self.content.text() != self.last_saved_text
    }

    fn display_name(&self) -> &str {
        if self.filename.trim().is_empty() {
            "untitled.txt"
        } else {
            &self.filename
        }
    }
}

#[derive(Debug)]
pub struct RoxanneApp {
    tabs: Vec<Tab>,
    active_tab: usize,
    viewport_height: usize,
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
    status_message: Option<String>,
    theme: ThemePalette,
    keymap: Keymap,
    mode: KeymapMode,
    plugins: PluginManager,
    active_menu: Option<Menu>,
    performance: PerformanceMetrics,
    perf_file_open_started: Option<Instant>,
    perf_file_save_started: Option<Instant>,
    perf_search_files_started: Option<Instant>,
}

impl RoxanneApp {
    fn tab(&self) -> &Tab {
        &self.tabs[self.active_tab]
    }

    fn tab_mut(&mut self) -> &mut Tab {
        &mut self.tabs[self.active_tab]
    }
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
    TabSelected(usize),
    TabClosed(usize),
    NewTab,
    NextTab,
    PrevTab,
    ReplaceChanged(String),
    ReplaceCurrent,
    ReplaceAll,
    CommandPaletteToggle,
    CommandPaletteChanged(String),
    CommandPaletteSelected(usize),
    ConfigFileChanged,
    FileTreeToggle,
    FileTreeToggleDir(PathBuf),
    FileTreeFileClicked(PathBuf),
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
    ToggleFileTree,
    About,
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
        let tab = Tab::new("untitled.txt", initial_text);
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
                tabs: vec![tab],
                active_tab: 0,
                viewport_height: DEFAULT_VIEWPORT_HEIGHT,
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
                status_message,
                theme: flags.theme,
                keymap: flags.keymap,
                mode: KeymapMode::Insert,
                plugins,
                active_menu: None,
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
                if action.is_edit() && !self.tab().multi_cursors.is_empty() {
                    if let EditorAction::Edit(edit) = action {
                        self.apply_multi_cursor_edit(&edit);
                    }
                } else {
                    self.tab_mut().content.perform(action);
                    let text = self.tab().content.text();
                    self.tab_mut().buffer.replace(&text);
                }
                self.sync_highlight_buffer();
                self.refresh_search_matches(true, true);
                self.refresh_diagnostics();
                let text = self.tab().content.text().to_string();
                let filename = self.tab().filename.clone();
                self.plugins.on_text_changed(&text, &filename);
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
                self.completion_panel_open = !self.tab().completion_items.is_empty();
                Self::editor_bounds_command()
            }
            Message::CompletionSelected(index) => {
                self.apply_completion(index);
                Command::none()
            }
            Message::CompletionClosed => {
                self.completion_panel_open = false;
                self.tab_mut().completion_items.clear();
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
                    if self.tab().multi_cursors.is_empty() {
                        self.record_undo_snapshot();
                        self.tab_mut().content.perform(EditorAction::Edit(insert));
                        let t = self.tab().content.text();
                        self.tab_mut().buffer.replace(&t);
                    } else {
                        self.record_undo_snapshot();
                        self.apply_multi_cursor_edit(&insert);
                    }
                    self.sync_highlight_buffer();
                    self.refresh_search_matches(true, true);
                    self.refresh_diagnostics();
                    let t = self.tab().content.text().to_string();
                    let f = self.tab().filename.clone();
                    self.plugins.on_text_changed(&t, &f);
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
                        let path_str = nav.path.display().to_string();
                        // Open in a new tab (or switch to existing)
                        let existing = self.tabs.iter().position(|t| t.filename == path_str);
                        if let Some(idx) = existing {
                            self.active_tab = idx;
                            // Reload content
                            let tab = self.tab_mut();
                            tab.content = EditorContent::with_text(&text);
                            tab.buffer.replace(&text);
                            tab.last_saved_text = text;
                        } else {
                            let mut new_tab = Tab::new(&path_str, &text);
                            new_tab.file_is_lossy = load.lossy;
                            new_tab.last_saved_text = text;
                            self.tabs.push(new_tab);
                            self.active_tab = self.tabs.len() - 1;
                        }
                        self.tab_mut().buffer.clear_history();
                        self.tab_mut().multi_cursors.clear();
                        self.completion_panel_open = false;
                        self.tab_mut().completion_items.clear();
                        self.tab_mut().suppress_undo_snapshot = false;
                        self.sync_highlight_buffer();
                        self.refresh_search_matches(false, true);
                        self.refresh_diagnostics();
                        let ct = self.tab().content.text().to_string();
                        let cf = self.tab().filename.clone();
                        self.plugins.on_text_changed(&ct, &cf);
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
                self.tab_mut().filename = value;
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
                        self.tab_mut().multi_cursors.clear();
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
                        let (line, _) = self.tab().content.cursor_position();
                        self.goto_line_input = format!("{}", line + 1);
                        self.goto_panel_open = true;
                        Self::editor_bounds_command()
                    }
                    MenuAction::ReloadConfig => self.reload_config(),
                    MenuAction::ToggleFileTree => {
                        return self.update(Message::FileTreeToggle);
                    }
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
                    Ok(load) => {
                        let text = load.text;
                        let filename = self.tab().filename.clone();
                        // Check if a tab already has this file open
                        let existing = self.tabs.iter().position(|t| t.filename == filename);
                        if let Some(idx) = existing {
                            self.active_tab = idx;
                        }
                        let tab = self.tab_mut();
                        tab.file_is_lossy = load.lossy;
                        tab.lossy_save_acknowledged = false;
                        tab.content = EditorContent::with_text(&text);
                        tab.buffer.replace(&text);
                        tab.last_saved_text = text;
                        tab.buffer.clear_history();
                        tab.multi_cursors.clear();
                        tab.completion_items.clear();
                        tab.suppress_undo_snapshot = false;
                        self.completion_panel_open = false;
                        self.sync_highlight_buffer();
                        self.refresh_search_matches(false, true);
                        self.refresh_diagnostics();
                        let ct = self.tab().content.text().to_string();
                        let cf = self.tab().filename.clone();
                        self.plugins.on_file_opened(&ct, &cf);
                        self.plugins.on_text_changed(&ct, &cf);
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
                        let saved_text = self.tab().content.text().to_string();
                        self.tab_mut().last_saved_text = saved_text.clone();
                        let filename = self.tab().filename.clone();
                        self.plugins.on_file_saved(&saved_text, &filename);
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
            Message::TabSelected(index) => {
                if index < self.tabs.len() {
                    self.active_tab = index;
                    self.refresh_search_matches(false, false);
                    self.refresh_viewport_cache();
                }
                Command::none()
            }
            Message::TabClosed(index) => {
                if index < self.tabs.len() {
                    // Don't close the last tab — create a new empty one instead
                    if self.tabs.len() == 1 {
                        self.tabs[0] = Tab::new("untitled.txt", "");
                        self.active_tab = 0;
                    } else {
                        self.tabs.remove(index);
                        if self.active_tab >= self.tabs.len() {
                            self.active_tab = self.tabs.len() - 1;
                        } else if self.active_tab > index {
                            self.active_tab -= 1;
                        }
                    }
                    self.refresh_search_matches(false, false);
                    self.refresh_viewport_cache();
                }
                Command::none()
            }
            Message::NewTab => {
                let tab = Tab::new("untitled.txt", "");
                self.tabs.push(tab);
                self.active_tab = self.tabs.len() - 1;
                self.refresh_viewport_cache();
                Command::none()
            }
            Message::NextTab => {
                if !self.tabs.is_empty() {
                    self.active_tab = (self.active_tab + 1) % self.tabs.len();
                    self.refresh_search_matches(false, false);
                    self.refresh_viewport_cache();
                }
                Command::none()
            }
            Message::ConfigFileChanged => {
                return self.reload_config();
            }
            Message::CommandPaletteToggle => {
                self.command_palette_open = !self.command_palette_open;
                self.command_palette_query.clear();
                Command::none()
            }
            Message::CommandPaletteChanged(value) => {
                self.command_palette_query = value;
                Command::none()
            }
            Message::CommandPaletteSelected(index) => {
                self.command_palette_open = false;
                let commands = self.filtered_commands();
                if let Some((_, action)) = commands.get(index) {
                    let action = *action;
                    return self.update(Message::MenuAction(action));
                }
                Command::none()
            }
            Message::ReplaceChanged(value) => {
                self.replace_text = value;
                Command::none()
            }
            Message::ReplaceCurrent => {
                self.replace_current_match();
                Command::none()
            }
            Message::ReplaceAll => {
                self.replace_all_matches();
                Command::none()
            }
            Message::FileTreeToggle => {
                self.file_tree_open = !self.file_tree_open;
                if self.file_tree_open && self.file_tree.is_none() {
                    let root = self.resolve_workspace_root().ok()
                        .or_else(|| std::env::current_dir().ok());
                    if let Some(root) = root {
                        self.file_tree = FileTreeEntry::scan(&root, 0);
                    }
                }
                Command::none()
            }
            Message::FileTreeToggleDir(path) => {
                if let Some(tree) = self.file_tree.as_mut() {
                    tree.toggle_dir(&path);
                }
                Command::none()
            }
            Message::FileTreeFileClicked(path) => {
                let path_str = path.display().to_string();
                // Check if already open
                if let Some(idx) = self.tabs.iter().position(|t| t.filename == path_str) {
                    self.active_tab = idx;
                    self.refresh_search_matches(false, false);
                    self.refresh_viewport_cache();
                    return Command::none();
                }
                // Open in new tab
                self.tab_mut().filename = path_str;
                self.perf_file_open_started = Some(Instant::now());
                let filename = self.tab().filename.clone();
                // Create a new tab and open the file
                let new_tab = Tab::new(&filename, "");
                self.tabs.push(new_tab);
                self.active_tab = self.tabs.len() - 1;
                self.open_file()
            }
            Message::PrevTab => {
                if !self.tabs.is_empty() {
                    self.active_tab = (self.active_tab + self.tabs.len() - 1) % self.tabs.len();
                    self.refresh_search_matches(false, false);
                    self.refresh_viewport_cache();
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

        let command_palette = self.command_palette();

        let mut content = column![menu_bar, tab_bar];
        if let Some(palette) = command_palette {
            content = content.push(
                Container::new(palette)
                    .width(Length::Fill)
                    .center_x(),
            );
        }
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
        let file_tree = self.file_tree_panel();
        let editor_row: Element<'_, Message> = if let Some(tree_panel) = file_tree {
            row![tree_panel, editor]
                .spacing(0)
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        } else {
            editor
        };
        let content = content
            .push(editor_row)
            .push(status_bar)
            .spacing(0)
            .width(Length::Fill)
            .height(Length::Fill);

        Container::new(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(theme::Container::Custom(styles::app_background(&self.theme)))
            .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            event::listen().map(Message::Event),
            config_watcher_subscription(),
        ])
    }
}

impl RoxanneApp {
    fn editor_bounds_command() -> Command<Message> {
        container::visible_bounds(editor_container_id()).map(Message::EditorBoundsChanged)
    }

    fn update_viewport_height(&mut self, bounds: Rectangle) {
        let available_height = (bounds.height - EDITOR_VERTICAL_PADDING).max(0.0);
        let line_height = EDITOR_LINE_HEIGHT.max(1.0);
        let visible_lines = (available_height / line_height).ceil().max(1.0) as usize;
        if visible_lines != self.viewport_height {
            self.viewport_height = visible_lines;
            self.refresh_viewport_cache();
        }
    }

    fn open_file(&mut self) -> Command<Message> {
        let filename = self.tab().filename.clone();
        if filename.trim().is_empty() {
            self.status_message = Some("Nom de fichier manquant.".to_string());
            return Command::none();
        }
        // If file is already open in another tab, just switch
        if let Some(idx) = self.tabs.iter().position(|t| t.filename == filename) {
            if idx != self.active_tab {
                self.active_tab = idx;
                self.refresh_search_matches(false, false);
                self.refresh_viewport_cache();
                return Command::none();
            }
        }
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
        if self.tab().filename.trim().is_empty() {
            self.status_message = Some("Nom de fichier manquant.".to_string());
            return Command::none();
        }
        if self.tab().file_is_lossy && !self.tab().lossy_save_acknowledged {
            self.tab_mut().lossy_save_acknowledged = true;
            self.status_message = Some(
                "Sauvegarde bloquée: le fichier contient des caractères invalides remplacés. \
Relancez «Enregistrer» pour confirmer l'écriture."
                    .to_string(),
            );
            return Command::none();
        }
        if self.tab().file_is_lossy {
            self.tab_mut().lossy_save_acknowledged = false;
        }
        let filename = self.tab().filename.clone();
        let ct = self.tab().content.text();
        self.tab_mut().buffer.replace(&ct);
        let text = self.tab().buffer.text().to_string();
        self.perf_file_save_started = Some(Instant::now());
        Command::perform(
            async move { file_ops::atomic_write(&filename, &text).map_err(|err| err.to_string()) },
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
                self.tab_mut().content
                    .perform(EditorAction::Move(Motion::DocumentStart));
                self.tab_mut().content
                    .perform(EditorAction::Select(Motion::DocumentEnd));
                self.refresh_viewport_cache();
                Command::none()
            }
            KeyAction::Copy => {
                if let Some(selection) = self.tab().content.selection() {
                    if !selection.is_empty() {
                        return clipboard::write(selection);
                    }
                }
                Command::none()
            }
            KeyAction::Cut => {
                if let Some(selection) = self.tab().content.selection() {
                    if selection.is_empty() {
                        return Command::none();
                    }
                    self.record_undo_snapshot();
                    self.tab_mut().content
                        .perform(EditorAction::Edit(EditorEdit::Backspace));
                    let t = self.tab().content.text();
                    self.tab_mut().buffer.replace(&t);
                    self.sync_highlight_buffer();
                    self.refresh_search_matches(true, true);
                    self.refresh_diagnostics();
                    let t = self.tab().content.text().to_string();
                    let f = self.tab().filename.clone();
                    self.plugins.on_text_changed(&t, &f);
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
                self.completion_panel_open = !self.tab().completion_items.is_empty();
                Command::none()
            }
            KeyAction::CompletionClose => {
                self.completion_panel_open = false;
                self.tab_mut().completion_items.clear();
                Command::none()
            }
            KeyAction::NewTab => {
                self.update(Message::NewTab)
            }
            KeyAction::CloseTab => {
                let idx = self.active_tab;
                self.update(Message::TabClosed(idx))
            }
            KeyAction::NextTab => {
                self.update(Message::NextTab)
            }
            KeyAction::PrevTab => {
                self.update(Message::PrevTab)
            }
            KeyAction::CommandPalette => {
                self.command_palette_open = !self.command_palette_open;
                self.command_palette_query.clear();
                Command::none()
            }
            KeyAction::ToggleFileTree => {
                self.update(Message::FileTreeToggle)
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
            .style(theme::Container::Custom(styles::menu_bar(&self.theme)));

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
        .style(theme::Button::Custom(styles::menu_button(&self.theme, is_active)))
        .on_press(Message::MenuSelected(menu))
        .into()
    }

    fn submenu(&self) -> Option<Element<'_, Message>> {
        let (label, actions) = match self.active_menu? {
            Menu::File => (
                "File",
                vec![
                    MenuEntry::action("Open File… — Ctrl+O", MenuAction::Open),
                    MenuEntry::Separator,
                    MenuEntry::Separator,
                    MenuEntry::action("Save — Ctrl+S", MenuAction::Save),
                    MenuEntry::Separator,
                    MenuEntry::action("Export Theme", MenuAction::ExportTheme),
                    MenuEntry::action("Import Theme", MenuAction::ImportTheme),
                    MenuEntry::Separator,
                ],
            ),
            Menu::Edit => (
                "Edit",
                vec![
                    MenuEntry::Separator,
                    MenuEntry::Separator,
                    MenuEntry::action("Find — Ctrl+F", MenuAction::Find),
                    MenuEntry::action("Find Next — F3", MenuAction::FindNext),
                    MenuEntry::action("Find Previous — Shift+F3", MenuAction::FindPrevious),
                    MenuEntry::action("Find in Files… — Ctrl+Shift+F", MenuAction::FindInFiles),
                    MenuEntry::Separator,
                    MenuEntry::action("Search Panel", MenuAction::ToggleSearchPanel),
                    MenuEntry::action("Diagnostics Panel", MenuAction::ToggleDiagnosticsPanel),
                ],
            ),
            Menu::Selection => (
                "Selection",
                vec![
                    MenuEntry::action("Select All — Ctrl+A", MenuAction::SelectAll),
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
                ],
            ),
            Menu::View => (
                "View",
                vec![
                    MenuEntry::action("Show Status Bar", MenuAction::ToggleStatusBar),
                    MenuEntry::action("Toggle File Tree — Ctrl+B", MenuAction::ToggleFileTree),
                    MenuEntry::Separator,
                ],
            ),
            Menu::Goto => (
                "Goto",
                vec![
                    MenuEntry::Separator,
                    MenuEntry::action("Go to Line… — Ctrl+G", MenuAction::GoToLine),
                ],
            ),
            Menu::Tools => (
                "Tools",
                vec![
                    MenuEntry::Separator,
                    MenuEntry::Separator,
                    MenuEntry::Separator,
                    MenuEntry::action("Reload Config", MenuAction::ReloadConfig),
                    MenuEntry::action("Performance Report", MenuAction::PerformanceReport),
                ],
            ),
            Menu::Project => (
                "Project",
                vec![
                    MenuEntry::Separator,
                    MenuEntry::Separator,
                ],
            ),
            Menu::Preferences => (
                "Preferences",
                vec![
                    MenuEntry::Separator,
                    MenuEntry::Separator,
                    MenuEntry::Separator,
                ],
            ),
            Menu::Help => (
                "Help",
                vec![
                    MenuEntry::Separator,
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
                        .style(theme::Button::Custom(styles::submenu_button(&self.theme)))
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
                .style(theme::Container::Custom(styles::submenu(&self.theme)))
                .into(),
        )
    }

    fn submenu_separator(&self) -> Element<'_, Message> {
        Container::new(Space::with_height(Length::Fixed(1.0)))
            .width(Length::Shrink)
            .style(theme::Container::Custom(styles::submenu_separator(&self.theme)))
            .into()
    }

    fn tab_bar(&self) -> Element<'_, Message> {
        let mut tabs_row = row![].spacing(2).padding([0, 8]).align_items(Alignment::Center);

        for (index, tab) in self.tabs.iter().enumerate() {
            let is_active = index == self.active_tab;
            let is_modified = tab.is_modified();
            let name = tab.display_name();

            let label = row![
                text(name)
                    .size(13)
                    .font(Font::MONOSPACE)
                    .style(if is_active {
                        Color::from_rgb8(230, 230, 230)
                    } else {
                        Color::from_rgb8(160, 160, 160)
                    }),
                text(if is_modified { "●" } else { "" })
                    .size(12)
                    .font(Font::MONOSPACE)
                    .style(Color::from_rgb8(180, 180, 180)),
            ]
            .spacing(4)
            .align_items(Alignment::Center);

            let tab_content = row![
                Button::new(label)
                    .padding([6, 12])
                    .style(if is_active {
                        theme::Button::Custom(styles::menu_button(&self.theme, true))
                    } else {
                        theme::Button::Custom(styles::menu_button(&self.theme, false))
                    })
                    .on_press(Message::TabSelected(index)),
                Button::new(
                    text("×").size(12).font(Font::MONOSPACE)
                        .style(Color::from_rgb8(150, 150, 150))
                )
                    .padding([4, 6])
                    .style(theme::Button::Custom(styles::submenu_button(&self.theme)))
                    .on_press(Message::TabClosed(index)),
            ]
            .spacing(0)
            .align_items(Alignment::Center);

            let tab_style = if is_active {
                theme::Container::Custom(styles::active_tab(&self.theme))
            } else {
                theme::Container::Transparent
            };

            tabs_row = tabs_row.push(
                Container::new(tab_content)
                    .style(tab_style)
                    .height(Length::Fill),
            );
        }

        // New tab button
        tabs_row = tabs_row.push(
            Button::new(text("+").size(14).font(Font::MONOSPACE).style(Color::from_rgb8(150, 150, 150)))
                .padding([4, 8])
                .style(theme::Button::Custom(styles::submenu_button(&self.theme)))
                .on_press(Message::NewTab),
        );

        Container::new(tabs_row)
            .width(Length::Fill)
            .height(Length::Fixed(32.0))
            .style(theme::Container::Custom(styles::tab_bar(&self.theme)))
            .into()
    }

    fn editor_area(&self) -> Element<'_, Message> {
        let line_height = EDITOR_LINE_HEIGHT.max(1.0);
        let vertical_padding = 12.0;
        let line_number_size = 14;
        let line_count = self.tab().buffer.line_count().max(1);
        let gutter_digits = line_count.to_string().len();
        let gutter_width = (gutter_digits as f32 * (line_number_size as f32 * 0.6)) + 24.0;

        // Build the entire gutter as a single multi-line text block so it
        // stretches with the editor instead of being limited by the number
        // of child widgets in a Column.
        let mut gutter_text = String::new();
        for i in 1..=line_count {
            if !gutter_text.is_empty() {
                gutter_text.push('\n');
            }
            gutter_text.push_str(&format!("{:>width$}", i, width = gutter_digits));
        }
        // Add tilde lines well past the document end so the gutter always
        // reaches the bottom of the visible area regardless of window size.
        let extra = 200usize;
        for _ in 0..extra {
            gutter_text.push('\n');
            gutter_text.push_str(&format!("{:>width$}", "~", width = gutter_digits));
        }

        let gutter_content = text(gutter_text)
            .size(line_number_size)
            .font(Font::MONOSPACE)
            .style(Color::from_rgb8(140, 140, 140))
            .line_height(LineHeight::Absolute(line_height.into()))
            .horizontal_alignment(Horizontal::Right);

        let gutter = Container::new(gutter_content)
            .width(Length::Fixed(gutter_width))
            .height(Length::Fill)
            .padding([vertical_padding, 8.0])
            .clip(true)
            .style(theme::Container::Custom(styles::gutter(&self.theme)));

        let editor = text_editor(&self.tab().content)
            .on_action(Message::Edit)
            .font(Font::MONOSPACE)
            .line_height(LineHeight::Absolute(line_height.into()))
            .padding([vertical_padding, 16.0])
            .height(Length::Fill)
            .highlight::<highlight::RoxanneHighlighter>(
                self.tab().highlight_settings.clone(),
                highlight::highlight_format,
            );

        let editor = Container::new(editor)
            .width(Length::Fill)
            .height(Length::Fill)
            .id(editor_container_id())
            .style(theme::Container::Custom(styles::editor(&self.theme)));

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

        let replace_row = row![
            TextInput::new("Remplacer par…", &self.replace_text)
                .on_input(Message::ReplaceChanged)
                .padding([4, 8])
                .size(12),
            Button::new(text("Remplacer").size(12).font(Font::MONOSPACE))
                .padding([4, 10])
                .style(theme::Button::Custom(styles::submenu_button(&self.theme)))
                .on_press(Message::ReplaceCurrent),
            Button::new(text("Tout remplacer").size(12).font(Font::MONOSPACE))
                .padding([4, 10])
                .style(theme::Button::Custom(styles::submenu_button(&self.theme)))
                .on_press(Message::ReplaceAll),
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
                .style(theme::Button::Custom(styles::submenu_button(&self.theme)))
                .on_press(Message::SearchInFiles),
            Button::new(text("Effacer résultats").size(12).font(Font::MONOSPACE))
                .padding([4, 10])
                .style(theme::Button::Custom(styles::submenu_button(&self.theme)))
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
                            .style(theme::Container::Custom(styles::panel_item(&self.theme))),
                    )
                    .style(theme::Button::Custom(styles::search_result_button(&self.theme)))
                    .on_press(Message::SearchResultSelected(index))
                    .into()
                })
                .collect::<Vec<Element<Message>>>();

            Scrollable::new(column(entries).spacing(4))
                .height(Length::Fixed(180.0))
                .into()
        };

        let panel = column![header, query_row, replace_row, scope_row, action_row, results]
            .spacing(10)
            .padding([8, 16]);

        Some(
            Container::new(panel)
                .width(Length::Fill)
                .style(theme::Container::Custom(styles::panel(&self.theme)))
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
                .style(theme::Button::Custom(styles::submenu_button(&self.theme)))
                .on_press(Message::GotoLineSubmit),
            Button::new(text("Fermer").size(12).font(Font::MONOSPACE))
                .padding([4, 10])
                .style(theme::Button::Custom(styles::submenu_button(&self.theme)))
                .on_press(Message::GotoLineClosed),
        ]
        .spacing(8)
        .align_items(Alignment::Center);

        let panel = column![header, input, actions].spacing(10).padding([8, 16]);

        Some(
            Container::new(panel)
                .width(Length::Fill)
                .style(theme::Container::Custom(styles::panel(&self.theme)))
                .into(),
        )
    }

    fn diagnostics_panel(&self) -> Option<Element<'_, Message>> {
        if !self.diagnostics_panel_open {
            return None;
        }

        let header = row![
            text("Diagnostics (brackets)")
                .size(12)
                .font(Font::MONOSPACE)
                .style(Color::from_rgb8(220, 220, 220)),
            text(format!("{} élément(s)", self.tab().diagnostics.len()))
                .size(12)
                .font(Font::MONOSPACE)
                .style(Color::from_rgb8(160, 160, 160)),
        ]
        .spacing(12)
        .align_items(Alignment::Center);

        let content: Element<Message> = if self.tab().diagnostics.is_empty() {
            text("Aucun diagnostic.")
                .size(12)
                .font(Font::MONOSPACE)
                .style(theme::Text::Color(Color::from_rgb8(150, 150, 150)))
                .into()
        } else {
            let entries = self
                .tab()
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
                        .style(theme::Container::Custom(styles::panel_item(&self.theme)))
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
                .style(theme::Container::Custom(styles::panel(&self.theme)))
                .into(),
        )
    }

    fn completion_panel(&self) -> Option<Element<'_, Message>> {
        if !self.completion_panel_open {
            return None;
        }

        let header = row![
            text("Complétions (mots-clés)")
                .size(12)
                .font(Font::MONOSPACE)
                .style(Color::from_rgb8(220, 220, 220)),
            text(format!("Préfixe: {}", self.tab().completion_prefix))
                .size(12)
                .font(Font::MONOSPACE)
                .style(Color::from_rgb8(160, 160, 160)),
        ]
        .spacing(12)
        .align_items(Alignment::Center);

        let list: Element<Message> = if self.tab().completion_items.is_empty() {
            text("Aucune suggestion.")
                .size(12)
                .font(Font::MONOSPACE)
                .style(theme::Text::Color(Color::from_rgb8(150, 150, 150)))
                .into()
        } else {
            let entries = self
                .tab()
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
                            .style(theme::Container::Custom(styles::panel_item(&self.theme))),
                    )
                    .style(theme::Button::Custom(styles::search_result_button(&self.theme)))
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
                .style(theme::Container::Custom(styles::panel(&self.theme)))
                .into(),
        )
    }

    fn status_bar(&self) -> Element<'_, Message> {
        let tab = self.tab();
        let is_modified = tab.is_modified();
        let has_matches = !tab.search_matches.is_empty();
        let cursor_position = tab.content.cursor_position();
        let cursor_count = tab.multi_cursors.len() + 1;
        let diagnostics_count = tab.diagnostics.len();
        let left = row![
            text(format!(
                "{}{}",
                tab.filename,
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
                .style(theme::Button::Custom(styles::submenu_button(&self.theme)))
                .on_press_maybe(has_matches.then_some(Message::SearchPrevious)),
            Button::new(text("▶").size(12).font(Font::MONOSPACE))
                .padding([2, 6])
                .style(theme::Button::Custom(styles::submenu_button(&self.theme)))
                .on_press_maybe(has_matches.then_some(Message::SearchNext)),
            TextInput::new("fichier…", &tab.filename)
                .on_input(Message::FilenameChanged)
                .padding([2, 8])
                .size(12),
        ]
        .spacing(12)
        .align_items(Alignment::Center);

        let matches = tab.search_matches.len();
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
            .style(theme::Container::Custom(styles::status_bar(&self.theme)))
            .into()
    }

    fn toggle_button(&self, label: &str, active: bool, message: Message) -> Element<'_, Message> {
        Button::new(text(label).size(12).font(Font::MONOSPACE))
            .padding([2, 6])
            .style(theme::Button::Custom(styles::toggle_button(&self.theme, active)))
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
        .style(theme::Button::Custom(styles::toggle_button(&self.theme, active)))
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
        let ct = self.tab().content.text().to_string();
        let cf = self.tab().filename.clone();
        plugins.on_text_changed(&ct, &cf);
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
        if let Some(root) = Self::workspace_root_from_filename(&self.tab().filename) {
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
                self.tab().search_matches.len(),
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
            search::search_in_workspace(root, query, options),
            Message::SearchResultsLoaded,
        )
    }

    fn refresh_search_matches(&mut self, preserve_index: bool, update_status: bool) {
        let options = self.search_options();
        let matches = search::find_matches(&self.tab().buffer, &self.search_query, options);
        match matches {
            Ok(m) => self.tab_mut().search_matches = m,
            Err(message) => {
                self.tab_mut().current_match_index = None;
                self.tab_mut().search_matches.clear();
                self.tab_mut().highlight_settings.search_matches.clear();
                if update_status && !self.search_query.is_empty() {
                    self.status_message = Some(format!("Recherche: {message}"));
                }
                return;
            }
        };
        let matches_clone = self.tab().search_matches.clone();
        self.tab_mut().highlight_settings.search_matches = matches_clone;
        if self.tab().search_matches.is_empty() {
            self.tab_mut().current_match_index = None;
            if update_status && !self.search_query.is_empty() {
                self.status_message = Some("Recherche: aucune occurrence.".to_string());
            }
            return;
        }

        let len = self.tab().search_matches.len();
        self.tab_mut().current_match_index = if preserve_index {
            self.tab().current_match_index
                .filter(|index| *index < len)
        } else {
            None
        };

        if update_status && !self.search_query.is_empty() {
            self.status_message = Some(format!(
                "Recherche: {} occurrence(s).",
                len
            ));
        }
    }

    fn find_next_match(&mut self, forward: bool) -> Command<Message> {
        if self.search_query.trim().is_empty() {
            self.status_message = Some("Recherche: saisissez un terme.".to_string());
            return Command::none();
        }

        if self.tab().search_matches.is_empty() {
            self.status_message = Some("Recherche: aucune occurrence.".to_string());
            return Command::none();
        }

        let total = self.tab().search_matches.len();
        let next_index = match self.tab().current_match_index {
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

        self.tab_mut().current_match_index = Some(next_index);
        self.jump_to_match(next_index);
        Command::none()
    }

    fn jump_to_match(&mut self, index: usize) {
        if let Some(match_position) = self.tab().search_matches.get(index).copied() {
            let line = match_position.line;
            let column = match_position.column;
            self.jump_to_position(line, column);
            self.status_message = Some(format!(
                "Recherche: {}/{} (ligne {}, colonne {}).",
                index + 1,
                self.tab().search_matches.len(),
                line + 1,
                column + 1
            ));
        }
    }

    fn all_commands() -> Vec<(&'static str, MenuAction)> {
        vec![
            ("Open File — Ctrl+O", MenuAction::Open),
            ("Save — Ctrl+S", MenuAction::Save),
            ("Find — Ctrl+F", MenuAction::Find),
            ("Find Next — F3", MenuAction::FindNext),
            ("Find Previous — Shift+F3", MenuAction::FindPrevious),
            ("Find in Files", MenuAction::FindInFiles),
            ("Select All — Ctrl+A", MenuAction::SelectAll),
            ("Add Cursor Next Match", MenuAction::AddCursorNextMatch),
            ("Add Cursors All Matches", MenuAction::AddCursorsAllMatches),
            ("Clear Multi Cursors", MenuAction::ClearMultiCursors),
            ("Toggle Diagnostics", MenuAction::ToggleDiagnosticsPanel),
            ("Toggle Search Panel", MenuAction::ToggleSearchPanel),
            ("Go To Line — Ctrl+G", MenuAction::GoToLine),
            ("Reload Config", MenuAction::ReloadConfig),
            ("Export Theme", MenuAction::ExportTheme),
            ("Import Theme", MenuAction::ImportTheme),
            ("Performance Report", MenuAction::PerformanceReport),
            ("Toggle File Tree — Ctrl+B", MenuAction::ToggleFileTree),
            ("About", MenuAction::About),
        ]
    }

    fn filtered_commands(&self) -> Vec<(&'static str, MenuAction)> {
        let query = self.command_palette_query.to_lowercase();
        if query.is_empty() {
            return Self::all_commands();
        }
        Self::all_commands()
            .into_iter()
            .filter(|(label, _)| label.to_lowercase().contains(&query))
            .collect()
    }

    fn command_palette(&self) -> Option<Element<'_, Message>> {
        if !self.command_palette_open {
            return None;
        }

        let commands = self.filtered_commands();

        let input = TextInput::new("Commande…", &self.command_palette_query)
            .on_input(Message::CommandPaletteChanged)
            .padding([6, 12])
            .size(14);

        let entries: Vec<Element<Message>> = commands
            .iter()
            .enumerate()
            .take(15)
            .map(|(index, (label, _))| {
                Button::new(
                    text(*label)
                        .size(13)
                        .font(Font::MONOSPACE)
                        .style(Color::from_rgb8(220, 220, 220)),
                )
                .width(Length::Fill)
                .padding([6, 16])
                .style(theme::Button::Custom(styles::search_result_button(&self.theme)))
                .on_press(Message::CommandPaletteSelected(index))
                .into()
            })
            .collect();

        let list: Element<Message> = if entries.is_empty() {
            text("Aucune commande.")
                .size(12)
                .font(Font::MONOSPACE)
                .style(theme::Text::Color(Color::from_rgb8(150, 150, 150)))
                .into()
        } else {
            Scrollable::new(column(entries).spacing(2))
                .height(Length::Fixed(300.0))
                .into()
        };

        let panel = column![input, list].spacing(8).padding([12, 16]);

        Some(
            Container::new(panel)
                .width(Length::Fixed(500.0))
                .style(theme::Container::Custom(styles::panel(&self.theme)))
                .into(),
        )
    }

    fn file_tree_panel(&self) -> Option<Element<'_, Message>> {
        if !self.file_tree_open {
            return None;
        }
        let tree = self.file_tree.as_ref()?;
        let entries = tree.flatten_visible();

        let mut items: Vec<Element<'_, Message>> = Vec::new();
        for (depth, entry) in entries.iter().take(FILE_TREE_MAX_ENTRIES) {
            let indent = "  ".repeat(*depth);
            let icon = if entry.is_dir {
                if entry.expanded { "v " } else { "> " }
            } else {
                "  "
            };
            let label = format!("{indent}{icon}{}", entry.name);
            let btn = if entry.is_dir {
                let p = entry.path.clone();
                Button::new(
                    text(label).size(12).font(Font::MONOSPACE),
                )
                .width(Length::Fill)
                .padding([1, 4])
                .style(theme::Button::Custom(styles::submenu_button(&self.theme)))
                .on_press(Message::FileTreeToggleDir(p))
            } else {
                let p = entry.path.clone();
                Button::new(
                    text(label).size(12).font(Font::MONOSPACE),
                )
                .width(Length::Fill)
                .padding([1, 4])
                .style(theme::Button::Custom(styles::submenu_button(&self.theme)))
                .on_press(Message::FileTreeFileClicked(p))
            };
            items.push(btn.into());
        }

        let tree_column = Column::with_children(items).spacing(0).width(Length::Fill);

        Some(
            Container::new(
                Scrollable::new(tree_column)
                    .height(Length::Fill)
                    .width(Length::Fixed(220.0)),
            )
            .style(theme::Container::Custom(styles::submenu(&self.theme)))
            .height(Length::Fill)
            .width(Length::Fixed(220.0))
            .into(),
        )
    }

    fn replace_current_match(&mut self) {
        let match_index = match self.tab().current_match_index {
            Some(idx) => idx,
            None => {
                self.status_message = Some("Remplacement: aucune occurrence sélectionnée.".to_string());
                return;
            }
        };
        let Some(m) = self.tab().search_matches.get(match_index).copied() else {
            return;
        };
        let start = Position::new(m.line, m.column);
        let end_col = m.column + m.length;
        let end = Position::new(m.line, end_col);
        self.record_undo_snapshot();
        let replace = self.replace_text.clone();
        self.tab_mut().buffer.delete_range(start, end);
        self.tab_mut().buffer.insert(start, &replace);
        let text = self.tab().buffer.text().to_string();
        self.tab_mut().content = EditorContent::with_text(&text);
        self.sync_highlight_buffer();
        self.refresh_search_matches(false, true);
        self.refresh_diagnostics();
        let ct = self.tab().content.text().to_string();
        let cf = self.tab().filename.clone();
        self.plugins.on_text_changed(&ct, &cf);
        self.refresh_viewport_cache();
        self.status_message = Some("Remplacement: 1 occurrence remplacée.".to_string());
    }

    fn replace_all_matches(&mut self) {
        if self.tab().search_matches.is_empty() {
            self.status_message = Some("Remplacement: aucune occurrence.".to_string());
            return;
        }
        self.record_undo_snapshot();
        // Replace in reverse order to preserve positions
        let mut matches: Vec<_> = self.tab().search_matches.clone();
        matches.sort_by(|a, b| {
            b.line.cmp(&a.line).then(b.column.cmp(&a.column))
        });
        let replace = self.replace_text.clone();
        let count = matches.len();
        for m in &matches {
            let start = Position::new(m.line, m.column);
            let end = Position::new(m.line, m.column + m.length);
            self.tab_mut().buffer.delete_range(start, end);
            self.tab_mut().buffer.insert(start, &replace);
        }
        let text = self.tab().buffer.text().to_string();
        self.tab_mut().content = EditorContent::with_text(&text);
        self.sync_highlight_buffer();
        self.refresh_search_matches(false, true);
        self.refresh_diagnostics();
        let ct = self.tab().content.text().to_string();
        let cf = self.tab().filename.clone();
        self.plugins.on_text_changed(&ct, &cf);
        self.refresh_viewport_cache();
        self.status_message = Some(format!("Remplacement: {} occurrence(s) remplacée(s).", count));
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
        let max_line = self.tab().buffer.line_count().saturating_sub(1);
        let clamped_line = line.min(max_line);
        let clamped_column = self
            .tab()
            .buffer
            .line(clamped_line)
            .map(|line| line.chars().count())
            .unwrap_or(0)
            .min(column);
        self.move_cursor_to(clamped_line, clamped_column);
    }

    fn move_cursor_to(&mut self, line: usize, column: usize) {
        let (cur_line, _cur_col) = self.tab().content.cursor_position();

        if line != cur_line {
            if line == 0 {
                self.tab_mut().content
                    .perform(EditorAction::Move(Motion::DocumentStart));
            } else if line < cur_line {
                for _ in 0..(cur_line - line) {
                    self.tab_mut().content.perform(EditorAction::Move(Motion::Up));
                }
            } else {
                for _ in 0..(line - cur_line) {
                    self.tab_mut().content.perform(EditorAction::Move(Motion::Down));
                }
            }
        }

        self.tab_mut().content.perform(EditorAction::Move(Motion::Home));
        for _ in 0..column {
            self.tab_mut().content.perform(EditorAction::Move(Motion::Right));
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
        if self.tab().search_matches.is_empty() {
            self.status_message = Some("Multi-curseurs: aucune occurrence.".to_string());
            return;
        }

        let (line, column) = self.tab().content.cursor_position();
        let next_match = self
            .tab()
            .search_matches
            .iter()
            .find(|match_position| {
                match_position.line > line
                    || (match_position.line == line && match_position.column > column)
            })
            .or_else(|| self.tab().search_matches.first())
            .copied();

        if let Some(match_position) = next_match {
            let position = Position::new(match_position.line, match_position.column);
            if !self.tab().multi_cursors.contains(&position) {
                self.tab_mut().multi_cursors.push(position);
            }
            self.status_message = Some("Multi-curseurs: ajout d'une position.".to_string());
        }
    }

    fn add_cursors_all_matches(&mut self) {
        if self.tab().search_matches.is_empty() {
            self.status_message = Some("Multi-curseurs: aucune occurrence.".to_string());
            return;
        }
        let (line, column) = self.tab().content.cursor_position();
        let primary = Position::new(line, column);
        let cursors: Vec<Position> = self
            .tab()
            .search_matches
            .iter()
            .map(|match_position| Position::new(match_position.line, match_position.column))
            .filter(|position| *position != primary)
            .collect();
        let count = cursors.len();
        self.tab_mut().multi_cursors = cursors;
        self.status_message = Some(format!(
            "Multi-curseurs: {} position(s).",
            count
        ));
    }

    fn apply_multi_cursor_edit(&mut self, edit: &EditorEdit) {
        let (line, column) = self.tab().content.cursor_position();
        let mut cursor_positions = Vec::with_capacity(self.tab().multi_cursors.len() + 1);
        cursor_positions.push(Position::new(line, column));
        cursor_positions.extend(self.tab().multi_cursors.iter().copied());

        let mut seen_indices = HashSet::new();
        let mut cursor_slots = Vec::new();
        for (id, position) in cursor_positions.iter().copied().enumerate() {
            let index = self.tab().buffer.index_from_position(position);
            if seen_indices.insert(index) {
                cursor_slots.push(CursorSlot {
                    id,
                    _position: position,
                    index,
                });
            }
        }

        cursor_slots.sort_by_key(|slot| slot.index);
        let text = self.tab().buffer.text().to_string();

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
        self.tab_mut().buffer.replace(&new_text);
        let bt = self.tab().buffer.text().to_string();
        self.tab_mut().content = EditorContent::with_text(&bt);
        self.sync_highlight_buffer();

        let new_indices = compute_new_indices(&cursor_slots, &filtered);
        let mut new_positions = Vec::new();
        for (index, position) in cursor_positions.iter().enumerate() {
            let new_index = new_indices
                .get(&index)
                .copied()
                .unwrap_or_else(|| self.tab().buffer.index_from_position(*position));
            new_positions.push(self.tab().buffer.position_from_index(new_index));
        }

        if let Some(primary) = new_positions.first() {
            self.move_cursor_to(primary.line, primary.column);
        }
        self.tab_mut().multi_cursors = new_positions.into_iter().skip(1).collect();
        let applied_cursors = self.tab().multi_cursors.len() + 1;
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
        if self.tab().suppress_undo_snapshot {
            return;
        }
        self.tab_mut().buffer.record_snapshot();
    }

    fn apply_undo(&mut self) {
        if !self.tab_mut().buffer.undo() {
            return;
        };
        let text = self.tab().buffer.text().to_string();
        self.apply_snapshot(&text);
    }

    fn apply_redo(&mut self) {
        if !self.tab_mut().buffer.redo() {
            return;
        };
        let text = self.tab().buffer.text().to_string();
        self.apply_snapshot(&text);
    }

    fn apply_snapshot(&mut self, text: &str) {
        self.tab_mut().suppress_undo_snapshot = true;
        self.tab_mut().content = EditorContent::with_text(text);
        self.tab_mut().buffer.replace(text);
        self.sync_highlight_buffer();
        self.refresh_search_matches(true, true);
        self.refresh_diagnostics();
        let ct = self.tab().content.text().to_string();
        let cf = self.tab().filename.clone();
        self.plugins.on_text_changed(&ct, &cf);
        self.refresh_viewport_cache();
        self.tab_mut().suppress_undo_snapshot = false;
    }

    fn sync_highlight_buffer(&mut self) {
        let text: std::sync::Arc<str> = self.tab().content.text().into();
        self.tab_mut().highlight_settings.buffer_text = text;
    }

    fn refresh_viewport_cache(&mut self) {
        let (line, _) = self.tab().content.cursor_position();
        let start_line = line.saturating_sub(self.viewport_height / 2);
        let started = Instant::now();
        let vh = self.viewport_height;
        let tab = self.tab_mut();
        tab.viewport_cache.update(&tab.buffer, start_line, vh);
        self.performance.last_viewport_refresh = Some(started.elapsed());
    }

    fn viewport_label(&self) -> String {
        match self.tab().viewport_cache.range() {
            Some((start, end)) => format!("Viewport: {}-{}", start + 1, end),
            None => "Viewport: -".to_string(),
        }
    }

    fn refresh_diagnostics(&mut self) {
        let diags = diagnostics::analyze(&self.tab().buffer);
        self.tab_mut().diagnostics = diags;
    }

    fn refresh_completions(&mut self) {
        let (line, column) = self.tab().content.cursor_position();
        let line_text = self.tab().buffer.line(line).unwrap_or("").to_string();
        let column = column.min(line_text.chars().count());
        let prefix = completion::extract_prefix(&line_text, column);
        let items = completion::build_items(&prefix);
        let count = items.len();
        let tab = self.tab_mut();
        tab.completion_prefix = prefix;
        tab.completion_items = items;
        if count == 0 {
            self.status_message = Some("Complétions: aucune suggestion.".to_string());
        } else {
            self.status_message = Some(format!(
                "Complétions: {} suggestion(s).",
                count
            ));
        }
    }

    fn apply_completion(&mut self, index: usize) {
        let Some(item) = self.tab().completion_items.get(index).cloned() else {
            return;
        };

        let remainder = item
            .label
            .strip_prefix(&self.tab().completion_prefix)
            .unwrap_or(&item.label)
            .to_string();
        if remainder.is_empty() {
            return;
        }

        let insert = EditorEdit::Paste(std::sync::Arc::new(remainder));
        if self.tab().multi_cursors.is_empty() {
            self.record_undo_snapshot();
            self.tab_mut().content.perform(EditorAction::Edit(insert.clone()));
            let t = self.tab().content.text();
            self.tab_mut().buffer.replace(&t);
        } else {
            self.record_undo_snapshot();
            self.apply_multi_cursor_edit(&insert);
        }
        self.sync_highlight_buffer();
        self.refresh_search_matches(true, true);
        self.refresh_diagnostics();
        let ct = self.tab().content.text().to_string();
        let cf = self.tab().filename.clone();
        self.plugins.on_text_changed(&ct, &cf);
        self.refresh_viewport_cache();
        self.completion_panel_open = false;
        self.tab_mut().completion_items.clear();
    }
}

fn config_watcher_subscription() -> Subscription<Message> {
    use std::sync::{Arc, Mutex};

    struct WatcherState {
        changed: Arc<Mutex<bool>>,
        _watcher: Option<notify::RecommendedWatcher>,
    }

    subscription::channel(
        std::any::TypeId::of::<ConfigWatcherId>(),
        16,
        |mut output| async move {
            use iced::futures::SinkExt;
            use notify::{RecursiveMode, Watcher};

            let changed = Arc::new(Mutex::new(false));
            let changed_clone = changed.clone();

            let watcher = notify::recommended_watcher(
                move |event: Result<notify::Event, notify::Error>| {
                    if let Ok(event) = event {
                        if matches!(
                            event.kind,
                            notify::EventKind::Modify(_) | notify::EventKind::Create(_)
                        ) {
                            if let Ok(mut flag) = changed_clone.lock() {
                                *flag = true;
                            }
                        }
                    }
                },
            )
            .ok()
            .map(|mut w| {
                for path in config::watch_paths() {
                    let _ = w.watch(&path, RecursiveMode::NonRecursive);
                }
                w
            });

            let _state = WatcherState {
                changed: changed.clone(),
                _watcher: watcher,
            };

            loop {
                // Use iced's time subscription internally would be ideal,
                // but in a channel we poll with a blocking sleep on a thread.
                let changed_ref = changed.clone();
                let did_change = iced::futures::future::poll_fn(|_cx| {
                    if let Ok(mut flag) = changed_ref.lock() {
                        if *flag {
                            *flag = false;
                            return std::task::Poll::Ready(true);
                        }
                    }
                    // Use a waker to re-poll after a delay
                    let waker = _cx.waker().clone();
                    std::thread::spawn(move || {
                        std::thread::sleep(std::time::Duration::from_secs(1));
                        waker.wake();
                    });
                    std::task::Poll::Pending
                })
                .await;

                if did_change {
                    let _ = output.send(Message::ConfigFileChanged).await;
                }
            }
        },
    )
}

struct ConfigWatcherId;

#[cfg(test)]
mod tests {
    use super::RoxanneApp;
    use crate::diagnostics;
    use crate::editor::TextBuffer;
    use crate::search::{self, SearchOptions};
    use std::fs;
    use tempfile::tempdir;


    #[test]
    fn search_and_diagnostics_use_character_columns() {
        let text = "aé👍b\n🙂}";
        let buffer = TextBuffer::from(text);
        let options = SearchOptions {
            regex: false,
            case_sensitive: true,
            include_hidden: false,
        };

        let matches = search::find_matches(&buffer, "👍", options).expect("matches");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].line, 0);
        assert_eq!(matches[0].column, 2);
        assert_eq!(matches[0].length, 1);

        let diagnostics = diagnostics::analyze(&buffer);
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

        let matches = search::find_matches(&buffer, "a", options).expect("matches");
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

        assert!(search::should_skip_entry(&hidden_entry, false));
        assert!(!search::should_skip_entry(&hidden_entry, true));
        assert!(search::should_skip_file(&hidden_path, false));
        assert!(!search::should_skip_file(&hidden_path, true));
    }

    #[test]
    fn diagnostics_ignore_escaped_quotes() {
        let text = r#"let value = "hello\"world";"#;
        let buffer = TextBuffer::from(text);

        let diagnostics = diagnostics::analyze(&buffer);
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

        let diagnostics = diagnostics::analyze(&buffer);
        assert!(
            diagnostics.is_empty(),
            "string/comment delimiters should be ignored: {diagnostics:?}"
        );
    }

    #[test]
    fn diagnostics_ignore_comment_delimiters() {
        let text = "fn main() { // }\n    let value = \"{\";\n}";
        let buffer = TextBuffer::from(text);

        let diagnostics = diagnostics::analyze(&buffer);
        assert!(
            diagnostics.is_empty(),
            "comment delimiters should be ignored: {diagnostics:?}"
        );
    }

    #[test]
    fn diagnostics_ignore_block_comment_delimiters() {
        let text = "fn main() {\n    /* { [ ( */\n    let value = 1;\n}\n";
        let buffer = TextBuffer::from(text);

        let diagnostics = diagnostics::analyze(&buffer);
        assert!(
            diagnostics.is_empty(),
            "block comment delimiters should be ignored: {diagnostics:?}"
        );
    }

    #[test]
    fn diagnostics_ignore_nested_block_comment_delimiters() {
        let text = "fn main() {\n    /* outer { [ /* inner ( ) */ still ] } */\n}\n";
        let buffer = TextBuffer::from(text);

        let diagnostics = diagnostics::analyze(&buffer);
        assert!(
            diagnostics.is_empty(),
            "nested block comment delimiters should be ignored: {diagnostics:?}"
        );
    }

    #[test]
    fn diagnostics_allow_multiline_strings() {
        let text = "fn main() {\n    let value = \"multi\nline\";\n}\n";
        let buffer = TextBuffer::from(text);

        let diagnostics = diagnostics::analyze(&buffer);
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

        let diagnostics = diagnostics::analyze(&buffer);
        assert!(
            diagnostics.is_empty(),
            "char literal braces should be ignored: {diagnostics:?}"
        );
    }

    #[test]
    fn diagnostics_ignore_raw_string_with_braces_and_quotes() {
        let text = r##"fn main() { let value = r#"{ "quoted" }"#; }"##;
        let buffer = TextBuffer::from(text);

        let diagnostics = diagnostics::analyze(&buffer);
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

