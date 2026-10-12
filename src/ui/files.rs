//! Reading and writing files, Save As, changes made by other programs, and
//! the rust-analyzer requests that open other files.

use super::tabs::Tab;
use super::{
    FileLoadResult, Message, RoxanneApp, SearchResultLoadResult, comparable_path, format_duration,
    kept_edits_message,
};
use crate::file_ops::{self, MAX_OPEN_FILE_SIZE};
use crate::i18n::{t, t_fmt};
use crate::lsp;
use crate::search::SearchResult;
use iced::Task;
use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::time::Instant;

/// Reads `path` as text, replacing invalid UTF-8. Returns the text and
/// whether anything was replaced.
fn read_text(path: &Path) -> Result<(String, bool), String> {
    let metadata = std::fs::metadata(path).map_err(|err| err.to_string())?;
    if metadata.len() > MAX_OPEN_FILE_SIZE {
        return Err(t_fmt(
            "file_error_too_large",
            &[("size", &metadata.len()), ("limit", &MAX_OPEN_FILE_SIZE)],
        ));
    }
    let bytes = std::fs::read(path).map_err(|err| err.to_string())?;
    let text = String::from_utf8_lossy(&bytes);
    let lossy = matches!(text, Cow::Owned(_));
    Ok((text.into_owned(), lossy))
}

impl RoxanneApp {
    /// Opens `path` in its tab if one shows it already, else in a new tab.
    pub(super) fn open_path(&mut self, path: String) -> Task<Message> {
        if let Some(index) = self.tabs.iter().position(|tab| tab.filename == path) {
            self.select_tab(index);
            return Task::none();
        }
        self.tabs.push(Tab::new(&path, ""));
        self.active_tab = self.tabs.len() - 1;
        self.open_file()
    }

    /// Loads the file named in the active tab's file name field.
    pub(super) fn open_file(&mut self) -> Task<Message> {
        let filename = self.tab().filename.clone();
        if filename.trim().is_empty() {
            self.status_message = Some(t("status_file_name_missing").to_string());
            return Task::none();
        }
        // If file is already open in another tab, just switch
        if let Some(index) = self.tabs.iter().position(|tab| tab.filename == filename)
            && index != self.active_tab
        {
            self.select_tab(index);
            return Task::none();
        }
        self.read_file(self.tab().id, filename)
    }

    /// Reads `path` into the tab `tab_id` once the read is done.
    pub(super) fn read_file(&mut self, tab_id: u64, path: String) -> Task<Message> {
        self.perf_file_open_started = Some(Instant::now());
        Task::perform(
            async move {
                let (text, lossy) = read_text(Path::new(&path))?;
                Ok(FileLoadResult {
                    tab_id,
                    path,
                    text,
                    lossy,
                })
            },
            Message::FileLoaded,
        )
    }

    pub(super) fn file_loaded(&mut self, result: Result<FileLoadResult, String>) {
        if let Some(started) = self.perf_file_open_started.take() {
            self.performance.last_file_open = Some(started.elapsed());
        }
        let load = match result {
            Ok(load) => load,
            Err(message) => {
                self.status_message = Some(t_fmt("status_open_failed", &[("error", &message)]));
                return;
            }
        };
        // The tab may have been closed while the file was read.
        let Some(index) = self.tab_index(load.tab_id) else {
            return;
        };
        self.active_tab = index;
        if self.tab().is_modified() {
            self.status_message = Some(kept_edits_message(&load.path));
            self.refresh_search_matches(false, false);
            self.refresh_viewport_cache();
            return;
        }
        self.tab_mut().load_text(&load.path, load.text, load.lossy);
        self.completion_panel_open = false;
        let text = self.tab().content.text();
        let filename = self.tab().filename.clone();
        self.plugins.on_file_opened(&text, &filename);
        self.text_changed(false);
        let duration = self
            .performance
            .last_file_open
            .map_or_else(|| "-".to_string(), format_duration);
        self.status_message = Some(if load.lossy {
            t_fmt("status_file_loaded_lossy", &[("duration", &duration)])
        } else {
            t_fmt("status_file_loaded", &[("duration", &duration)])
        });
    }

    pub(super) fn save_file(&mut self) -> Task<Message> {
        let tab = self.tab();
        // A new tab, or one whose file name was edited, asks where to save
        // instead of writing a name relative to the working directory or
        // replacing another file unasked.
        if tab.disk_path.as_deref() != Some(tab.filename.as_str()) {
            return self.open_save_as();
        }
        self.write_tab()
    }

    pub(super) fn open_save_as(&mut self) -> Task<Message> {
        self.save_as_input = self.tab().filename.clone();
        self.save_as_overwrite = None;
        self.save_as_open = true;
        Task::none()
    }

