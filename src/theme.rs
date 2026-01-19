use iced::Color;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy)]
pub struct ThemePalette {
    pub app_background: Color,
    pub menu_bar: Color,
    pub menu_button_active: Color,
    pub menu_button_hover: Color,
    pub submenu_bar: Color,
    pub button_base: Color,
    pub button_hover: Color,
    pub toggle_active: Color,
    pub toggle_inactive: Color,
    pub tab_bar: Color,
    pub tab_active: Color,
    pub editor_background: Color,
    pub status_bar: Color,
    pub panel_background: Color,
    pub panel_item_background: Color,
    pub panel_item_hover: Color,
}

impl Default for ThemePalette {
    fn default() -> Self {
        Self {
            app_background: Color::from_rgb8(30, 30, 32),
            menu_bar: Color::from_rgb8(45, 45, 48),
            menu_button_active: Color::from_rgb8(65, 65, 70),
            menu_button_hover: Color::from_rgb8(70, 70, 74),
            submenu_bar: Color::from_rgb8(40, 40, 44),
            button_base: Color::from_rgb8(55, 55, 60),
            button_hover: Color::from_rgb8(65, 65, 70),
            toggle_active: Color::from_rgb8(90, 90, 96),
            toggle_inactive: Color::from_rgb8(55, 55, 60),
            tab_bar: Color::from_rgb8(37, 37, 40),
            tab_active: Color::from_rgb8(50, 50, 54),
            editor_background: Color::from_rgb8(28, 28, 30),
            status_bar: Color::from_rgb8(45, 45, 48),
            panel_background: Color::from_rgb8(36, 36, 40),
            panel_item_background: Color::from_rgb8(42, 42, 46),
            panel_item_hover: Color::from_rgb8(60, 60, 66),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ThemeConfig {
    pub name: Option<String>,
    #[serde(default)]
    pub palette: PaletteConfig,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PaletteConfig {
    pub app_background: Option<String>,
    pub menu_bar: Option<String>,
    pub menu_button_active: Option<String>,
    pub menu_button_hover: Option<String>,
    pub submenu_bar: Option<String>,
    pub button_base: Option<String>,
    pub button_hover: Option<String>,
    pub toggle_active: Option<String>,
    pub toggle_inactive: Option<String>,
    pub tab_bar: Option<String>,
    pub tab_active: Option<String>,
    pub editor_background: Option<String>,
    pub status_bar: Option<String>,
    pub panel_background: Option<String>,
    pub panel_item_background: Option<String>,
    pub panel_item_hover: Option<String>,
}

impl ThemeConfig {
    pub fn apply_to(&self, palette: &mut ThemePalette) -> Vec<String> {
        let mut warnings = Vec::new();
        let mut apply = |value: &Option<String>, target: &mut Color, label: &str| {
            if let Some(value) = value {
                match parse_color(value) {
                    Ok(color) => *target = color,
                    Err(err) => warnings.push(format!("Thème: {label}: {err}")),
                }
            }
        };

        apply(
            &self.palette.app_background,
            &mut palette.app_background,
            "app_background",
        );
        apply(&self.palette.menu_bar, &mut palette.menu_bar, "menu_bar");
        apply(
            &self.palette.menu_button_active,
            &mut palette.menu_button_active,
            "menu_button_active",
        );
        apply(
            &self.palette.menu_button_hover,
            &mut palette.menu_button_hover,
            "menu_button_hover",
        );
        apply(
            &self.palette.submenu_bar,
            &mut palette.submenu_bar,
            "submenu_bar",
        );
        apply(
            &self.palette.button_base,
            &mut palette.button_base,
            "button_base",
        );
        apply(
            &self.palette.button_hover,
            &mut palette.button_hover,
            "button_hover",
        );
        apply(
            &self.palette.toggle_active,
            &mut palette.toggle_active,
            "toggle_active",
        );
        apply(
            &self.palette.toggle_inactive,
            &mut palette.toggle_inactive,
            "toggle_inactive",
        );
        apply(&self.palette.tab_bar, &mut palette.tab_bar, "tab_bar");
        apply(&self.palette.tab_active, &mut palette.tab_active, "tab_active");
        apply(
            &self.palette.editor_background,
            &mut palette.editor_background,
            "editor_background",
        );
        apply(
            &self.palette.status_bar,
            &mut palette.status_bar,
            "status_bar",
        );
        apply(
            &self.palette.panel_background,
            &mut palette.panel_background,
            "panel_background",
        );
        apply(
            &self.palette.panel_item_background,
            &mut palette.panel_item_background,
            "panel_item_background",
        );
        apply(
            &self.palette.panel_item_hover,
            &mut palette.panel_item_hover,
            "panel_item_hover",
        );

        warnings
    }
}

fn parse_color(value: &str) -> Result<Color, String> {
    let trimmed = value.trim().trim_start_matches('#');
    if trimmed.len() != 6 {
        return Err(format!("couleur invalide '{value}'"));
    }

    let red = u8::from_str_radix(&trimmed[0..2], 16)
        .map_err(|_| format!("couleur invalide '{value}'"))?;
    let green = u8::from_str_radix(&trimmed[2..4], 16)
        .map_err(|_| format!("couleur invalide '{value}'"))?;
    let blue = u8::from_str_radix(&trimmed[4..6], 16)
        .map_err(|_| format!("couleur invalide '{value}'"))?;

    Ok(Color::from_rgb8(red, green, blue))
}
