//! Changing the text of the active tab: edits, paste, multiple cursors, undo
//! and redo, completions, and moving the cursor.

use super::{Message, RoxanneApp};
use crate::completion;
use crate::diagnostics;
use crate::editor::Position;
use crate::i18n::{t, t_fmt};
use iced::widget::text_editor::{
    Action as EditorAction, Content as EditorContent, Cursor, Edit as EditorEdit, Motion,
    Position as EditorPosition,
};
use iced::{Task, clipboard};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};

impl RoxanneApp {
    pub(super) fn on_edit(&mut self, action: EditorAction) {
        // Track explicit scroll events
        if let EditorAction::Scroll { lines } = &action {
            let tab = self.tab_mut();
            tab.scroll_offset = (tab.scroll_offset + lines).max(0);
        }
        let lines_before = self.tab().buffer.line_count();
        let is_edit = action.is_edit();
        if is_edit {
            self.record_undo_snapshot();
        }
        match action {
            EditorAction::Edit(edit) if !self.tab().multi_cursors.is_empty() => {
                self.apply_multi_cursor_edit(&edit);
            }
            action => {
                self.tab_mut().content.perform(action);
                let text = self.tab().content.text();
                self.tab_mut().buffer.replace(&text);
            }
        }
        // When lines are removed, the iced widget's internal scroll
        // state becomes stale. Rebuild Content to force a reset.
        if self.tab().buffer.line_count() != lines_before {
            self.resync_widget_scroll();
        }
        self.clamp_scroll_to_cursor();
        if is_edit {
            self.text_changed(true);
        } else {
            self.refresh_viewport_cache();
        }
    }

    pub(super) fn paste(&mut self, text: Option<String>) {
        let Some(text) = text.filter(|text| !text.is_empty()) else {
            return;
        };
        self.insert(EditorEdit::Paste(Arc::new(text)));
    }

    /// Inserts `edit` at every cursor.
    fn insert(&mut self, edit: EditorEdit) {
        self.record_undo_snapshot();
        if self.tab().multi_cursors.is_empty() {
            self.tab_mut().content.perform(EditorAction::Edit(edit));
            let text = self.tab().content.text();
            self.tab_mut().buffer.replace(&text);
        } else {
            self.apply_multi_cursor_edit(&edit);
        }
        self.text_changed(true);
    }

    /// Everything that follows a change of the active tab's text: highlight,
    /// search matches, diagnostics, plugins and the visible lines.
    pub(super) fn text_changed(&mut self, keep_match: bool) {
        self.sync_highlight_buffer();
        self.refresh_search_matches(keep_match, true);
        self.refresh_diagnostics();
        let text = self.tab().content.text();
        let filename = self.tab().filename.clone();
        self.plugins.on_text_changed(&text, &filename);
        self.refresh_viewport_cache();
    }

    pub(super) fn copy(&mut self) -> Task<Message> {
        match self.tab().content.selection() {
            Some(selection) if !selection.is_empty() => clipboard::write(selection),
            _ => Task::none(),
        }
    }

    pub(super) fn cut(&mut self) -> Task<Message> {
        let Some(selection) = self.tab().content.selection().filter(|s| !s.is_empty()) else {
            return Task::none();
        };
        self.record_undo_snapshot();
        self.tab_mut()
            .content
            .perform(EditorAction::Edit(EditorEdit::Backspace));
        let text = self.tab().content.text();
        self.tab_mut().buffer.replace(&text);
        self.text_changed(true);
        clipboard::write(selection)
    }

    pub(super) fn select_all(&mut self) {
        self.tab_mut().content.perform(EditorAction::SelectAll);
        self.refresh_viewport_cache();
    }

    pub(super) fn add_cursor_next_match(&mut self) {
        if self.tab().search_matches.is_empty() {
            self.status_message = Some(t("status_cursors_no_match").to_string());
            return;
        }

        let (line, column) = self.cursor_position();
        let next_match = self
            .tab()
            .search_matches
            .iter()
            .find(|found| found.line > line || (found.line == line && found.column > column))
            .or_else(|| self.tab().search_matches.first())
            .copied();

        if let Some(found) = next_match {
            let position = Position::new(found.line, found.column);
            if !self.tab().multi_cursors.contains(&position) {
                self.tab_mut().multi_cursors.push(position);
            }
            self.status_message = Some(t("status_cursors_added").to_string());
        }
    }

