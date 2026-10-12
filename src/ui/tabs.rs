//! Open files: one [`Tab`] each, and the tab bar above the editor.

use super::style;
use super::{DEFAULT_VIEWPORT_HEIGHT, Message, RoxanneApp, UNTITLED};
use crate::completion::CompletionItem;
use crate::diagnostics::{self, Diagnostic};
use crate::editor::highlight::{self, MatchPosition};
use crate::editor::{Position, TextBuffer, ViewportCache};
use crate::i18n::t;
use colony_ui::{Palette, Typography};
use iced::widget::text_editor::Content as EditorContent;
use iced::widget::{button, container, row, tooltip};
use iced::{Alignment, Element, Length};
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug)]
pub struct Tab {
    /// Unique for the life of the app, so a read or write that finishes in
    /// the background finds its own tab even when another one has the same
    /// file name.
    pub id: u64,
    pub filename: String,
    /// The file this buffer was last loaded from or saved to. `None` for a new
    /// tab, which must go through Save As before it is written anywhere.
    pub disk_path: Option<String>,
    pub content: EditorContent,
    pub buffer: TextBuffer,
    pub viewport_cache: ViewportCache,
    /// `content.text()` when the tab last matched its file, so a tab counts as
    /// modified only when its text differs from what is on disk.
    pub last_saved_text: String,
    pub highlight_settings: highlight::Settings,
    pub multi_cursors: Vec<Position>,
    pub diagnostics: Vec<Diagnostic>,
    pub completion_items: Vec<CompletionItem>,
    pub completion_prefix: String,
    pub file_is_lossy: bool,
    pub lossy_save_acknowledged: bool,
    pub suppress_undo_snapshot: bool,
    pub search_matches: Vec<MatchPosition>,
    pub current_match_index: Option<usize>,
    pub scroll_offset: i32,
}

impl Tab {
    pub fn new(filename: &str, text: &str) -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        let buffer = TextBuffer::from(text);
        let diagnostics = diagnostics::analyze(&buffer);
        let language = std::path::Path::new(filename)
            .extension()
            .and_then(|ext| ext.to_str())
            .map(highlight::Language::from_extension)
            .unwrap_or(highlight::Language::Plain);
        let content = EditorContent::with_text(text);
        let mut viewport_cache = ViewportCache::new();
        viewport_cache.update(&buffer, 0, DEFAULT_VIEWPORT_HEIGHT);
        Self {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            filename: filename.to_string(),
            disk_path: None,
            last_saved_text: content.text(),
            content,
            buffer,
            viewport_cache,
            highlight_settings: highlight::Settings {
                language,
                buffer_text: text.into(),
                ..highlight::Settings::default()
            },
            multi_cursors: Vec::new(),
            diagnostics,
            completion_items: Vec::new(),
            completion_prefix: String::new(),
            file_is_lossy: false,
            lossy_save_acknowledged: false,
            suppress_undo_snapshot: false,
            search_matches: Vec::new(),
            current_match_index: None,
            scroll_offset: 0,
        }
    }

    pub fn is_modified(&self) -> bool {
        self.content.text() != self.last_saved_text
    }

    /// Shows `text` read from `path`, starting a fresh undo history. Callers
    /// check `is_modified` first, so unsaved edits are never replaced.
    pub fn load_text(&mut self, path: &str, text: String, lossy: bool) {
        self.disk_path = Some(path.to_string());
        self.file_is_lossy = lossy;
        self.lossy_save_acknowledged = false;
        self.content = EditorContent::with_text(&text);
        self.buffer.replace(&text);
        self.last_saved_text = self.content.text();
        self.buffer.clear_history();
        self.multi_cursors.clear();
        self.completion_items.clear();
        self.suppress_undo_snapshot = false;
        self.scroll_offset = 0;
    }

    pub fn display_name(&self) -> &str {
        if self.filename.trim().is_empty() {
            UNTITLED
        } else {
            &self.filename
        }
    }
}

impl RoxanneApp {
    pub(super) fn select_tab(&mut self, index: usize) {
        if index < self.tabs.len() {
            self.active_tab = index;
            self.refresh_search_matches(false, false);
            self.refresh_viewport_cache();
        }
    }

    /// Closes the tab at `index`. The last tab is replaced by an empty one.
    pub(super) fn close_tab(&mut self, index: usize) {
        if index >= self.tabs.len() {
            return;
        }
        if self.tabs.len() == 1 {
            self.tabs[0] = Tab::new(UNTITLED, "");
            self.active_tab = 0;
        } else {
            self.tabs.remove(index);
            if self.active_tab >= self.tabs.len() {
                self.active_tab = self.tabs.len() - 1;
            } else if self.active_tab > index {
                self.active_tab -= 1;
            }
        }
        self.refresh_search_matches(false, false);
        self.refresh_viewport_cache();
    }

    pub(super) fn new_tab(&mut self) {
        self.tabs.push(Tab::new(UNTITLED, ""));
        self.active_tab = self.tabs.len() - 1;
        self.refresh_viewport_cache();
    }

    pub(super) fn cycle_tab(&mut self, forward: bool) {
        let count = self.tabs.len();
        let index = if forward {
            (self.active_tab + 1) % count
        } else {
            (self.active_tab + count - 1) % count
        };
        self.select_tab(index);
    }

    pub(super) fn tab_bar(&self, typo: &Typography) -> Element<'_, Message> {
        let mut tabs = row![].spacing(2).padding([4, 8]).align_y(Alignment::Center);

        for (index, tab) in self.tabs.iter().enumerate() {
            let active = index == self.active_tab;
            let mut name = row![style::button_label(typo, tab.display_name(), 13)]
                .spacing(6)
                .align_y(Alignment::Center);
            if tab.is_modified() {
                // A filled dot, the usual mark of unsaved changes.
                name = name.push(
                    style::label(typo, "\u{25cf}", 10, Palette::ACCENT()).font(style::code_font()),
                );
            }
            let close = tooltip(
                button(
                    style::button_label(typo, colony_ui::widgets::icons::CLOSE, 11)
                        .font(colony_ui::fonts::glyph_font()),
                )
                .padding([4, 6])
                .style(style::flat)
                .on_press(Message::TabClosed(index)),
                container(style::label(
                    typo,
                    t("tab_close"),
                    11,
                    Palette::TEXT_PRIMARY(),
                ))
                .padding([2, 6])
                .style(style::surface(Palette::BG_CARD())),
                tooltip::Position::Bottom,
            );
            tabs = tabs.push(
                row![
                    button(name)
                        .padding([6, 12])
                        .style(style::tab(active))
                        .on_press(Message::TabSelected(index)),
                    close,
                ]
                .align_y(Alignment::Center),
            );
        }

        tabs = tabs.push(
            button(style::button_label(typo, "+", 14).font(style::code_font()))
                .padding([4, 8])
                .style(style::flat)
                .on_press(Message::NewTab),
        );

        container(tabs)
            .width(Length::Fill)
            .style(style::surface(Palette::BG_SIDEBAR()))
            .into()
    }
}
