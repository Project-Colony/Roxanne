use iced::alignment::{Horizontal, Vertical};
use iced::theme;
use iced::widget::{
    Column, Container, Row, Scrollable, Text, column, container, row, scrollable, text,
};
use iced::{
    Alignment, Application, Background, Color, Command, Element, Font, Length, Settings, Theme,
    executor,
};

#[derive(Debug, Clone)]
pub enum Message {}

#[derive(Debug)]
pub struct RoxanneApp {
    filename: String,
    is_modified: bool,
    cursor_line: usize,
    cursor_col: usize,
    buffer: Vec<String>,
}

impl RoxanneApp {
    pub fn run() -> iced::Result {
        RoxanneApp::run_with(Settings::default())
    }

    pub fn run_with(settings: Settings) -> iced::Result {
        <Self as Application>::run(settings)
    }
}

impl Application for RoxanneApp {
    type Executor = executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {
        (
            Self {
                filename: "untitled.txt".to_string(),
                is_modified: false,
                cursor_line: 1,
                cursor_col: 1,
                buffer: vec![
                    "Roxanne – éditeur en mode Iced".to_string(),
                    "".to_string(),
                    "Objectif: MVP inspiré de Sublime Text".to_string(),
                    "- Menu bar, tabs, status bar".to_string(),
                    "- Zone d'édition monospace".to_string(),
                ],
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        "Roxanne".to_string()
    }

    fn update(&mut self, _message: Message) -> Command<Message> {
        Command::none()
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
            })
            .collect::<Vec<_>>();

        let row = Row::with_children(items)
            .spacing(24)
            .padding([6, 16])
            .align_items(Alignment::Center);

        Container::new(row)
            .width(Length::Fill)
            .style(theme::Container::Custom(Box::new(MenuBarStyle)))
            .into()
    }

    fn tab_bar(&self) -> Element<Message> {
        let tab = row![
            text("untitled.txt")
                .size(13)
                .font(Font::MONOSPACE)
                .style(Color::from_rgb8(230, 230, 230)),
            text("×")
                .size(13)
                .font(Font::MONOSPACE)
                .style(Color::from_rgb8(180, 180, 180))
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
            .height(Length::Units(32))
            .style(theme::Container::Custom(Box::new(TabBarStyle)))
            .into()
    }

    fn editor_area(&self) -> Element<Message> {
        let line_items = self.buffer.iter().enumerate().map(|(index, line)| {
            let number = format!("{:>4}", index + 1);
            row![
                text(number)
                    .size(13)
                    .font(Font::MONOSPACE)
                    .style(Color::from_rgb8(120, 120, 120)),
                text(line)
                    .size(13)
                    .font(Font::MONOSPACE)
                    .style(Color::from_rgb8(210, 210, 210))
            ]
            .spacing(16)
            .align_items(Alignment::Start)
            .into()
        });

        let content = Column::with_children(line_items)
            .spacing(4)
            .padding([12, 16]);

        let scroll = Scrollable::new(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .direction(scrollable::Direction::Vertical(
                scrollable::Properties::new(),
            ));

        Container::new(scroll)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(theme::Container::Custom(Box::new(EditorStyle)))
            .into()
    }

    fn status_bar(&self) -> Element<Message> {
        let left = text(format!(
            "{}{}",
            self.filename,
            if self.is_modified { " ●" } else { "" }
        ))
        .size(12)
        .font(Font::MONOSPACE)
        .style(Color::from_rgb8(200, 200, 200));

        let right = text(format!(
            "Ln {}, Col {}   UTF-8   LF",
            self.cursor_line, self.cursor_col
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
