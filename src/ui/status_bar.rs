//! The status bar: the file name, quick search, the cursor and the last
//! message.

use super::search_panel::{SEARCH_INPUT, toggle_button};
use super::style;
use super::{Message, RoxanneApp};
use crate::i18n::{t, t_fmt};
use crate::keymap::KeymapMode;
use colony_ui::{Palette, Typography};
use iced::widget::{Button, TextInput, button, container, row, text_input};
use iced::{Alignment, Element, Length};

/// Font Awesome's chevron-left, which the Nerd Font carries too.
const CHEVRON_LEFT: &str = "\u{f053}";

fn input<'a>(
    typo: &Typography,
    placeholder: &str,
    value: &str,
    on_input: fn(String) -> Message,
) -> TextInput<'a, Message> {
    text_input(placeholder, value)
        .on_input(on_input)
        .padding([2, 8])
        .size(typo.sz(12))
        .font(typo.regular)
        .style(style::input)
}

/// Previous or next match: disabled while there is none.
fn arrow<'a>(typo: &Typography, glyph: &'a str, message: Option<Message>) -> Button<'a, Message> {
    button(style::button_label(typo, glyph, 10).font(colony_ui::fonts::glyph_font()))
        .padding([2, 6])
        .style(style::flat)
        .on_press_maybe(message)
}

impl RoxanneApp {
    pub(super) fn status_bar(&self, typo: &Typography) -> Element<'_, Message> {
        let tab = self.tab();
        let has_matches = !tab.search_matches.is_empty();

        let left = row![
            style::label(
                typo,
                if tab.is_modified() {
                    format!("{} \u{25cf}", tab.display_name())
                } else {
                    tab.display_name().to_string()
                },
                12,
                Palette::TEXT_SECONDARY(),
            ),
            input(
                typo,
                t("status_search_placeholder"),
                &self.search_query,
                Message::SearchChanged
            )
            .id(SEARCH_INPUT)
            .on_submit(Message::SearchNext),
            toggle_button(
                typo,
                "Aa",
                self.search_case_sensitive,
                Message::SearchToggleCaseSensitive
            ),
            toggle_button(typo, ".*", self.search_regex, Message::SearchToggleRegex),
            arrow(
                typo,
                CHEVRON_LEFT,
                has_matches.then_some(Message::SearchPrevious)
            ),
            arrow(
                typo,
                colony_ui::widgets::icons::CHEVRON_RIGHT,
                has_matches.then_some(Message::SearchNext)
            ),
            input(
                typo,
                t("status_file_placeholder"),
                &tab.filename,
                Message::FilenameChanged
            ),
        ]
        .spacing(12)
        .align_y(Alignment::Center);

        let right = style::label(
            typo,
            self.status_segments().join("   "),
            12,
            Palette::TEXT_MUTED(),
        )
        .width(Length::Fill)
        .align_x(iced::alignment::Horizontal::Right);

        container(
            row![left, right]
                .spacing(16)
                .padding([6, 16])
                .width(Length::Fill)
                .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .style(style::surface(Palette::BG_SIDEBAR()))
        .into()
    }

    /// The right-hand side of the status bar: only what is worth showing now.
    fn status_segments(&self) -> Vec<String> {
        let tab = self.tab();
        let mut segments = Vec::new();
        if self.mode != KeymapMode::Insert {
            segments.push(t_fmt("status_mode", &[("mode", &self.mode.label())]));
        }
        let matches = tab.search_matches.len();
        if !self.search_query.is_empty() && matches > 0 {
            segments.push(t_fmt("status_matches", &[("count", &matches)]));
        }
        let cursors = tab.multi_cursors.len() + 1;
        if cursors > 1 {
            segments.push(t_fmt("status_cursors", &[("count", &cursors)]));
        }
        if !tab.diagnostics.is_empty() {
            segments.push(t_fmt(
                "status_diagnostics",
                &[("count", &tab.diagnostics.len())],
            ));
        }
        let (line, column) = self.cursor_position();
        // The editor counts bytes, people count characters.
        let column = tab
            .buffer
            .line(line)
            .and_then(|text| text.get(..column))
            .map_or(column, |before| before.chars().count());
        segments.push(t_fmt(
            "status_position",
            &[
                ("line", &(line + 1)),
                ("lines", &tab.buffer.line_count()),
                ("column", &(column + 1)),
            ],
        ));
        if self.config.editor.word_wrap {
            segments.push(t("status_wrap").to_string());
        }
        if self.config.editor.minimap {
            segments.push(t("status_minimap").to_string());
        }
        let plugins = self
            .plugins
            .statuses()
            .into_iter()
            .map(|status| format!("{}: {}", status.label, status.value))
            .collect::<Vec<_>>()
            .join("   ");
        if !plugins.is_empty() {
            segments.push(plugins);
        }
        if let Some(message) = &self.status_message {
            segments.push(message.clone());
        }
        segments
    }
}
