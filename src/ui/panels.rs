//! The panels that open above the editor: go to line, diagnostics,
//! completions, Save As, and the question about a file another program
//! changed.

use super::search_panel::panel_button;
use super::style;
use super::{Message, RoxanneApp};
use crate::diagnostics::DiagnosticSeverity;
use crate::i18n::{t, t_fmt};
use colony_ui::{Palette, Typography};
use iced::widget::{button, column, container, row, scrollable, text_input};
use iced::{Alignment, Element, Length};

impl RoxanneApp {
    pub(super) fn open_goto_panel(&mut self) {
        let (line, _) = self.cursor_position();
        self.goto_line_input = (line + 1).to_string();
        self.goto_panel_open = true;
    }

    pub(super) fn handle_goto_line(&mut self) {
        let input = self.goto_line_input.trim();
        if input.is_empty() {
            self.status_message = Some(t("status_goto_empty").to_string());
            return;
        }
        match parse_goto_input(input) {
            Ok((line, column)) => {
                self.jump_to_position(line, column);
                self.status_message = Some(t_fmt(
                    "status_goto_done",
                    &[("line", &(line + 1)), ("column", &(column + 1))],
                ));
                self.goto_panel_open = false;
            }
            Err(message) => {
                self.status_message = Some(t_fmt("status_goto_error", &[("error", &message)]));
            }
        }
    }

