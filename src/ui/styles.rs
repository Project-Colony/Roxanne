use crate::theme::ThemePalette;
use iced::widget::{button, container};
use iced::{Background, Color, Theme};

/// A single container style that picks its background from a palette color.
pub struct PaletteContainer {
    pub color: Color,
}

impl container::StyleSheet for PaletteContainer {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(self.color)),
            text_color: None,
            border: Default::default(),
            shadow: Default::default(),
        }
    }
}

/// A simple button style with no background, colored text, and a hover color.
pub struct PaletteButton {
    pub hover: Color,
}

impl button::StyleSheet for PaletteButton {
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
        appearance.background = Some(Background::Color(self.hover));
        appearance
    }
}

/// A menu button that can be active (showing a background) or not.
pub struct MenuButtonStyle {
    pub active: bool,
    pub active_color: Color,
    pub hover_color: Color,
}

impl button::StyleSheet for MenuButtonStyle {
    type Style = Theme;

    fn active(&self, _style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: self.active.then_some(Background::Color(self.active_color)),
            text_color: Color::from_rgb8(220, 220, 220),
            border: Default::default(),
            shadow_offset: Default::default(),
            shadow: Default::default(),
        }
    }

    fn hovered(&self, style: &Self::Style) -> button::Appearance {
        let mut appearance = self.active(style);
        appearance.background = Some(Background::Color(self.hover_color));
        appearance
    }
}

/// A toggle button that shows different backgrounds for active/inactive.
pub struct ToggleButtonStyle {
    pub active: bool,
    pub active_color: Color,
    pub inactive_color: Color,
    pub hover_color: Color,
}

impl button::StyleSheet for ToggleButtonStyle {
    type Style = Theme;

    fn active(&self, _style: &Self::Style) -> button::Appearance {
        let background = if self.active {
            self.active_color
        } else {
            self.inactive_color
        };
        button::Appearance {
            background: Some(Background::Color(background)),
            text_color: Color::from_rgb8(230, 230, 230),
            border: Default::default(),
            shadow_offset: Default::default(),
            shadow: Default::default(),
        }
    }

    fn hovered(&self, style: &Self::Style) -> button::Appearance {
        let mut appearance = self.active(style);
        appearance.background = Some(Background::Color(self.hover_color));
        appearance
    }
}

/// A search result button with no background, white text, hover from palette.
pub struct SearchResultButtonStyle {
    pub hover_color: Color,
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
        appearance.background = Some(Background::Color(self.hover_color));
        appearance
    }
}

// Helper constructors for common palette-driven styles

pub fn app_background(palette: &ThemePalette) -> Box<PaletteContainer> {
    Box::new(PaletteContainer {
        color: palette.app_background,
    })
}

pub fn menu_bar(palette: &ThemePalette) -> Box<PaletteContainer> {
    Box::new(PaletteContainer {
        color: palette.menu_bar,
    })
}

pub fn submenu(palette: &ThemePalette) -> Box<PaletteContainer> {
    Box::new(PaletteContainer {
        color: palette.submenu_bar,
    })
}

pub fn submenu_separator(palette: &ThemePalette) -> Box<PaletteContainer> {
    Box::new(PaletteContainer {
        color: palette.menu_button_hover,
    })
}

pub fn tab_bar(palette: &ThemePalette) -> Box<PaletteContainer> {
    Box::new(PaletteContainer {
        color: palette.tab_bar,
    })
}

pub fn active_tab(palette: &ThemePalette) -> Box<PaletteContainer> {
    Box::new(PaletteContainer {
        color: palette.tab_active,
    })
}

pub fn editor(palette: &ThemePalette) -> Box<PaletteContainer> {
    Box::new(PaletteContainer {
        color: palette.editor_background,
    })
}

pub fn gutter(palette: &ThemePalette) -> Box<PaletteContainer> {
    Box::new(PaletteContainer {
        color: palette.panel_background,
    })
}

pub fn status_bar(palette: &ThemePalette) -> Box<PaletteContainer> {
    Box::new(PaletteContainer {
        color: palette.status_bar,
    })
}

pub fn panel(palette: &ThemePalette) -> Box<PaletteContainer> {
    Box::new(PaletteContainer {
        color: palette.panel_background,
    })
}

pub fn panel_item(palette: &ThemePalette) -> Box<PaletteContainer> {
    Box::new(PaletteContainer {
        color: palette.panel_item_background,
    })
}

pub fn submenu_button(palette: &ThemePalette) -> Box<PaletteButton> {
    Box::new(PaletteButton {
        hover: palette.button_hover,
    })
}

pub fn menu_button(palette: &ThemePalette, is_active: bool) -> Box<MenuButtonStyle> {
    Box::new(MenuButtonStyle {
        active: is_active,
        active_color: palette.menu_button_active,
        hover_color: palette.menu_button_hover,
    })
}

pub fn toggle_button(palette: &ThemePalette, is_active: bool) -> Box<ToggleButtonStyle> {
    Box::new(ToggleButtonStyle {
        active: is_active,
        active_color: palette.toggle_active,
        inactive_color: palette.toggle_inactive,
        hover_color: palette.button_hover,
    })
}

pub fn search_result_button(palette: &ThemePalette) -> Box<SearchResultButtonStyle> {
    Box::new(SearchResultButtonStyle {
        hover_color: palette.panel_item_hover,
    })
}
