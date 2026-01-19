use iced::alignment::{Horizontal, Vertical};
use iced::theme;
use iced::widget::text_editor::{Action as EditorAction, Content as EditorContent};
use iced::widget::{Button, Container, Row, TextInput, column, container, row, text, text_editor};
use iced::{
    Alignment, Application, Background, Color, Command, Element, Font, Length, Settings, Theme,
    executor,
};

#[derive(Debug)]
pub struct RoxanneApp {
    filename: String,
    content: EditorContent,
    last_saved_text: String,
    search_query: String,
    status_message: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    Edit(EditorAction),
    SearchChanged(String),
    FilenameChanged(String),
    OpenPressed,
    SavePressed,
    FileLoaded(Result<String, String>),
    FileSaved(Result<(), String>),
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
        (
            Self {
                filename: "untitled.txt".to_string(),
                content: EditorContent::with_text(initial_text),
                last_saved_text: initial_text.to_string(),
                search_query: String::new(),
                status_message: None,
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
                Command::none()
            }
            Message::SearchChanged(value) => {
                self.search_query = value;
                Command::none()
            }
            Message::FilenameChanged(value) => {
                self.filename = value;
                Command::none()
            }
            Message::OpenPressed => {
                if self.filename.trim().is_empty() {
                    self.status_message = Some("Nom de fichier manquant.".to_string());
                    return Command::none();
                }
                let filename = self.filename.clone();
                Command::perform(
                    async move {
                        std::fs::read_to_string(&filename).map_err(|err| err.to_string())
                    },
                    Message::FileLoaded,
                )
            }
            Message::SavePressed => {
                if self.filename.trim().is_empty() {
                    self.status_message = Some("Nom de fichier manquant.".to_string());
                    return Command::none();
                }
                let filename = self.filename.clone();
                let text = self.content.text().to_string();
                Command::perform(
                    async move { std::fs::write(&filename, text).map_err(|err| err.to_string()) },
                    Message::FileSaved,
                )
            }
            Message::FileLoaded(result) => {
                match result {
                    Ok(text) => {
                        self.content = EditorContent::with_text(&text);
                        self.last_saved_text = text;
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
}

impl RoxanneApp {
    fn menu_bar(&self) -> Element<Message> {
        let items = ["File", "Edit", "Selection", "View", "Goto", "Tools", "Help"]
            .iter()
            .map(|label| {
                text(*label)
                    .size(14)
                    .style(Color::from_rgb8(220, 220, 220))
                    .font(Font::MONOSPACE)
                    .into()
            })
            .collect::<Vec<_>>();

        let file_controls = row![
            Button::new(text("Open").size(13).font(Font::MONOSPACE))
                .padding([2, 8])
                .on_press(Message::OpenPressed),
            Button::new(text("Save").size(13).font(Font::MONOSPACE))
                .padding([2, 8])
                .on_press(Message::SavePressed)
        ]
        .spacing(8)
        .align_items(Alignment::Center);

        let row = row![Row::with_children(items).spacing(24), file_controls]
            .spacing(24)
            .padding([6, 16])
            .align_items(Alignment::Center);

        Container::new(row)
            .width(Length::Fill)
            .style(theme::Container::Custom(Box::new(MenuBarStyle)))
            .into()
    }

    fn tab_bar(&self) -> Element<Message> {
        let is_modified = self.content.text() != self.last_saved_text;
        let tab = row![
            text("untitled.txt")
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
            .font_size(14)
            .padding([12, 16]);

        Container::new(editor)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(theme::Container::Custom(Box::new(EditorStyle)))
            .into()
    }

    fn status_bar(&self) -> Element<Message> {
        let is_modified = self.content.text() != self.last_saved_text;
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
            TextInput::new("fichier…", &self.filename)
                .on_input(Message::FilenameChanged)
                .padding([2, 8])
                .size(12),
        ]
        .spacing(12)
        .align_items(Alignment::Center);

        let matches = count_matches(&self.content.text(), &self.search_query);
        let status_text = self
            .status_message
            .clone()
            .unwrap_or_else(|| "Prêt.".to_string());

        let right = text(format!(
            "Occurrences: {}   UTF-8   LF   {}",
            matches, status_text
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

fn count_matches(haystack: &str, needle: &str) -> usize {
    if needle.is_empty() {
        return 0;
    }
    haystack.match_indices(needle).count()
}
