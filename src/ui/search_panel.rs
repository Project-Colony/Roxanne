//! Find and replace in the active file, and search across the workspace.

use super::style;
use super::{Message, RoxanneApp, format_duration};
use crate::editor::Position;
use crate::i18n::{t, t_fmt};
use crate::search::{self, SearchOptions, SearchResultsSummary, SearchScope};
use colony_ui::{Palette, Typography};
use iced::widget::{button, column, container, row, scrollable, text_input};
use iced::{Alignment, Element, Length, Task};
use std::path::{Path, PathBuf};
use std::time::Instant;

/// The status bar's search field, which Find puts the cursor in.
pub(super) const SEARCH_INPUT: &str = "roxanne-search";
/// Match case and regular expression, written as the symbols every editor
/// uses and named in their tooltips.
const CASE_SYMBOL: &str = "Aa";
const REGEX_SYMBOL: &str = ".*";

impl RoxanneApp {
    /// The match case and regular expression toggles.
    pub(super) fn match_toggles(&self, typo: &Typography) -> Element<'_, Message> {
        row![
            style::tip(
                typo,
                toggle_button(
                    typo,
                    CASE_SYMBOL,
                    self.search_case_sensitive,
                    Message::SearchToggleCaseSensitive,
                ),
                t("search_match_case"),
            ),
            style::tip(
                typo,
                toggle_button(
                    typo,
                    REGEX_SYMBOL,
                    self.search_regex,
                    Message::SearchToggleRegex
                ),
                t("search_regex"),
            ),
        ]
        .spacing(8)
        .into()
    }

    pub(super) fn search_options(&self) -> SearchOptions {
        SearchOptions {
            regex: self.search_regex,
            case_sensitive: self.search_case_sensitive,
            include_hidden: self.search_include_hidden,
        }
    }

    pub(super) fn refresh_current_file_search(&mut self) {
        if self.search_scope == SearchScope::CurrentFile {
            self.refresh_search_matches(false, true);
        }
    }

    pub(super) fn refresh_search_matches(&mut self, preserve_index: bool, update_status: bool) {
        let options = self.search_options();
        let query_given = !self.search_query.is_empty();
        let matches = match search::find_matches(&self.tab().buffer, &self.search_query, options) {
            Ok(matches) => matches,
            Err(message) => {
                let tab = self.tab_mut();
                tab.current_match_index = None;
                tab.search_matches.clear();
                tab.highlight_settings.search_matches.clear();
                if update_status && query_given {
                    self.status_message =
                        Some(t_fmt("status_search_error", &[("error", &message)]));
                }
                return;
            }
        };
        let tab = self.tab_mut();
        tab.highlight_settings.search_matches = matches.clone();
        tab.search_matches = matches;
        let count = tab.search_matches.len();
        tab.current_match_index = if preserve_index {
            tab.current_match_index.filter(|index| *index < count)
        } else {
            None
        };
        if update_status && query_given {
            self.status_message = Some(if count == 0 {
                t("status_search_none").to_string()
            } else {
                t_fmt("status_search_count", &[("count", &count)])
            });
        }
    }

    pub(super) fn find_next_match(&mut self, forward: bool) {
        if self.search_query.trim().is_empty() {
            self.status_message = Some(t("status_search_empty").to_string());
            return;
        }
        let total = self.tab().search_matches.len();
        if total == 0 {
            self.status_message = Some(t("status_search_none").to_string());
            return;
        }
        let next_index = match (self.tab().current_match_index, forward) {
            (Some(index), true) => (index + 1) % total,
            (Some(index), false) => (index + total - 1) % total,
            (None, true) => 0,
            (None, false) => total - 1,
        };
        self.tab_mut().current_match_index = Some(next_index);
        self.jump_to_match(next_index);
    }

    fn jump_to_match(&mut self, index: usize) {
        if let Some(found) = self.tab().search_matches.get(index).copied() {
            self.jump_to_position(found.line, found.column);
            self.status_message = Some(t_fmt(
                "status_search_position",
                &[
                    ("index", &(index + 1)),
                    ("count", &self.tab().search_matches.len()),
                    ("line", &(found.line + 1)),
                    ("column", &(found.column + 1)),
                ],
            ));
        }
    }

    pub(super) fn search_in_files(&mut self) -> Task<Message> {
        if self.search_query.trim().is_empty() {
            self.status_message = Some(t("status_search_files_empty").to_string());
            return Task::none();
        }

        if self.search_scope == SearchScope::CurrentFile {
            let started = Instant::now();
            self.refresh_search_matches(false, true);
            self.performance.last_search = Some(started.elapsed());
            let duration = self
                .performance
                .last_search
                .map_or_else(|| "-".to_string(), format_duration);
            self.status_message = Some(t_fmt(
                "status_search_file_done",
                &[
                    ("count", &self.tab().search_matches.len()),
                    ("duration", &duration),
                ],
            ));
            return Task::none();
        }

        let query = self.search_query.clone();
        let options = self.search_options();
        let root = match self.resolve_workspace_root() {
            Ok(path) => path,
            Err(err) => {
                self.status_message = Some(t_fmt("status_search_files_error", &[("error", &err)]));
                return Task::none();
            }
        };

        self.perf_search_files_started = Some(Instant::now());
        Task::perform(
            search::search_in_workspace(root, query, options),
            Message::SearchResultsLoaded,
        )
    }

    pub(super) fn search_results_loaded(&mut self, result: Result<SearchResultsSummary, String>) {
        if let Some(started) = self.perf_search_files_started.take() {
            self.performance.last_search_files = Some(started.elapsed());
        }
        let summary = match result {
            Ok(summary) => summary,
            Err(message) => {
                self.status_message =
                    Some(t_fmt("status_search_files_error", &[("error", &message)]));
                return;
            }
        };
        self.search_results = summary.results;
        let duration = self
            .performance
            .last_search_files
            .map_or_else(|| "-".to_string(), format_duration);
        let mut details = Vec::new();
        for (count, key) in [
            (summary.skipped_too_large, "search_skipped_too_large"),
            (summary.skipped_invalid_utf8, "search_skipped_not_utf8"),
            (summary.skipped_read_errors, "search_skipped_read_errors"),
        ] {
            if count > 0 {
                details.push(t_fmt(key, &[("count", &count)]));
            }
        }
        let mut message = t_fmt(
            "status_search_files_done",
            &[
                ("count", &self.search_results.len()),
                ("duration", &duration),
            ],
        );
        if !details.is_empty() {
            message.push(' ');
            let skipped = summary.skipped_too_large
                + summary.skipped_invalid_utf8
                + summary.skipped_read_errors;
            message.push_str(&t_fmt(
                "status_search_files_skipped",
                &[("count", &skipped), ("details", &details.join(", "))],
            ));
        }
        if summary.truncated {
            message.push(' ');
            message.push_str(t("status_search_files_truncated"));
        }
        self.status_message = Some(message);
    }

    pub(super) fn replace_current_match(&mut self) {
        let Some(match_index) = self.tab().current_match_index else {
            self.status_message = Some(t("status_replace_no_selection").to_string());
            return;
        };
        let Some(found) = self.tab().search_matches.get(match_index).copied() else {
            return;
        };
        let start = Position::new(found.line, found.column);
        let end = Position::new(found.line, found.column + found.length);
        self.record_undo_snapshot();
        let replacement = self.replace_text.clone();
        self.tab_mut().buffer.delete_range(start, end);
        self.tab_mut().buffer.insert(start, &replacement);
        let text = self.tab().buffer.text().to_string();
        self.replace_text_of_tab(&text);
        self.text_changed(false);
        // Move on to the next match.
        let count = self.tab().search_matches.len();
        if count == 0 {
            self.status_message = Some(t("status_replace_last").to_string());
            return;
        }
        let next = match_index.min(count - 1);
        self.tab_mut().current_match_index = Some(next);
        self.jump_to_match(next);
        self.status_message = Some(t_fmt(
            "status_replace_one",
            &[("index", &(next + 1)), ("count", &count)],
        ));
    }

    pub(super) fn replace_all_matches(&mut self) {
        let count = self.tab().search_matches.len();
        if count == 0 {
            self.status_message = Some(t("status_replace_none").to_string());
            return;
        }
        self.record_undo_snapshot();

        let text = if self.search_regex {
            // Regex-aware, so the replacement can use capture groups ($1).
            match search::replace_with_captures(
                self.tab().buffer.text(),
                &self.search_query,
                &self.replace_text,
                true,
                self.search_case_sensitive,
            ) {
                Ok(text) => text,
                Err(err) => {
                    self.status_message = Some(t_fmt("status_replace_error", &[("error", &err)]));
                    return;
                }
            }
        } else {
            // From the end, so earlier positions stay valid.
            let mut matches = self.tab().search_matches.clone();
            matches.sort_by(|a, b| b.line.cmp(&a.line).then(b.column.cmp(&a.column)));
            let replacement = self.replace_text.clone();
            for found in &matches {
                let start = Position::new(found.line, found.column);
                let end = Position::new(found.line, found.column + found.length);
                self.tab_mut().buffer.delete_range(start, end);
                self.tab_mut().buffer.insert(start, &replacement);
            }
            self.tab().buffer.text().to_string()
        };
        self.replace_text_of_tab(&text);
        self.text_changed(false);
        self.status_message = Some(t_fmt("status_replace_all", &[("count", &count)]));
    }

    pub(super) fn open_search_result(&mut self, index: usize) -> Task<Message> {
        match self.search_results.get(index).cloned() {
            Some(result) => self.open_location(result),
            None => Task::none(),
        }
    }

    pub(super) fn resolve_workspace_root(&self) -> Result<PathBuf, String> {
        if let Some(root) = Self::workspace_root_from_filename(&self.tab().filename) {
            return Ok(root);
        }
        std::env::current_dir().map_err(|err| err.to_string())
    }

    pub(super) fn workspace_root_from_filename(filename: &str) -> Option<PathBuf> {
        let trimmed = filename.trim();
        if trimmed.is_empty() {
            return None;
        }

        let path = Path::new(trimmed);
        let base_dir = if path.is_dir() { path } else { path.parent()? };

        for ancestor in base_dir.ancestors() {
            if ancestor.join(".roxanne.toml").is_file() {
                return Some(ancestor.to_path_buf());
            }
        }

        Some(base_dir.to_path_buf())
    }

    pub(super) fn search_panel(&self, typo: &Typography) -> Option<Element<'_, Message>> {
        if !self.search_panel_open {
            return None;
        }

        let header = row![
            style::label(typo, t("search_panel_title"), 12, Palette::TEXT_PRIMARY()),
            style::label(
                typo,
                t_fmt(
                    "search_result_count",
                    &[("count", &self.search_results.len())]
                ),
                12,
                Palette::TEXT_MUTED(),
            ),
        ]
        .spacing(12)
        .align_y(Alignment::Center);

        let query_row = row![
            text_input(t("search_placeholder_workspace"), &self.search_query)
                .on_input(Message::SearchChanged)
                .on_submit(Message::SearchInFiles)
                .padding([4, 8])
                .size(typo.sz(12))
                .font(typo.regular)
                .style(style::input),
            self.match_toggles(typo),
            toggle_button(
                typo,
                t("search_hidden_files"),
                self.search_include_hidden,
                Message::SearchToggleIncludeHidden
            ),
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        let replace_row = row![
            text_input(t("replace_placeholder"), &self.replace_text)
                .on_input(Message::ReplaceChanged)
                .padding([4, 8])
                .size(typo.sz(12))
                .font(typo.regular)
                .style(style::input),
            panel_button(typo, t("replace_one"), Message::ReplaceCurrent),
            panel_button(typo, t("replace_all"), Message::ReplaceAll),
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        let scope_row = row![
            toggle_button(
                typo,
                SearchScope::CurrentFile.label(),
                self.search_scope == SearchScope::CurrentFile,
                Message::SearchScopeSelected(SearchScope::CurrentFile)
            ),
            toggle_button(
                typo,
                SearchScope::Workspace.label(),
                self.search_scope == SearchScope::Workspace,
                Message::SearchScopeSelected(SearchScope::Workspace)
            ),
            panel_button(typo, t("search_run"), Message::SearchInFiles),
            panel_button(typo, t("search_clear"), Message::SearchResultsCleared),
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        let results: Element<'_, Message> = if self.search_results.is_empty() {
            style::label(typo, t("search_no_results"), 12, Palette::TEXT_DIMMER()).into()
        } else {
            let entries = self
                .search_results
                .iter()
                .enumerate()
                .map(|(index, result)| {
                    let title = style::label(
                        typo,
                        format!(
                            "{}:{}:{}",
                            result.path.display(),
                            result.line + 1,
                            result.column + 1
                        ),
                        12,
                        Palette::TEXT_SECONDARY(),
                    );
                    let preview =
                        style::label(typo, result.preview.trim(), 12, Palette::TEXT_MUTED())
                            .font(style::code_font());
                    button(column![title, preview].spacing(2))
                        .width(Length::Fill)
                        .padding([4, 12])
                        .style(style::flat)
                        .on_press(Message::SearchResultSelected(index))
                        .into()
                });
            scrollable(column(entries).spacing(4))
                .height(Length::Fixed(180.0 * typo.scale))
                .into()
        };

        let panel = column![header, query_row, replace_row, scope_row, results]
            .spacing(10)
            .padding([8, 16]);

        Some(
            container(panel)
                .width(Length::Fill)
                .style(style::surface(Palette::BG_CARD()))
                .into(),
        )
    }
}

/// An on/off option, marked like a selected item: accent fill.
pub(super) fn toggle_button<'a>(
    typo: &Typography,
    label: &'a str,
    active: bool,
    message: Message,
) -> Element<'a, Message> {
    button(style::button_label(typo, label, 12))
        .padding([2, 8])
        .style(colony_ui::widgets::selection_style(active))
        .on_press(message)
        .into()
}

/// An action of a panel.
pub(super) fn panel_button<'a>(
    typo: &Typography,
    label: &'a str,
    message: Message,
) -> Element<'a, Message> {
    button(style::button_label(typo, label, 12))
        .padding([4, 10])
        .style(style::card)
        .on_press(message)
        .into()
}
