# Editor core

The text model lives in `src/editor/`: `buffer.rs` holds the text and its
history, `highlight.rs` and `syntax.rs` colour it, and `viewport.rs` keeps the
lines around the cursor. This page covers the buffer and how the window edits
it; [rendering.md](rendering.md) covers the rest.

## Two copies of the text

Each tab holds its text twice:

- `content`, an iced `text_editor::Content`, is what the editor widget draws and
  edits. It owns the cursor and the selection.
- `buffer`, a `TextBuffer`, is what the rest of Roxanne reads: undo and redo,
  search and replace, diagnostics, completions, the LSP and the plugins.

After every edit the window copies the widget's text into the buffer
(`TextBuffer::replace`, which does nothing when the text is unchanged). The
other way round, undo, redo, replace and reloading a file write the buffer
first and then rebuild `content` from it.

## `TextBuffer`

- The text is a `ropey::Rope`, so inserting or deleting in the middle of a large
  file does not move the rest of it.
- `text()` returns the whole text as a `&str` from a `String` cache, rebuilt
  after every change. That copy is the main cost of an edit on a large file;
  the `text_buffer_repeated_edits` benchmark watches it.
- `revision()` counts changes. The viewport cache compares it to skip work when
  nothing changed.
- A `Position` is a line and a column, both from zero. Columns count
  characters, not bytes. The iced widget counts bytes, so the window converts
  at the boundary (`cursor_position`, `move_cursor_to`).
- `line_count()` counts a trailing newline as an extra, empty last line, as the
  editor shows it.

## Undo and redo

The history is a stack of rope snapshots. Cloning a `Rope` shares its nodes, so
a snapshot costs little memory however large the file is.

- Before an edit, the window calls `record_snapshot`, unless the previous edit
  was less than 500 ms earlier: a burst of typing is one undo step.
- A new snapshot clears the redo stack. Identical consecutive snapshots are
  stored once.
- The history keeps the last 200 steps per tab.
- Loading a file clears the history of its tab.

## Multiple cursors

The tab keeps the extra cursors as a list of `Position`s, beside the widget's
own cursor. While the list is not empty, an edit does not go to the widget:
`ui/editing.rs` turns it into one operation per cursor (insert, paste, enter,
backspace, delete, indent), sorts them by position, drops any that would
overlap an earlier one, and builds the new text in one pass. Each cursor then
moves by what the edits before it added or removed, and the widget is rebuilt
from the result.

Cursors are added on search matches (**Selection → Add Cursor to Next Match**,
**Add Cursors to All Matches**), so the search text decides where they go.
