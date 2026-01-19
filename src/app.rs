use crate::editor::highlight::MatchPosition;
use crate::editor::{TextBuffer, highlight};
use iced::alignment::{Horizontal, Vertical};
use iced::theme;
use iced::widget::button;
use iced::widget::text_editor::{Action as EditorAction, Content as EditorContent, Motion};
use iced::widget::{
    Button, Container, Row, Scrollable, TextInput, column, container, row, text, text_editor,
};
use iced::{
    Alignment, Application, Background, Color, Command, Element, Font, Length, Settings,
    Subscription, Theme, event, executor, keyboard,
};
use regex::{Regex, RegexBuilder};
use std::path::{Path, PathBuf};
use walkdir::{DirEntry, WalkDir};

#[derive(Debug)]
pub struct RoxanneApp {
    filename: String,
    content: EditorContent,
    buffer: TextBuffer,
    last_saved_text: String,
    search_query: String,
    search_matches: Vec<MatchPosition>,
    current_match_index: Option<usize>,
    search_case_sensitive: bool,
    search_regex: bool,
    search_panel_open: bool,
    search_scope: SearchScope,
    search_results: Vec<SearchResult>,
    highlight_settings: highlight::Settings,
    status_message: Option<String>,
    active_menu: Option<Menu>,
}

#[derive(Debug, Clone)]
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
    SearchInFiles,
    SearchResultsLoaded(Result<Vec<SearchResult>, String>),
    SearchResultOpened(Result<(String, SearchResult), String>),
    SearchResultsCleared,
    FilenameChanged(String),
    OpenPressed,
    SavePressed,
    MenuSelected(Menu),
    MenuAction(MenuAction),
    FileLoaded(Result<String, String>),
    FileSaved(Result<(), String>),
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
    Find,
    FindNext,
    FindPrevious,
    FindInFiles,
    SelectAll,
    ToggleStatusBar,
    ToggleSearchPanel,
    GoToLine,
    ToolsSettings,
    About,
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
    pub fn run() -> iced::Result {
        RoxanneApp::run_with(Settings::default())
    }

    pub fn run_with(settings: Settings<()>) -> iced::Result {
        <Self as Application>::run(settings)
    }
}