    pub(super) fn submit_save_as(&mut self) -> Task<Message> {
        let new_name = self.save_as_input.trim().to_string();
        if new_name.is_empty() {
            self.status_message = Some(t("status_save_as_name_missing").to_string());
            return Task::none();
        }
        // Ask about replaced characters while the panel is still open
        // and the tab keeps its name, so the next submit writes.
        if self.asks_lossy_confirmation() {
            return Task::none();
        }
        // symlink_metadata also sees a dangling symlink, which a save
        // would replace.
        let replaces_other_file = self.tab().disk_path.as_deref() != Some(&new_name)
            && std::fs::symlink_metadata(&new_name).is_ok();
        if replaces_other_file && self.save_as_overwrite.as_deref() != Some(&new_name) {
            self.status_message = Some(t_fmt("status_save_as_exists", &[("path", &new_name)]));
            self.save_as_overwrite = Some(new_name);
            return Task::none();
        }
        self.save_as_overwrite = None;
        self.tab_mut().filename = new_name;
        self.save_as_open = false;
        self.write_tab()
    }

    /// Writes the active tab to its file name.
    pub(super) fn write_tab(&mut self) -> Task<Message> {
        if self.asks_lossy_confirmation() {
            return Task::none();
        }
        if self.tab().file_is_lossy {
            self.tab_mut().lossy_save_acknowledged = false;
        }
        let (tab_id, filename) = (self.tab().id, self.tab().filename.clone());
        let text = self.tab().content.text();
        self.tab_mut().buffer.replace(&text);
        self.perf_file_save_started = Some(Instant::now());
        Task::perform(
            async move { file_ops::atomic_write(&filename, &text).map(|()| (tab_id, filename, text)) },
            Message::FileSaved,
        )
    }

    pub(super) fn file_saved(&mut self, result: Result<(u64, String, String), String>) {
        if let Some(started) = self.perf_file_save_started.take() {
            self.performance.last_file_save = Some(started.elapsed());
        }
        match result {
            Ok((tab_id, path, saved_text)) => {
                self.plugins.on_file_saved(&saved_text, &path);
                // Only the text that was written counts as saved:
                // edits typed during the write stay marked as modified.
                if let Some(index) = self.tab_index(tab_id) {
                    let tab = &mut self.tabs[index];
                    tab.last_saved_text = saved_text;
                    tab.disk_path = Some(path);
                }
                let duration = self
                    .performance
                    .last_file_save
                    .map_or_else(|| "-".to_string(), format_duration);
                self.status_message = Some(t_fmt("status_file_saved", &[("duration", &duration)]));
            }
            Err(message) => {
                self.status_message = Some(t_fmt("status_save_failed", &[("error", &message)]));
            }
        }
    }

    /// For a tab whose invalid characters were replaced on load: asks once and
    /// returns true, so the save stops until the user confirms with a second one.
    pub(super) fn asks_lossy_confirmation(&mut self) -> bool {
        if !self.tab().file_is_lossy || self.tab().lossy_save_acknowledged {
            return false;
        }
        self.tab_mut().lossy_save_acknowledged = true;
        self.status_message = Some(t("status_save_lossy_confirm").to_string());
        true
    }

    pub(super) fn tab_index(&self, tab_id: u64) -> Option<usize> {
        self.tabs.iter().position(|tab| tab.id == tab_id)
    }

    /// The tab whose file is `path`, however either is spelled (the watcher
    /// reports absolute paths): the active tab when it matches, else the first.
    pub(super) fn tab_index_for_disk_path(&self, path: &str) -> Option<usize> {
        let wanted = comparable_path(path);
        let shows = |tab: &Tab| {
            tab.disk_path
                .as_deref()
                .is_some_and(|disk_path| comparable_path(disk_path) == wanted)
        };
        if shows(self.tab()) {
            return Some(self.active_tab);
        }
        self.tabs.iter().position(shows)
    }

    /// Reloads the file another program changed, unless its tab has unsaved
    /// edits.
    pub(super) fn reload_changed_file(&mut self) -> Task<Message> {
        let Some(path) = self.file_changed_externally.take() else {
            return Task::none();
        };
        let Some(index) = self.tab_index_for_disk_path(&path) else {
            self.status_message = Some(t_fmt("status_file_not_open", &[("path", &path)]));
            return Task::none();
        };
        let tab = &self.tabs[index];
        if tab.is_modified() {
            self.status_message = Some(kept_edits_message(&path));
            return Task::none();
        }
        // The tab's own spelling of the path, which Save compares
        // with the file name.
        let (tab_id, disk_path) = (tab.id, tab.disk_path.clone().unwrap_or(path));
        self.read_file(tab_id, disk_path)
    }