    pub(super) fn add_cursors_all_matches(&mut self) {
        if self.tab().search_matches.is_empty() {
            self.status_message = Some(t("status_cursors_no_match").to_string());
            return;
        }
        let (line, column) = self.cursor_position();
        let primary = Position::new(line, column);
        let cursors: Vec<Position> = self
            .tab()
            .search_matches
            .iter()
            .map(|found| Position::new(found.line, found.column))
            .filter(|position| *position != primary)
            .collect();
        let count = cursors.len();
        self.tab_mut().multi_cursors = cursors;
        self.status_message = Some(t_fmt("status_cursors_count", &[("count", &count)]));
    }

    pub(super) fn clear_cursors(&mut self) {
        self.tab_mut().multi_cursors.clear();
        self.status_message = Some(t("status_cursors_cleared").to_string());
    }

    fn apply_multi_cursor_edit(&mut self, edit: &EditorEdit) {
        let (line, column) = self.cursor_position();
        let mut cursor_positions = Vec::with_capacity(self.tab().multi_cursors.len() + 1);
        cursor_positions.push(Position::new(line, column));
        cursor_positions.extend(self.tab().multi_cursors.iter().copied());

        let mut seen_indices = HashSet::new();
        let mut cursor_slots = Vec::new();
        for (id, position) in cursor_positions.iter().copied().enumerate() {
            let index = self.tab().buffer.index_from_position(position);
            if seen_indices.insert(index) {
                cursor_slots.push(CursorSlot { id, index });
            }
        }

        cursor_slots.sort_by_key(|slot| slot.index);
        let text = self.tab().buffer.text().to_string();

        let mut operations: Vec<EditOperation> = cursor_slots
            .iter()
            .filter_map(|slot| EditOperation::from_edit(slot, edit, &text))
            .collect();
        operations
            .sort_by(|left, right| left.start.cmp(&right.start).then(left.end.cmp(&right.end)));

        let mut filtered = Vec::new();
        let mut ignored_operations = 0;
        let mut last_end = 0;
        for operation in operations {
            if operation.start < last_end {
                ignored_operations += 1;
                continue;
            }
            last_end = operation.end;
            filtered.push(operation);
        }

        let new_text = apply_operations(&text, &filtered);
        self.tab_mut().buffer.replace(&new_text);
        self.tab_mut().content = EditorContent::with_text(&new_text);
        self.sync_highlight_buffer();

        let new_indices = compute_new_indices(&cursor_slots, &filtered);
        let new_positions: Vec<Position> = cursor_positions
            .iter()
            .enumerate()
            .map(|(index, position)| {
                let new_index = new_indices
                    .get(&index)
                    .copied()
                    .unwrap_or_else(|| self.tab().buffer.index_from_position(*position));
                self.tab().buffer.position_from_index(new_index)
            })
            .collect();

        if let Some(primary) = new_positions.first() {
            self.move_cursor_to(primary.line, primary.column);
        }
        self.tab_mut().multi_cursors = new_positions.into_iter().skip(1).collect();
        let count = self.tab().multi_cursors.len() + 1;
        self.status_message = Some(if ignored_operations > 0 {
            t_fmt(
                "status_cursors_edit_merged",
                &[("count", &count), ("merged", &ignored_operations)],
            )
        } else {
            t_fmt("status_cursors_edit", &[("count", &count)])
        });
    }

    pub(super) fn record_undo_snapshot(&mut self) {
        if self.tab().suppress_undo_snapshot {
            return;
        }
        // Transaction grouping: skip snapshot if last edit was within 500ms
        let now = Instant::now();
        if let Some(last) = self.undo_group_timer
            && now.duration_since(last) < Duration::from_millis(500)
        {
            self.undo_group_timer = Some(now);
            return;
        }
        self.undo_group_timer = Some(now);
        self.tab_mut().buffer.record_snapshot();
    }

    pub(super) fn apply_undo(&mut self) {
        if self.tab_mut().buffer.undo() {
            let text = self.tab().buffer.text().to_string();
            self.apply_snapshot(&text);
        }
    }

    pub(super) fn apply_redo(&mut self) {
        if self.tab_mut().buffer.redo() {
            let text = self.tab().buffer.text().to_string();
            self.apply_snapshot(&text);
        }
    }

