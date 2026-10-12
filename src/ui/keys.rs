//! Keyboard shortcuts: the keymap's actions, in insert and normal mode.

use super::{Message, RoxanneApp};
use crate::i18n::t;
use crate::keymap::{KeyAction, KeymapMode};
use iced::widget::text_editor::{Action as EditorAction, Edit as EditorEdit, Motion};
use iced::{Task, clipboard, keyboard};

impl RoxanneApp {
    /// A key press no widget used: runs the keymap's action for it, if any.
    /// Not while Preferences hides the editor: the action would change a file
    /// the user cannot see.
    pub(super) fn on_key(&mut self, event: keyboard::Event) -> Task<Message> {
        let keyboard::Event::KeyPressed { key, modifiers, .. } = event else {
            return Task::none();
        };
        if self.preferences.open {
            return Task::none();
        }
        let Some(action) = self.config.keymap.match_event(&key, modifiers, self.mode) else {
            return Task::none();
        };
        // The editor copies, cuts and pastes on its own shortcuts.
        if matches!(action, KeyAction::Copy | KeyAction::Cut | KeyAction::Paste)
            && modifiers.command()
            && let keyboard::Key::Character(value) = &key
            && matches!(value.to_ascii_lowercase().as_str(), "c" | "x" | "v")
        {
            return Task::none();
        }
        self.handle_key_action(action)
    }

    pub(super) fn handle_key_action(&mut self, action: KeyAction) -> Task<Message> {
        match action {
            KeyAction::Save => return self.save_file(),
            KeyAction::Open => return self.open_file(),
            KeyAction::Find => return self.focus_search(),
            KeyAction::FindNext => self.find_next_match(true),
            KeyAction::FindPrevious => self.find_next_match(false),
            KeyAction::SelectAll => self.select_all(),
            KeyAction::Copy => return self.copy(),
            KeyAction::Cut => return self.cut(),
            KeyAction::Paste => return clipboard::read().map(Message::PasteFromClipboard),
            KeyAction::Undo => self.apply_undo(),
            KeyAction::Redo => self.apply_redo(),
            KeyAction::Completion => self.request_completions(),
            KeyAction::CompletionClose => self.close_completions(),
            KeyAction::NewTab => self.new_tab(),
            KeyAction::CloseTab => self.close_tab(self.active_tab),
            KeyAction::NextTab => self.cycle_tab(true),
            KeyAction::PrevTab => self.cycle_tab(false),
            KeyAction::CommandPalette => self.toggle_command_palette(),
            KeyAction::ToggleFileTree => self.toggle_file_tree(),
            KeyAction::EnterInsertMode => self.enter_mode(KeymapMode::Insert, "status_mode_insert"),
            KeyAction::EnterNormalMode => self.enter_mode(KeymapMode::Normal, "status_mode_normal"),
            KeyAction::MoveLeft => self.move_cursor(Motion::Left),
            KeyAction::MoveDown => self.move_cursor(Motion::Down),
            KeyAction::MoveUp => self.move_cursor(Motion::Up),
            KeyAction::MoveRight => self.move_cursor(Motion::Right),
            KeyAction::MoveWordForward => self.move_cursor(Motion::WordRight),
            KeyAction::MoveWordBackward => self.move_cursor(Motion::WordLeft),
            KeyAction::MoveLineStart => self.move_cursor(Motion::Home),
            KeyAction::MoveLineEnd => self.move_cursor(Motion::End),
            KeyAction::MoveFileTop => self.jump_to_position(0, 0),
            KeyAction::MoveFileBottom => {
                let last_line = self.tab().buffer.line_count().saturating_sub(1);
                self.jump_to_position(last_line, 0);
            }
            KeyAction::DeleteChar => self.delete(&[]),
            KeyAction::DeleteLine => self.delete(&[
                EditorAction::Move(Motion::Home),
                EditorAction::Select(Motion::Down),
            ]),
            KeyAction::InsertAfter => {
                self.tab_mut()
                    .content
                    .perform(EditorAction::Move(Motion::Right));
                self.enter_mode(KeymapMode::Insert, "status_mode_insert_after");
            }
            KeyAction::InsertLineBelow => {
                self.open_line(&[EditorAction::Move(Motion::End)], &[]);
                self.enter_mode(KeymapMode::Insert, "status_mode_insert_below");
            }
            KeyAction::InsertLineAbove => {
                self.open_line(
                    &[EditorAction::Move(Motion::Home)],
                    &[EditorAction::Move(Motion::Up)],
                );
                self.enter_mode(KeymapMode::Insert, "status_mode_insert_above");
            }
        }
        Task::none()
    }

    fn enter_mode(&mut self, mode: KeymapMode, status_key: &'static str) {
        self.mode = mode;
        self.status_message = Some(t(status_key).to_string());
    }

    fn move_cursor(&mut self, motion: Motion) {
        self.tab_mut().content.perform(EditorAction::Move(motion));
        self.clamp_scroll_to_cursor();
        self.refresh_viewport_cache();
    }

    /// Deletes the character after the cursor, after running `select`.
    fn delete(&mut self, select: &[EditorAction]) {
        self.record_undo_snapshot();
        for action in select {
            self.tab_mut().content.perform(action.clone());
        }
        self.tab_mut()
            .content
            .perform(EditorAction::Edit(EditorEdit::Delete));
        let text = self.tab().content.text();
        self.tab_mut().buffer.replace(&text);
        self.resync_widget_scroll();
        self.clamp_scroll_to_cursor();
        self.text_changed(true);
    }

    /// Breaks a line after the `before` moves, then makes the `after` moves.
    fn open_line(&mut self, before: &[EditorAction], after: &[EditorAction]) {
        for action in before {
            self.tab_mut().content.perform(action.clone());
        }
        self.record_undo_snapshot();
        self.tab_mut()
            .content
            .perform(EditorAction::Edit(EditorEdit::Enter));
        for action in after {
            self.tab_mut().content.perform(action.clone());
        }
        let text = self.tab().content.text();
        self.tab_mut().buffer.replace(&text);
        self.text_changed(true);
    }

    /// Puts the cursor in the status bar's search field.
    pub(super) fn focus_search(&mut self) -> Task<Message> {
        iced::widget::operation::focus(super::search_panel::SEARCH_INPUT)
    }
}