    /// Opens `result.path` in a tab, or switches to the tab showing it, and
    /// moves the cursor to the result.
    pub(super) fn open_location(&mut self, result: SearchResult) -> Task<Message> {
        Task::perform(
            async move {
                let (text, lossy) = read_text(&result.path)?;
                Ok(SearchResultLoadResult {
                    text,
                    result,
                    lossy,
                })
            },
            Message::SearchResultOpened,
        )
    }

    pub(super) fn search_result_opened(&mut self, result: Result<SearchResultLoadResult, String>) {
        let load = match result {
            Ok(load) => load,
            Err(message) => {
                self.status_message =
                    Some(t_fmt("status_search_files_error", &[("error", &message)]));
                return;
            }
        };
        let found = load.result;
        let path = found.path.display().to_string();
        match self.tabs.iter().position(|tab| tab.filename == path) {
            Some(index) => self.active_tab = index,
            None => {
                self.tabs.push(Tab::new(&path, ""));
                self.active_tab = self.tabs.len() - 1;
            }
        }
        let kept_edits = self.tab().is_modified();
        if !kept_edits {
            self.tab_mut().load_text(&path, load.text, load.lossy);
        }
        self.completion_panel_open = false;
        self.text_changed(false);
        self.jump_to_position(found.line, found.column);
        let line = found.line + 1;
        let column = found.column + 1;
        self.status_message = Some(if kept_edits {
            kept_edits_message(&path)
        } else if load.lossy {
            t_fmt(
                "status_search_opened_lossy",
                &[("path", &path), ("line", &line), ("column", &column)],
            )
        } else {
            t_fmt(
                "status_search_opened",
                &[("path", &path), ("line", &line), ("column", &column)],
            )
        });
    }

    pub(super) fn lsp_hover(&mut self) -> Task<Message> {
        match self.lsp_target() {
            Some((session, path, text, line, character)) => Task::perform(
                session.hover(path, text, line, character),
                Message::LspHoverResult,
            ),
            None => Task::none(),
        }
    }

    pub(super) fn lsp_goto_definition(&mut self) -> Task<Message> {
        match self.lsp_target() {
            Some((session, path, text, line, character)) => Task::perform(
                session.definition(path, text, line, character),
                Message::LspDefinitionResult,
            ),
            None => Task::none(),
        }
    }

    /// The rust-analyzer session for the active tab's workspace, started on
    /// first use, with the file, its text and the cursor to ask about. `None`,
    /// with a status message, when the LSP is turned off or the tab is not a
    /// saved Rust file.
    fn lsp_target(&mut self) -> Option<(&lsp::LspSession, PathBuf, String, u32, u32)> {
        if !self.config.lsp.enabled {
            self.status_message = Some(t("status_lsp_disabled").to_string());
            return None;
        }
        let tab = self.tab();
        let Some(path) = tab
            .disk_path
            .as_deref()
            .map(comparable_path)
            .filter(|path| path.extension().is_some_and(|ext| ext == "rs"))
        else {
            self.status_message = Some(t("status_lsp_rust_only").to_string());
            return None;
        };
        // The editor counts bytes in a line, the server counts characters.
        let (line, byte_column) = self.cursor_position();
        let character = tab
            .buffer
            .line(line)
            .and_then(|text| text.get(..byte_column))
            .map_or(byte_column, |before| before.chars().count());
        let text = tab.buffer.text().to_string();
        let root = lsp::workspace_root(&path);
        if self
            .lsp
            .as_ref()
            .is_none_or(|session| session.root() != root || !session.is_running())
        {
            // Replacing a session shuts its server down in the background.
            self.lsp = Some(lsp::LspSession::start(&self.config.lsp.rust_analyzer, root));
        }
        self.status_message = Some(t("status_lsp_waiting").to_string());
        let session = self.lsp.as_ref()?;
        Some((session, path, text, line as u32, character as u32))
    }

    pub(super) fn open_definition(&mut self, location: lsp::LspLocation) -> Task<Message> {
        let Some(index) = self.tab_index_for_disk_path(&location.path.display().to_string()) else {
            return self.open_location(SearchResult {
                path: location.path,
                line: location.line,
                column: location.column,
                preview: String::new(),
            });
        };
        self.active_tab = index;
        self.refresh_search_matches(false, false);
        self.jump_to_position(location.line, location.column);
        self.status_message = Some(t_fmt(
            "status_lsp_definition",
            &[
                ("line", &(location.line + 1)),
                ("column", &(location.column + 1)),
            ],
        ));
        Task::none()
    }
}