impl Application for RoxanneApp {
    type Executor = executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {
        let initial_text = "Roxanne – éditeur en mode Iced\n\n\
            Objectif: MVP inspiré de Sublime Text\n\
            - Menu bar, tabs, status bar\n\
            - Zone d'édition monospace";
        let buffer = TextBuffer::from(initial_text);
        (
            Self {
                filename: "untitled.txt".to_string(),
                content: EditorContent::with_text(initial_text),
                buffer,
                last_saved_text: initial_text.to_string(),
                search_query: String::new(),
                search_matches: Vec::new(),
                current_match_index: None,
                search_case_sensitive: false,
                search_regex: false,
                search_panel_open: false,
                search_scope: SearchScope::CurrentFile,
                search_results: Vec::new(),
                highlight_settings: highlight::Settings::default(),
                status_message: None,
                active_menu: None,
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
                self.content.perform(action);
                self.buffer.replace(&self.content.text());
                self.refresh_search_matches(true);
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
            Message::SearchInFiles => self.search_in_files(),
            Message::SearchResultsLoaded(result) => {
                match result {
                    Ok(results) => {
                        self.search_results = results;
                        self.status_message = Some(format!(
                            "Recherche fichiers: {} résultat(s).",
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
                        self.status_message =
                            Some("Aller à: fonctionnalité à venir (ligne).".to_string());
                        Command::none()
                    }
                    MenuAction::ToolsSettings => {
                        self.status_message =
                            Some("Outils: paramètres disponibles prochainement.".to_string());
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
                match result {
                    Ok(text) => {
                        self.content = EditorContent::with_text(&text);
                        self.buffer.replace(&text);
                        self.last_saved_text = text;
                        self.refresh_search_matches(false);
                        self.status_message = Some("Fichier chargé.".to_string());
                    }
                    Err(message) => {
                        self.status_message = Some(format!("Erreur d'ouverture: {message}"));
                    }
                }
                Command::none()
            }
            Message::FileSaved(result) => {
                match result {
                    Ok(()) => {
                        self.last_saved_text = self.content.text().to_string();
                        self.status_message = Some("Fichier sauvegardé.".to_string());
                    }
                    Err(message) => {
                        self.status_message = Some(format!("Erreur de sauvegarde: {message}"));
                    }
                }
                Command::none()
            }
        }
    }

    fn view(&self) -> Element<Message> {
        let menu_bar = self.menu_bar();
        let tab_bar = self.tab_bar();
        let search_panel = self.search_panel();
        let editor = self.editor_area();
        let status_bar = self.status_bar();

        let mut content = column![menu_bar, tab_bar];
        if let Some(panel) = search_panel {
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
            .style(theme::Container::Custom(Box::new(AppBackground)))
            .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        event::listen_with(|event, status| {
            if status == event::Status::Captured {
                return None;
            }

            match event {
                event::Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) => {
                    let key = key.as_ref();
                    if modifiers.command() {
                        match key {
                            keyboard::Key::Character("s") => Some(Message::SavePressed),
                            keyboard::Key::Character("o") => Some(Message::OpenPressed),
                            keyboard::Key::Character("f") => {
                                Some(Message::MenuAction(MenuAction::Find))
                            }
                            _ => None,
                        }
                    } else {
                        match key {
                            keyboard::Key::Named(keyboard::key::Named::F3) => {
                                if modifiers.shift() {
                                    Some(Message::SearchPrevious)
                                } else {
                                    Some(Message::SearchNext)
                                }
                            }
                            _ => None,
                        }
                    }
                }
                _ => None,
            }
        })
    }
}

impl RoxanneApp {
    fn open_file(&mut self) -> Command<Message> {
        if self.filename.trim().is_empty() {
            self.status_message = Some("Nom de fichier manquant.".to_string());
            return Command::none();
        }
        let filename = self.filename.clone();
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
        Command::perform(
            async move { std::fs::write(&filename, text).map_err(|err| err.to_string()) },
            Message::FileSaved,
        )
    }

    fn menu_bar(&self) -> Element<Message> {
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
            .style(theme::Container::Custom(Box::new(MenuBarStyle)));

        let mut column = column![top_row];
        if let Some(submenu) = self.submenu() {
            column = column.push(submenu);
        }

        Container::new(column).width(Length::Fill).into()
    }

    fn menu_button(&self, label: &str, menu: Menu) -> Element<Message> {
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
        })))
        .on_press(Message::MenuSelected(menu))
        .into()
    }

    fn submenu(&self) -> Option<Element<Message>> {
        let (label, actions) = match self.active_menu? {
            Menu::File => (
                "File",
                vec![("Open", MenuAction::Open), ("Save", MenuAction::Save)],
            ),
            Menu::Edit => (
                "Edit",
                vec![
                    ("Find", MenuAction::Find),
                    ("Find Next", MenuAction::FindNext),
                    ("Find Previous", MenuAction::FindPrevious),
                    ("Find in Files", MenuAction::FindInFiles),
                ],
            ),
            Menu::Selection => ("Selection", vec![("Select All", MenuAction::SelectAll)]),
            Menu::View => (
                "View",
                vec![
                    ("Status Bar", MenuAction::ToggleStatusBar),
                    ("Search Panel", MenuAction::ToggleSearchPanel),
                ],
            ),
            Menu::Goto => ("Goto", vec![("Go to Line", MenuAction::GoToLine)]),
            Menu::Tools => ("Tools", vec![("Settings", MenuAction::ToolsSettings)]),
            Menu::Help => ("Help", vec![("About", MenuAction::About)]),
        };

        let row = row![
            text(label)
                .size(12)
                .font(Font::MONOSPACE)
                .style(Color::from_rgb8(180, 180, 180)),
            Row::with_children(
                actions
                    .into_iter()
                    .map(|(name, action)| {
                        Button::new(text(name).size(12).font(Font::MONOSPACE))
                            .padding([2, 8])
                            .style(theme::Button::Custom(Box::new(SubmenuButtonStyle)))
                            .on_press(Message::MenuAction(action))
                            .into()
                    })
                    .collect::<Vec<Element<Message>>>(),
            )
            .spacing(8),
        ]
        .spacing(12)
        .align_items(Alignment::Center)
        .padding([4, 16]);

        Some(
            Container::new(row)
                .width(Length::Fill)
                .style(theme::Container::Custom(Box::new(SubmenuStyle)))
                .into(),
        )
    }

    fn tab_bar(&self) -> Element<Message> {
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
                .style(theme::Container::Custom(Box::new(ActiveTabStyle)))
                .height(Length::Fill)
        ]
        .spacing(4)
        .padding([0, 8])
        .align_items(Alignment::Center);

        Container::new(row)
            .width(Length::Fill)
            .height(Length::Fixed(32.0))
            .style(theme::Container::Custom(Box::new(TabBarStyle)))
            .into()
    }

