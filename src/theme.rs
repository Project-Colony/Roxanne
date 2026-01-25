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
    pub syntax: SyntaxPalette,
}

impl Default for ThemePalette {
    fn default() -> Self {
        Self::dark()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SyntaxPalette {
    pub keyword: Color,
    pub r#type: Color,
    pub string: Color,
    pub comment: Color,
    pub number: Color,
    pub search_match: Color,
}

impl Default for SyntaxPalette {
    fn default() -> Self {
        Self {
            keyword: Color::from_rgb8(86, 156, 214),
            r#type: Color::from_rgb8(78, 201, 176),
            string: Color::from_rgb8(206, 145, 120),
            comment: Color::from_rgb8(106, 153, 85),
            number: Color::from_rgb8(181, 206, 168),
            search_match: Color::from_rgb8(255, 213, 79),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ThemeConfig {
    pub name: Option<String>,
    #[serde(default)]
    pub palette: PaletteConfig,
    #[serde(default)]
    pub syntax: SyntaxConfig,
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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SyntaxConfig {
    pub keyword: Option<String>,
    pub r#type: Option<String>,
    pub string: Option<String>,
    pub comment: Option<String>,
    pub number: Option<String>,
    pub search_match: Option<String>,
}

impl ThemeConfig {
    pub fn apply_to(&self, palette: &mut ThemePalette) -> Vec<String> {
        let mut warnings = Vec::new();
        if let Some(name) = &self.name {
            match ThemePalette::from_name(name) {
                Some(theme) => *palette = theme,
                None => warnings.push(format!("Thème: nom inconnu '{name}'")),
            }
        }

        apply_color(
            &mut warnings,
            &self.palette.app_background,
            &mut palette.app_background,
            "app_background",
        );
        apply_color(
            &mut warnings,
            &self.palette.menu_bar,
            &mut palette.menu_bar,
            "menu_bar",
        );
        apply_color(
            &mut warnings,
            &self.palette.menu_button_active,
            &mut palette.menu_button_active,
            "menu_button_active",
        );
        apply_color(
            &mut warnings,
            &self.palette.menu_button_hover,
            &mut palette.menu_button_hover,
            "menu_button_hover",
        );
        apply_color(
            &mut warnings,
            &self.palette.submenu_bar,
            &mut palette.submenu_bar,
            "submenu_bar",
        );
        apply_color(
            &mut warnings,
            &self.palette.button_base,
            &mut palette.button_base,
            "button_base",
        );
        apply_color(
            &mut warnings,
            &self.palette.button_hover,
            &mut palette.button_hover,
            "button_hover",
        );
        apply_color(
            &mut warnings,
            &self.palette.toggle_active,
            &mut palette.toggle_active,
            "toggle_active",
        );
        apply_color(
            &mut warnings,
            &self.palette.toggle_inactive,
            &mut palette.toggle_inactive,
            "toggle_inactive",
        );
        apply_color(
            &mut warnings,
            &self.palette.tab_bar,
            &mut palette.tab_bar,
            "tab_bar",
        );
        apply_color(
            &mut warnings,
            &self.palette.tab_active,
            &mut palette.tab_active,
            "tab_active",
        );
        apply_color(
            &mut warnings,
            &self.palette.editor_background,
            &mut palette.editor_background,
            "editor_background",
        );
        apply_color(
            &mut warnings,
            &self.palette.status_bar,
            &mut palette.status_bar,
            "status_bar",
        );
        apply_color(
            &mut warnings,
            &self.palette.panel_background,
            &mut palette.panel_background,
            "panel_background",
        );
        apply_color(
            &mut warnings,
            &self.palette.panel_item_background,
            &mut palette.panel_item_background,
            "panel_item_background",
        );
        apply_color(
            &mut warnings,
            &self.palette.panel_item_hover,
            &mut palette.panel_item_hover,
            "panel_item_hover",
        );
        apply_color(
            &mut warnings,
            &self.syntax.keyword,
            &mut palette.syntax.keyword,
            "syntax.keyword",
        );
        apply_color(
            &mut warnings,
            &self.syntax.r#type,
            &mut palette.syntax.r#type,
            "syntax.type",
        );
        apply_color(
            &mut warnings,
            &self.syntax.string,
            &mut palette.syntax.string,
            "syntax.string",
        );
        apply_color(
            &mut warnings,
            &self.syntax.comment,
            &mut palette.syntax.comment,
            "syntax.comment",
        );
        apply_color(
            &mut warnings,
            &self.syntax.number,
            &mut palette.syntax.number,
            "syntax.number",
        );
        apply_color(
            &mut warnings,
            &self.syntax.search_match,
            &mut palette.syntax.search_match,
            "syntax.search_match",
        );

        warnings
    }
}

fn apply_color(
    warnings: &mut Vec<String>,
    value: &Option<String>,
    target: &mut Color,
    label: &str,
) {
    if let Some(value) = value {
        match parse_color(value) {
            Ok(color) => *target = color,
            Err(err) => warnings.push(format!("Thème: {label}: {err}")),
        }
    }
}

impl ThemePalette {
    pub fn from_name(name: &str) -> Option<Self> {
        if name.eq_ignore_ascii_case("default") {
            return Some(Self::dark());
        }
        if name.eq_ignore_ascii_case("dark") {
            return Some(Self::dark());
        }
        if name.eq_ignore_ascii_case("light") {
            return Some(Self::light());
        }
        None
    }

    pub fn to_config(&self) -> ThemeConfig {
        ThemeConfig {
            name: None,
            palette: PaletteConfig {
                app_background: Some(color_to_hex(self.app_background)),
                menu_bar: Some(color_to_hex(self.menu_bar)),
                menu_button_active: Some(color_to_hex(self.menu_button_active)),
                menu_button_hover: Some(color_to_hex(self.menu_button_hover)),
                submenu_bar: Some(color_to_hex(self.submenu_bar)),
                button_base: Some(color_to_hex(self.button_base)),
                button_hover: Some(color_to_hex(self.button_hover)),
                toggle_active: Some(color_to_hex(self.toggle_active)),
                toggle_inactive: Some(color_to_hex(self.toggle_inactive)),
                tab_bar: Some(color_to_hex(self.tab_bar)),
                tab_active: Some(color_to_hex(self.tab_active)),
                editor_background: Some(color_to_hex(self.editor_background)),
                status_bar: Some(color_to_hex(self.status_bar)),
                panel_background: Some(color_to_hex(self.panel_background)),
                panel_item_background: Some(color_to_hex(self.panel_item_background)),
                panel_item_hover: Some(color_to_hex(self.panel_item_hover)),
            },
            syntax: SyntaxConfig {
                keyword: Some(color_to_hex(self.syntax.keyword)),
                r#type: Some(color_to_hex(self.syntax.r#type)),
                string: Some(color_to_hex(self.syntax.string)),
                comment: Some(color_to_hex(self.syntax.comment)),
                number: Some(color_to_hex(self.syntax.number)),
                search_match: Some(color_to_hex(self.syntax.search_match)),
            },
        }
    }

    fn dark() -> Self {
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
            syntax: SyntaxPalette::default(),
        }
    }

    fn light() -> Self {
        Self {
            app_background: Color::from_rgb8(245, 245, 246),
            menu_bar: Color::from_rgb8(225, 225, 228),
            menu_button_active: Color::from_rgb8(205, 205, 210),
            menu_button_hover: Color::from_rgb8(215, 215, 218),
            submenu_bar: Color::from_rgb8(232, 232, 234),
            button_base: Color::from_rgb8(210, 210, 214),
            button_hover: Color::from_rgb8(198, 198, 204),
            toggle_active: Color::from_rgb8(180, 180, 188),
            toggle_inactive: Color::from_rgb8(210, 210, 214),
            tab_bar: Color::from_rgb8(230, 230, 234),
            tab_active: Color::from_rgb8(205, 205, 210),
            editor_background: Color::from_rgb8(255, 255, 255),
            status_bar: Color::from_rgb8(225, 225, 228),
            panel_background: Color::from_rgb8(235, 235, 238),
            panel_item_background: Color::from_rgb8(220, 220, 224),
            panel_item_hover: Color::from_rgb8(200, 200, 206),
            syntax: SyntaxPalette {
                keyword: Color::from_rgb8(0, 92, 179),
                r#type: Color::from_rgb8(0, 128, 113),
                string: Color::from_rgb8(163, 74, 48),
                comment: Color::from_rgb8(87, 126, 69),
                number: Color::from_rgb8(110, 124, 65),
                search_match: Color::from_rgb8(196, 128, 0),
            },
        }
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

fn color_to_hex(color: Color) -> String {
    let red = (color.r.clamp(0.0, 1.0) * 255.0).round() as u8;
    let green = (color.g.clamp(0.0, 1.0) * 255.0).round() as u8;
    let blue = (color.b.clamp(0.0, 1.0) * 255.0).round() as u8;
    format!("#{:02x}{:02x}{:02x}", red, green, blue)
}
