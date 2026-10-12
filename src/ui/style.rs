//! Widget styles, all drawn from the active Colony palette.
//!
//! Nothing here holds a colour of its own: every one comes from
//! [`colony_ui::Palette`], so a theme, accent or high-contrast change reaches
//! the whole window on the next frame.

use colony_ui::{Palette, Typography};
use iced::widget::text::IntoFragment;
use iced::widget::{Text, button, container, text, text_editor, text_input, tooltip};
use iced::{Background, Border, Color, Element, Font, Theme};

/// The iced theme for the widgets Roxanne does not style itself (scroll bars),
/// built from the active palette.
pub fn iced_theme() -> Theme {
    let palette = colony_ui::active_palette();
    Theme::custom(
        "Colony",
        iced::theme::Palette {
            background: palette.bg_primary,
            text: palette.text_primary,
            primary: colony_ui::effective_accent(),
            success: palette.success,
            warning: palette.warning,
            danger: palette.error,
        },
    )
}

/// The font code is shown in: always the monospace Nerd Font, also with the
/// dyslexia font on, because OpenDyslexic is proportional and would break the
/// columns and the line-number gutter.
pub fn code_font() -> Font {
    colony_ui::fonts::glyph_font()
}

/// A label in the interface font at `sz(size)`.
pub fn label<'a>(
    typo: &Typography,
    content: impl IntoFragment<'a>,
    size: u16,
    color: Color,
) -> Text<'a> {
    text(content)
        .size(typo.sz(size))
        .font(typo.regular)
        .color(color)
}

/// A label that takes its colour from the button it sits in.
pub fn button_label<'a>(typo: &Typography, content: impl IntoFragment<'a>, size: u16) -> Text<'a> {
    text(content).size(typo.sz(size)).font(typo.regular)
}

/// `content` with `tip` shown below it on hover: the words for a control
/// drawn as a symbol.
pub fn tip<'a, M: 'a>(
    typo: &Typography,
    content: impl Into<Element<'a, M>>,
    tip: &'a str,
) -> Element<'a, M> {
    tooltip(
        content,
        container(label(typo, tip, 11, Palette::TEXT_PRIMARY()))
            .padding([2, 6])
            .style(|_| container::Style {
                background: Some(Palette::BG_CARD().into()),
                border: Border::default()
                    .rounded(4)
                    .width(1)
                    .color(Palette::BORDER_SUBTLE()),
                ..container::Style::default()
            }),
        tooltip::Position::Bottom,
    )
    .into()
}

/// A surface filled with `background`.
pub fn surface(background: Color) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style {
        background: Some(background.into()),
        text_color: Some(Palette::TEXT_PRIMARY()),
        ..container::Style::default()
    }
}

/// A one-pixel line between menu entries.
pub fn divider(_: &Theme) -> container::Style {
    container::Style {
        background: Some(Palette::DIVIDER().into()),
        ..container::Style::default()
    }
}

/// A button with no fill until hovered: menu entries, list rows, small
/// actions.
pub fn flat(_: &Theme, status: button::Status) -> button::Style {
    let (background, text_color) = match status {
        button::Status::Hovered => (Palette::BG_CARD_HOVER(), Palette::TEXT_PRIMARY()),
        button::Status::Pressed => (Palette::BG_CARD_PRESSED(), Palette::TEXT_PRIMARY()),
        button::Status::Active => (Color::TRANSPARENT, Palette::TEXT_SECONDARY()),
        button::Status::Disabled => (Color::TRANSPARENT, Palette::TEXT_DIMMEST()),
    };
    button::Style {
        background: Some(background.into()),
        text_color,
        border: Border::default().rounded(4),
        ..button::Style::default()
    }
}

/// A button drawn as a filled card: the actions of a panel.
pub fn card(_: &Theme, status: button::Status) -> button::Style {
    let (background, text_color) = match status {
        button::Status::Hovered => (Palette::BTN_HOVER(), Palette::TEXT_PRIMARY()),
        button::Status::Pressed => (Palette::BTN_PRESSED(), Palette::TEXT_PRIMARY()),
        button::Status::Active => (Palette::BTN_DEFAULT(), Palette::TEXT_PRIMARY()),
        button::Status::Disabled => (Palette::BTN_DEFAULT(), Palette::TEXT_DIMMEST()),
    };
    button::Style {
        background: Some(background.into()),
        text_color,
        border: Border::default().rounded(4),
        ..button::Style::default()
    }
}

pub fn input(_: &Theme, status: text_input::Status) -> text_input::Style {
    let border = match status {
        text_input::Status::Focused { .. } => Palette::ACCENT(),
        text_input::Status::Hovered => Palette::TEXT_DIMMER(),
        _ => Palette::BORDER_SUBTLE(),
    };
    text_input::Style {
        background: Background::Color(Palette::BG_INPUT()),
        border: Border::default().rounded(4).width(1).color(border),
        icon: Palette::TEXT_MUTED(),
        placeholder: Palette::TEXT_PLACEHOLDER(),
        value: Palette::TEXT_PRIMARY(),
        selection: selection(),
    }
}

pub fn editor(_: &Theme, _: text_editor::Status) -> text_editor::Style {
    text_editor::Style {
        background: Background::Color(Palette::BG_PRIMARY()),
        border: Border::default(),
        placeholder: Palette::TEXT_PLACEHOLDER(),
        value: Palette::TEXT_PRIMARY(),
        selection: selection(),
    }
}

/// Selected text: the accent, see-through enough to read the text over it.
fn selection() -> Color {
    Color {
        a: 0.35,
        ..Palette::ACCENT()
    }
}