    pub(super) fn goto_panel(&self, typo: &Typography) -> Option<Element<'_, Message>> {
        if !self.goto_panel_open {
            return None;
        }
        let header = row![
            style::label(typo, t("goto_title"), 12, Palette::TEXT_PRIMARY()),
            style::label(typo, t("goto_hint"), 12, Palette::TEXT_MUTED()),
        ]
        .spacing(12)
        .align_y(Alignment::Center);
        let input = text_input(t("goto_placeholder"), &self.goto_line_input)
            .on_input(Message::GotoLineChanged)
            .on_submit(Message::GotoLineSubmit)
            .padding([4, 8])
            .size(typo.sz(12))
            .font(typo.regular)
            .style(style::input);
        let actions = row![
            panel_button(typo, t("goto_go"), Message::GotoLineSubmit),
            panel_button(typo, t("panel_close"), Message::GotoLineClosed),
        ]
        .spacing(8);
        Some(panel(column![header, input, actions].spacing(10)))
    }

    pub(super) fn diagnostics_panel(&self, typo: &Typography) -> Option<Element<'_, Message>> {
        if !self.diagnostics_panel_open {
            return None;
        }
        let diagnostics = &self.tab().diagnostics;
        let header = row![
            style::label(typo, t("diagnostics_title"), 12, Palette::TEXT_PRIMARY()),
            style::label(
                typo,
                t_fmt("diagnostics_count", &[("count", &diagnostics.len())]),
                12,
                Palette::TEXT_MUTED(),
            ),
        ]
        .spacing(12)
        .align_y(Alignment::Center);

        let content: Element<'_, Message> = if diagnostics.is_empty() {
            style::label(typo, t("diagnostics_none"), 12, Palette::TEXT_DIMMER()).into()
        } else {
            let entries = diagnostics.iter().map(|diagnostic| {
                let color = match diagnostic.severity {
                    DiagnosticSeverity::Error => Palette::ERROR(),
                    DiagnosticSeverity::Warning => Palette::WARNING(),
                };
                container(
                    column![
                        style::label(
                            typo,
                            format!("{}:{}", diagnostic.line + 1, diagnostic.column + 1),
                            12,
                            color,
                        ),
                        style::label(typo, &diagnostic.message, 12, Palette::TEXT_SECONDARY()),
                    ]
                    .spacing(2),
                )
                .padding([4, 12])
                .into()
            });
            scrollable(column(entries).spacing(4))
                .height(Length::Fixed(160.0 * typo.scale))
                .into()
        };
        Some(panel(column![header, content].spacing(10)))
    }

    pub(super) fn completion_panel(&self, typo: &Typography) -> Option<Element<'_, Message>> {
        if !self.completion_panel_open {
            return None;
        }
        let tab = self.tab();
        let header = row![
            style::label(typo, t("completion_title"), 12, Palette::TEXT_PRIMARY()),
            style::label(
                typo,
                t_fmt("completion_prefix", &[("prefix", &tab.completion_prefix)]),
                12,
                Palette::TEXT_MUTED(),
            ),
        ]
        .spacing(12)
        .align_y(Alignment::Center);

        let list: Element<'_, Message> = if tab.completion_items.is_empty() {
            style::label(typo, t("completion_none"), 12, Palette::TEXT_DIMMER()).into()
        } else {
            let entries = tab
                .completion_items
                .iter()
                .enumerate()
                .map(|(index, item)| {
                    button(
                        column![
                            style::label(typo, &item.label, 12, Palette::TEXT_PRIMARY())
                                .font(style::code_font()),
                            style::label(typo, &item.detail, 12, Palette::TEXT_MUTED()),
                        ]
                        .spacing(2),
                    )
                    .width(Length::Fill)
                    .padding([4, 12])
                    .style(style::flat)
                    .on_press(Message::CompletionSelected(index))
                    .into()
                });
            scrollable(column(entries).spacing(4))
                .height(Length::Fixed(160.0 * typo.scale))
                .into()
        };
        Some(panel(column![header, list].spacing(10)))
    }

    pub(super) fn save_as_panel(&self, typo: &Typography) -> Option<Element<'_, Message>> {
        if !self.save_as_open {
            return None;
        }
        let input = text_input(t("save_as_placeholder"), &self.save_as_input)
            .on_input(Message::SaveAsChanged)
            .on_submit(Message::SaveAsSubmit)
            .padding([4, 8])
            .size(typo.sz(12))
            .font(typo.regular)
            .style(style::input);
        let confirming = self.save_as_overwrite.as_deref() == Some(self.save_as_input.trim());
        let save_label = if confirming {
            t("save_as_replace")
        } else {
            t("save_as_save")
        };
        let actions = row![
            panel_button(typo, save_label, Message::SaveAsSubmit),
            panel_button(typo, t("panel_cancel"), Message::SaveAsClosed),
        ]
        .spacing(8);
        Some(panel(
            column![
                style::label(typo, t("save_as_title"), 12, Palette::TEXT_PRIMARY()),
                input,
                actions,
            ]
            .spacing(10),
        ))
    }

    /// Asks whether to reload a file another program changed.
    pub(super) fn external_change_panel(&self, typo: &Typography) -> Option<Element<'_, Message>> {
        let path = self.file_changed_externally.as_deref()?;
        Some(panel(
            row![
                style::label(
                    typo,
                    t_fmt("external_change_question", &[("path", &path)]),
                    12,
                    Palette::TEXT_PRIMARY(),
                )
                .width(Length::Fill),
                panel_button(
                    typo,
                    t("external_change_reload"),
                    Message::FileChangedReload
                ),
                panel_button(
                    typo,
                    t("external_change_ignore"),
                    Message::FileChangedIgnore
                ),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        ))
    }
}

/// A panel's frame: full width, on the card surface.
fn panel<'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(content)
        .width(Length::Fill)
        .padding([8, 16])
        .style(style::surface(Palette::BG_CARD()))
        .into()
}

/// `line` or `line:column`, both counted from 1, into 0-based numbers.
pub(super) fn parse_goto_input(input: &str) -> Result<(usize, usize), String> {
    let mut parts = input.split([':', ',']);
    let line_part = parts.next().unwrap_or("").trim();
    let column_part = parts.next().map(str::trim);

    if parts.next().is_some() {
        return Err(t("goto_error_format").to_string());
    }

    let line = line_part
        .parse::<usize>()
        .map_err(|_| t("goto_error_line").to_string())?;
    if line == 0 {
        return Err(t("goto_error_line_zero").to_string());
    }

    let column = match column_part {
        Some(value) if !value.is_empty() => value
            .parse::<usize>()
            .map_err(|_| t("goto_error_column").to_string())?,
        _ => 1,
    };
    if column == 0 {
        return Err(t("goto_error_column_zero").to_string());
    }

    Ok((line - 1, column - 1))
}