    fn editor_area(&self) -> Element<Message> {
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
            .style(theme::Container::Custom(Box::new(EditorStyle)))
            .into()
    }

    fn search_panel(&self) -> Option<Element<Message>> {
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
                .style(theme::Button::Custom(Box::new(SubmenuButtonStyle)))
                .on_press(Message::SearchInFiles),
            Button::new(text("Effacer résultats").size(12).font(Font::MONOSPACE))
                .padding([4, 10])
                .style(theme::Button::Custom(Box::new(SubmenuButtonStyle)))
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
                            .style(theme::Container::Custom(Box::new(SearchResultStyle))),
                    )
                    .style(theme::Button::Custom(Box::new(SearchResultButtonStyle)))
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
                .style(theme::Container::Custom(Box::new(SearchPanelStyle)))
                .into(),
        )
    }

    fn status_bar(&self) -> Element<Message> {
        let is_modified = self.content.text() != self.last_saved_text;
        let has_matches = !self.search_matches.is_empty();
        let cursor_position = self.content.cursor_position();
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
                .style(theme::Button::Custom(Box::new(SubmenuButtonStyle)))
                .on_press_maybe(has_matches.then_some(Message::SearchPrevious)),
            Button::new(text("▶").size(12).font(Font::MONOSPACE))
                .padding([2, 6])
                .style(theme::Button::Custom(Box::new(SubmenuButtonStyle)))
                .on_press_maybe(has_matches.then_some(Message::SearchNext)),
            Button::new(text("Recherche +").size(12).font(Font::MONOSPACE))
                .padding([2, 8])
                .style(theme::Button::Custom(Box::new(SubmenuButtonStyle)))
                .on_press(Message::SearchPanelToggled),
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

        let right = text(format!(
            "Occurrences: {}   Ln {}, Col {}   UTF-8   LF   {}",
            matches,
            cursor_position.0 + 1,
            cursor_position.1 + 1,
            status_text
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
            .style(theme::Container::Custom(Box::new(StatusBarStyle)))
            .into()
    }

    fn toggle_button(&self, label: &str, active: bool, message: Message) -> Element<Message> {
        Button::new(text(label).size(12).font(Font::MONOSPACE))
            .padding([2, 6])
            .style(theme::Button::Custom(Box::new(ToggleButtonStyle {
                active,
            })))
            .on_press(message)
            .into()
    }

    fn search_scope_button(&self, scope: SearchScope) -> Element<Message> {
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
        })))
        .on_press(Message::SearchScopeSelected(scope))
        .into()
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
            self.refresh_search_matches(false);
            self.status_message = Some(format!(
                "Recherche fichier: {} occurrence(s).",
                self.search_matches.len()
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
    }
}

struct AppBackground;