    fn apply_snapshot(&mut self, text: &str) {
        self.tab_mut().suppress_undo_snapshot = true;
        self.tab_mut().content = EditorContent::with_text(text);
        self.tab_mut().buffer.replace(text);
        self.text_changed(true);
        self.tab_mut().suppress_undo_snapshot = false;
    }

    /// Replaces the whole text of the active tab with `text`.
    pub(super) fn replace_text_of_tab(&mut self, text: &str) {
        self.tab_mut().buffer.replace(text);
        self.tab_mut().content = EditorContent::with_text(text);
    }

    /// Force the iced text_editor widget to re-sync its internal scroll with
    /// the current cursor position: moving to the start resets the widget's
    /// scroll, and moving back to the cursor scrolls it into view again.
    pub(super) fn resync_widget_scroll(&mut self) {
        let cursor = self.tab().content.cursor();
        let content = &mut self.tab_mut().content;
        content.perform(EditorAction::Move(Motion::DocumentStart));
        content.move_to(cursor);
    }

    /// Ensure scroll_offset keeps the cursor line visible, mirroring the
    /// auto-scroll behaviour of the iced text_editor widget.
    pub(super) fn clamp_scroll_to_cursor(&mut self) {
        let (cursor_line, _) = self.cursor_position();
        let visible = self.viewport_height.max(1) as i32;
        let max_offset = (self.tab().buffer.line_count().max(1) as i32 - 1).max(0);
        let tab = self.tab_mut();
        let cursor = cursor_line as i32;
        if cursor < tab.scroll_offset {
            tab.scroll_offset = cursor;
        }
        if cursor >= tab.scroll_offset + visible {
            tab.scroll_offset = cursor - visible + 1;
        }
        tab.scroll_offset = tab.scroll_offset.clamp(0, max_offset);
    }

    /// The cursor's line and its column in bytes, as the editor counts them.
    pub(super) fn cursor_position(&self) -> (usize, usize) {
        let position = self.tab().content.cursor().position;
        (position.line, position.column)
    }

    /// Moves the cursor to `line` and `column`, a column counted in
    /// characters, both clamped to the text.
    pub(super) fn jump_to_position(&mut self, line: usize, column: usize) {
        let line = line.min(self.tab().buffer.line_count().saturating_sub(1));
        let column = column.min(
            self.tab()
                .buffer
                .line(line)
                .map_or(0, |text| text.chars().count()),
        );
        self.move_cursor_to(line, column);
        self.clamp_scroll_to_cursor();
    }

    /// Moves the cursor to `line` and `column`, a column counted in
    /// characters.
    pub(super) fn move_cursor_to(&mut self, line: usize, column: usize) {
        // The editor counts columns in bytes.
        let byte_column = self.tab().buffer.line(line).map_or(0, |text| {
            text.char_indices()
                .nth(column)
                .map_or(text.trim_end_matches(['\n', '\r']).len(), |(index, _)| {
                    index
                })
        });
        self.tab_mut().content.move_to(Cursor {
            position: EditorPosition {
                line,
                column: byte_column,
            },
            selection: None,
        });
        self.refresh_viewport_cache();
    }

    pub(super) fn sync_highlight_buffer(&mut self) {
        let text: Arc<str> = self.tab().content.text().into();
        self.tab_mut().highlight_settings.buffer_text = text;
    }

    pub(super) fn refresh_viewport_cache(&mut self) {
        let (line, _) = self.cursor_position();
        let start_line = line.saturating_sub(self.viewport_height / 2);
        let started = Instant::now();
        let visible = self.viewport_height;
        let tab = self.tab_mut();
        tab.viewport_cache.update(&tab.buffer, start_line, visible);
        self.performance.last_viewport_refresh = Some(started.elapsed());
    }

    pub(super) fn refresh_diagnostics(&mut self) {
        let found = diagnostics::analyze(&self.tab().buffer);
        self.tab_mut().diagnostics = found;
    }

