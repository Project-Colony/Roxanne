use iced::alignment::{Horizontal, Vertical};
use iced::theme;
use iced::widget::text_editor::{Action as EditorAction, Content as EditorContent, Motion};
use iced::widget::button;
use iced::widget::{Button, Container, Row, TextInput, column, container, row, text, text_editor};
use iced::{
    Alignment, Application, Background, Color, Command, Element, Font, Length, Settings,
    Subscription, Theme, event, executor, keyboard,
};
use crate::editor::TextBuffer;

#[derive(Debug)]
pub struct RoxanneApp {
    filename: String,
    content: EditorContent,
    buffer: TextBuffer,
    last_saved_text: String,
    search_query: String,
    search_matches: Vec<(usize, usize)>,
    current_match_index: Option<usize>,
    status_message: Option<String>,
    active_menu: Option<Menu>,
}

#[derive(Debug, Clone)]
pub enum Message {
    Edit(EditorAction),
    SearchChanged(String),
    SearchNext,
    SearchPrevious,
    FilenameChanged(String),
    OpenPressed,
    SavePressed,
    MenuSelected(Menu),
    MenuAction(MenuAction),
    FileLoaded(Result<String, String>),
    FileSaved(Result<(), String>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Menu {
    File,
    Edit,
    Selection,
    View,
    Goto,
    Tools,
    Help,
}

#[derive(Debug, Clone, Copy)]
enum MenuAction {
    Open,
    Save,
    Find,
    FindNext,
    FindPrevious,
    SelectAll,
    ToggleStatusBar,
    GoToLine,
    ToolsSettings,
    About,
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
                self.refresh_search_matches(false);
                Command::none()
            }
            Message::SearchNext => {
                self.find_next_match(true)
            }
            Message::SearchPrevious => {
                self.find_next_match(false)
            }
            Message::FilenameChanged(value) => {
                self.filename = value;
                Command::none()
            }
            Message::OpenPressed => {
                self.open_file()
            }
            Message::SavePressed => {
                self.save_file()
            }
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
                    MenuAction::SelectAll => {
                        self.status_message =
                            Some("Sélection: Ctrl+A ou Cmd+A pour tout sélectionner.".to_string());
                        Command::none()
                    }
                    MenuAction::ToggleStatusBar => {
                        self.status_message = Some("Affichage: options avancées à venir.".to_string());
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
        let editor = self.editor_area();
        let status_bar = self.status_bar();

        let content = column![menu_bar, tab_bar, editor, status_bar]
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
                            keyboard::Key::Character("f") => Some(Message::MenuAction(MenuAction::Find)),
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

        Container::new(column)
            .width(Length::Fill)
            .into()
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
        .style(theme::Button::Custom(Box::new(MenuButtonStyle { active: is_active })))
        .on_press(Message::MenuSelected(menu))
        .into()
    }

    fn submenu(&self) -> Option<Element<Message>> {
        let (label, actions) = match self.active_menu? {
            Menu::File => (
                "File",
                vec![
                    ("Open", MenuAction::Open),
                    ("Save", MenuAction::Save),
                ],
            ),
            Menu::Edit => (
                "Edit",
                vec![
                    ("Find", MenuAction::Find),
                    ("Find Next", MenuAction::FindNext),
                    ("Find Previous", MenuAction::FindPrevious),
                ],
            ),
            Menu::Selection => ("Selection", vec![("Select All", MenuAction::SelectAll)]),
            Menu::View => (
                "View",
                vec![("Status Bar", MenuAction::ToggleStatusBar)],
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
            .padding([12, 16]);

        Container::new(editor)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(theme::Container::Custom(Box::new(EditorStyle)))
            .into()
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
            Button::new(text("◀").size(12).font(Font::MONOSPACE))
                .padding([2, 6])
                .style(theme::Button::Custom(Box::new(SubmenuButtonStyle)))
                .on_press_maybe(has_matches.then_some(Message::SearchPrevious)),
            Button::new(text("▶").size(12).font(Font::MONOSPACE))
                .padding([2, 6])
                .style(theme::Button::Custom(Box::new(SubmenuButtonStyle)))
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

    fn refresh_search_matches(&mut self, preserve_index: bool) {
        self.search_matches = find_matches(&self.buffer, &self.search_query);
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
        if let Some(&(line, column)) = self.search_matches.get(index) {
            self.move_cursor_to(line, column);
            self.status_message = Some(format!(
                "Recherche: {}/{} (ligne {}, colonne {}).",
                index + 1,
                self.search_matches.len(),
                line + 1,
                column + 1
            ));
        }
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

fn find_matches(buffer: &TextBuffer, needle: &str) -> Vec<(usize, usize)> {
    if needle.is_empty() {
        return Vec::new();
    }

    let mut matches = Vec::new();
    for (line_index, line) in buffer.lines().enumerate() {
        let mut search_start = 0;
        while let Some(found) = line[search_start..].find(needle) {
            let column = search_start + found;
            matches.push((line_index, column));
            search_start = column + needle.len();
        }
    }

    matches
}