impl container::StyleSheet for AppBackground {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(Color::from_rgb8(30, 30, 32))),
            text_color: None,
            border: Default::default(),
            shadow: Default::default(),
        }
    }
}

struct MenuBarStyle;

impl container::StyleSheet for MenuBarStyle {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(Color::from_rgb8(45, 45, 48))),
            text_color: None,
            border: Default::default(),
            shadow: Default::default(),
        }
    }
}

struct MenuButtonStyle {
    active: bool,
}

impl button::StyleSheet for MenuButtonStyle {
    type Style = Theme;

    fn active(&self, _style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: self
                .active
                .then(|| Background::Color(Color::from_rgb8(65, 65, 70))),
            text_color: Color::from_rgb8(220, 220, 220),
            border: Default::default(),
            shadow_offset: Default::default(),
            shadow: Default::default(),
        }
    }

    fn hovered(&self, style: &Self::Style) -> button::Appearance {
        let mut appearance = self.active(style);
        appearance.background = Some(Background::Color(Color::from_rgb8(70, 70, 74)));
        appearance
    }
}

struct SubmenuStyle;

impl container::StyleSheet for SubmenuStyle {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(Color::from_rgb8(40, 40, 44))),
            text_color: None,
            border: Default::default(),
            shadow: Default::default(),
        }
    }
}

struct SubmenuButtonStyle;

impl button::StyleSheet for SubmenuButtonStyle {
    type Style = Theme;

    fn active(&self, _style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(Color::from_rgb8(55, 55, 60))),
            text_color: Color::from_rgb8(230, 230, 230),
            border: Default::default(),
            shadow_offset: Default::default(),
            shadow: Default::default(),
        }
    }

    fn hovered(&self, style: &Self::Style) -> button::Appearance {
        let mut appearance = self.active(style);
        appearance.background = Some(Background::Color(Color::from_rgb8(65, 65, 70)));
        appearance
    }
}

struct ToggleButtonStyle {
    active: bool,
}

impl button::StyleSheet for ToggleButtonStyle {
    type Style = Theme;

    fn active(&self, _style: &Self::Style) -> button::Appearance {
        let background = if self.active {
            Background::Color(Color::from_rgb8(90, 90, 96))
        } else {
            Background::Color(Color::from_rgb8(55, 55, 60))
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
        appearance.background = Some(Background::Color(Color::from_rgb8(100, 100, 106)));
        appearance
    }
}

struct TabBarStyle;

impl container::StyleSheet for TabBarStyle {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(Color::from_rgb8(37, 37, 40))),
            text_color: None,
            border: Default::default(),
            shadow: Default::default(),
        }
    }
}

struct ActiveTabStyle;

impl container::StyleSheet for ActiveTabStyle {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(Color::from_rgb8(50, 50, 54))),
            text_color: None,
            border: Default::default(),
            shadow: Default::default(),
        }
    }
}

struct EditorStyle;

impl container::StyleSheet for EditorStyle {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(Color::from_rgb8(28, 28, 30))),
            text_color: None,
            border: Default::default(),
            shadow: Default::default(),
        }
    }
}

struct StatusBarStyle;

impl container::StyleSheet for StatusBarStyle {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(Color::from_rgb8(45, 45, 48))),
            text_color: None,
            border: Default::default(),
            shadow: Default::default(),
        }
    }
}

struct SearchPanelStyle;

impl container::StyleSheet for SearchPanelStyle {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(Color::from_rgb8(36, 36, 40))),
            text_color: None,
            border: Default::default(),
            shadow: Default::default(),
        }
    }
}

struct SearchResultStyle;

impl container::StyleSheet for SearchResultStyle {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(Color::from_rgb8(42, 42, 46))),
            text_color: None,
            border: Default::default(),
            shadow: Default::default(),
        }
    }
}

struct SearchResultButtonStyle;

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
        appearance.background = Some(Background::Color(Color::from_rgb8(60, 60, 66)));
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
                length,
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

struct SearchResultLine {
    line: usize,
    column: usize,
    length: usize,
    preview: String,
}
