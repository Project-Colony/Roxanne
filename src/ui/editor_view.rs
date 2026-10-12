//! The text area: line numbers, the editor itself and the minimap.

use super::style;
use super::{Message, RoxanneApp};
use crate::editor::highlight;
use crate::i18n::{t, t_fmt};
use colony_ui::{Palette, Typography};
use iced::widget::text::{LineHeight, Wrapping};
use iced::widget::{column, container, row, sensor, text_editor};
use iced::{Element, Length, Size};

/// The base size of code, before the user's text scale.
const CODE_SIZE: u16 = 14;
/// Space above and below the text, in the editor and the gutter alike.
const VERTICAL_PADDING: f32 = 12.0;

/// The size of code and the height of one of its lines, both following the
/// user's text scale, so the gutter's numbers stay level with their lines.
fn code_metrics(typo: &Typography) -> (f32, f32) {
    let size = typo.sz(CODE_SIZE);
    (size, (size * 1.3).round())
}

impl RoxanneApp {
    /// Works out how many lines fit in the editor once it reports its size.
    pub(super) fn update_viewport_height(&mut self, size: Size) {
        let (_, line_height) = code_metrics(&Typography::current());
        let available = (size.height - 2.0 * VERTICAL_PADDING).max(0.0);
        let visible_lines = (available / line_height).ceil().max(1.0) as usize;
        if visible_lines != self.viewport_height {
            self.viewport_height = visible_lines;
            self.refresh_viewport_cache();
        }
    }

    pub(super) fn editor_area(&self, typo: &Typography) -> Element<'_, Message> {
        let (code_size, line_height) = code_metrics(typo);
        let editor_config = &self.config.editor;
        let editor = text_editor(&self.tab().content)
            .on_action(Message::Edit)
            .placeholder(t("editor_placeholder"))
            .font(style::code_font())
            .size(code_size)
            .line_height(LineHeight::Absolute(line_height.into()))
            .padding([VERTICAL_PADDING, 16.0])
            .height(Length::Fill)
            .wrapping(if editor_config.word_wrap {
                Wrapping::Word
            } else {
                Wrapping::None
            })
            .style(style::editor)
            .highlight_with::<highlight::RoxanneHighlighter>(
                self.tab().highlight_settings.clone(),
                highlight::highlight_format,
            );
        let editor = sensor(
            container(editor)
                .width(Length::Fill)
                .height(Length::Fill)
                .style(style::surface(Palette::BG_PRIMARY())),
        )
        .on_show(Message::EditorResized)
        .on_resize(Message::EditorResized);

        // Wrapped lines take more than one row, so numbers would drift away
        // from their lines: the gutter shows only while lines do not wrap.
        if !editor_config.line_numbers || editor_config.word_wrap {
            return editor.into();
        }
        row![self.gutter(code_size, line_height), editor]
            .height(Length::Fill)
            .into()
    }

    /// Line numbers for the lines in view, from the tracked scroll offset.
    fn gutter(&self, code_size: f32, line_height: f32) -> Element<'_, Message> {
        let line_count = self.tab().buffer.line_count().max(1);
        let digits = line_count.to_string().len();
        let width = digits as f32 * (code_size * 0.6) + 24.0;
        let start = (self.tab().scroll_offset.max(0) as usize).min(line_count);
        // Enough rows to fill the visible height, whatever the scroll.
        let rows = self.viewport_height.max(1) * 2 + 20;
        let numbers = (start..start + rows)
            .map(|index| {
                if index < line_count {
                    format!("{:>digits$}", index + 1)
                } else {
                    format!("{:>digits$}", "~")
                }
            })
            .collect::<Vec<_>>()
            .join("\n");

        container(
            iced::widget::text(numbers)
                .size(code_size)
                .font(style::code_font())
                .color(Palette::TEXT_DIMMER())
                .line_height(LineHeight::Absolute(line_height.into()))
                .align_x(iced::alignment::Horizontal::Right),
        )
        .width(Length::Fixed(width))
        .height(Length::Fill)
        .padding([VERTICAL_PADDING, 8.0])
        .clip(true)
        .style(style::surface(Palette::BG_PRIMARY()))
        .into()
    }

    /// A condensed outline of the file, one character per row.
    pub(super) fn minimap_panel(&self, typo: &Typography) -> Option<Element<'_, Message>> {
        if !self.config.editor.minimap {
            return None;
        }
        let tab = self.tab();
        let total_lines = tab.buffer.line_count().max(1);
        let (cursor_line, _) = self.cursor_position();

        let rows: usize = 60.min(total_lines);
        let step = (total_lines as f64 / rows as f64).max(1.0);
        let mut outline = String::new();
        for i in 0..rows {
            let line_index = (i as f64 * step) as usize;
            let trimmed = tab.buffer.line(line_index).unwrap_or("").trim();
            let next_row = ((i + 1) as f64 * step) as usize;
            outline.push(
                if line_index <= cursor_line && cursor_line < next_row.max(line_index + 1) {
                    '\u{2588}'
                } else if trimmed.is_empty() {
                    ' '
                } else if trimmed.len() < 10 {
                    '\u{2591}'
                } else if trimmed.len() < 30 {
                    '\u{2592}'
                } else {
                    '\u{2593}'
                },
            );
            outline.push('\n');
        }

        let content = column![
            style::label(typo, t("minimap_title"), 10, Palette::TEXT_DIMMER()),
            iced::widget::text(outline)
                .size(typo.sz(6))
                .font(style::code_font())
                .color(Palette::TEXT_DIM()),
            style::label(
                typo,
                t_fmt(
                    "minimap_lines",
                    &[("line", &(cursor_line + 1)), ("total", &total_lines)],
                ),
                10,
                Palette::TEXT_DIMMER(),
            ),
        ]
        .spacing(2)
        .padding(4);

        Some(
            container(content)
                .width(Length::Fixed(60.0 * typo.scale))
                .height(Length::Fill)
                .style(style::surface(Palette::BG_SIDEBAR()))
                .into(),
        )
    }
}