    /// Lists the completions for the word before the cursor and opens the
    /// panel when there are any.
    pub(super) fn request_completions(&mut self) {
        let (line, column) = self.cursor_position();
        let line_text = self.tab().buffer.line(line).unwrap_or("").to_string();
        let column = line_text
            .get(..column)
            .map_or(line_text.chars().count(), |before| before.chars().count());
        let prefix = completion::extract_prefix(&line_text, column);
        let buffer_text = self.tab().buffer.text().to_string();
        let items = completion::build_items_with_buffer(&prefix, Some(&buffer_text));
        let count = items.len();
        let tab = self.tab_mut();
        tab.completion_prefix = prefix;
        tab.completion_items = items;
        self.completion_panel_open = count > 0;
        self.status_message = Some(if count == 0 {
            t("status_completions_none").to_string()
        } else {
            t_fmt("status_completions_count", &[("count", &count)])
        });
    }

    pub(super) fn close_completions(&mut self) {
        self.completion_panel_open = false;
        self.tab_mut().completion_items.clear();
    }

    pub(super) fn apply_completion(&mut self, index: usize) {
        let Some(item) = self.tab().completion_items.get(index).cloned() else {
            return;
        };
        let remainder = item
            .label
            .strip_prefix(&self.tab().completion_prefix)
            .unwrap_or(&item.label)
            .to_string();
        if !remainder.is_empty() {
            self.insert(EditorEdit::Paste(Arc::new(remainder)));
        }
        self.close_completions();
    }
}

#[derive(Debug, Clone)]
struct CursorSlot {
    id: usize,
    index: usize,
}

#[derive(Debug, Clone)]
struct EditOperation {
    id: usize,
    start: usize,
    end: usize,
    insert: String,
    base: usize,
}

impl EditOperation {
    fn from_edit(slot: &CursorSlot, edit: &EditorEdit, text: &str) -> Option<Self> {
        let insert_at = |insert: String| Self {
            id: slot.id,
            start: slot.index,
            end: slot.index,
            base: slot.index + insert.len(),
            insert,
        };
        match edit {
            EditorEdit::Insert(ch) => Some(insert_at(ch.to_string())),
            EditorEdit::Paste(contents) => Some(insert_at(contents.as_str().to_string())),
            EditorEdit::Enter => Some(insert_at("\n".to_string())),
            // One indent at each cursor; unindenting several lines at once
            // is left to a single cursor.
            EditorEdit::Indent => Some(insert_at("    ".to_string())),
            EditorEdit::Unindent => None,
            EditorEdit::Backspace => {
                let start = prev_char_boundary(text, slot.index)?;
                Some(Self {
                    id: slot.id,
                    start,
                    end: slot.index,
                    base: start,
                    insert: String::new(),
                })
            }
            EditorEdit::Delete => {
                let end = next_char_boundary(text, slot.index)?;
                Some(Self {
                    id: slot.id,
                    start: slot.index,
                    end,
                    base: slot.index,
                    insert: String::new(),
                })
            }
        }
    }

    fn delta(&self) -> isize {
        self.insert.len() as isize - (self.end - self.start) as isize
    }
}

fn apply_operations(text: &str, operations: &[EditOperation]) -> String {
    let mut output = String::with_capacity(text.len().saturating_add(operations.len() * 2));
    let mut cursor = 0;
    for operation in operations {
        if operation.start > text.len() || operation.end > text.len() || operation.start < cursor {
            continue;
        }
        output.push_str(&text[cursor..operation.start]);
        output.push_str(&operation.insert);
        cursor = operation.end;
    }
    output.push_str(&text[cursor..]);
    output
}

fn compute_new_indices(
    cursor_slots: &[CursorSlot],
    operations: &[EditOperation],
) -> HashMap<usize, usize> {
    let mut new_indices = HashMap::new();
    for slot in cursor_slots {
        let base = operations
            .iter()
            .find(|op| op.id == slot.id)
            .map_or(slot.index, |op| op.base);
        let delta: isize = operations
            .iter()
            .filter(|op| op.start < slot.index)
            .map(EditOperation::delta)
            .sum();
        new_indices.insert(slot.id, (base as isize + delta).max(0) as usize);
    }
    new_indices
}

fn prev_char_boundary(text: &str, index: usize) -> Option<usize> {
    if index == 0 {
        return None;
    }
    text[..index].char_indices().last().map(|(i, _)| i)
}

fn next_char_boundary(text: &str, index: usize) -> Option<usize> {
    if index >= text.len() {
        return None;
    }
    let next = text[index..]
        .chars()
        .next()
        .map(|ch| index + ch.len_utf8())?;
    Some(next.min(text.len()))
}
