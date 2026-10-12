# Using Roxanne

## The window

From top to bottom:

- **The menu bar.** The **Roxanne** button on the left opens Preferences, then
  come File, Edit, Selection, View, Go, Tools, Project and Help. Each entry
  shows the shortcut it has in the keymap in use.
- **The tab bar.** One tab per open file. A dot after the name marks unsaved
  changes; the `+` button opens a new tab.
- **Panels**, when they are open: search and replace, go to line, diagnostics,
  completions, Save As, and the bar that offers to reload a file another
  program changed.
- **The editor**, with the file tree on its left and the minimap on its right
  when you turn them on.
- **The status bar**: a quick search field, the file name field, the cursor
  position, the keymap mode, the plugin counters and the last message.

## Opening and saving files

Roxanne starts with an empty tab and has no file dialog. To open a file, type
its path in the file name field of the status bar and choose **File → Open
File** (`Ctrl+O`, `Cmd+O` on macOS), or click it in the file tree (**View →
File Tree**, `Ctrl+B`). A file that is already open gets its tab back instead
of a second copy.

Files larger than 5 MiB are refused. Bytes that are not valid UTF-8 are
replaced when the file loads, and the first save after that asks you to save
again before it writes the replaced text.

**File → Save** (`Ctrl+S`) writes the file in place. A new tab, or one whose
file name you edited, opens Save As instead, and Save As asks before it replaces
an existing file. Every save goes to a temporary file next to the real one,
which then replaces it in a single rename: the file is never left half written,
keeps its permissions, and a symlink stays a link.

When another program changes or replaces an open file, a bar above the editor
offers to reload it. A tab with unsaved edits keeps them: Reload then leaves
the tab as it is and the status bar says so.

## Editing

The editor has undo and redo (`Ctrl+Z`, `Ctrl+Shift+Z` or `Ctrl+Y`), which
group edits made less than half a second apart, and the usual cut, copy and
paste.

**Selection → Add Cursor to Next Match** and **Add Cursors to All Matches**
place extra cursors on the occurrences of the search text; typing, deleting and
pasting then happen at every cursor. **Remove Extra Cursors** goes back to one.

`Ctrl+Space` lists completions for the word before the cursor: Rust keywords
and types, and the words already in the file. `Escape` closes the list.

**Go → Go to Line** takes a line, or `line:column`.

### Normal mode

The default keymap starts in insert mode, where keys type text. `Ctrl+[`
switches to an experimental vim-like normal mode: `h`, `j`, `k`, `l` move, `w`
and `b` move by word, `0` goes to the start of the line, `g` and `G` to the top
and bottom of the file, `x` deletes a character, `d` deletes the line, and `i`,
`a`, `o` and `O` go back to insert mode. The status bar shows the mode.

Shortcuts only see the keys the text area does not use itself. While the text
area has the keyboard focus, letters type text in either mode: press `Escape`
to leave it before using normal mode, and click in the text to type again.

All of these shortcuts can be changed; see
[configuration.md](configuration.md#keymap).

## Searching

**Edit → Find** (`Ctrl+F`) puts the cursor in the search field of the status
bar, which searches the open file as you type; `F3` and `Shift+F3` go to the
next and previous match. **Edit → Search Panel** opens the full panel, with
options for case sensitivity and regular expressions and the replace field.

**Replace** changes the current match and **Replace All** every match, in the
open file only. With regular expressions on, the replacement can use capture
groups such as `$1`. Both can be undone.

Set the scope to the workspace, or choose **Project → Find in Files**, to search
every file under the workspace: the folder of the nearest `.roxanne.toml` above
the open file, or the file's own folder. That search runs in the background and
lists up to 500 matches; click one to open it. It does not replace. Hidden files
are skipped unless you include them, and so are `.git`, `target`,
`node_modules`, `dist`, `build` and `out`, common binary formats, files larger
than 5 MiB and files that are not valid UTF-8.

## Code intelligence

- **Highlighting** with tree-sitter for Rust, C, Go, JavaScript, JSON,
  Markdown, Python and TOML, chosen by file extension.
- **The diagnostics panel** (**Edit → Diagnostics Panel**) lists brackets that
  do not match and strings or character literals that never end, as you type.
- **rust-analyzer**, for Rust files only: **Tools → Hover Information** and
  **Tools → Go to Definition**. See
  [configuration.md](configuration.md#lsp) for how the server starts and what it
  may run.

## Other tools

- **View → Command Palette** (`Ctrl+Shift+P`) lists every action; type to
  filter, click to run.
- **View → Line Numbers**, **Word Wrap** and **Minimap** turn these on and
  off, and save the choice like Preferences → Editor does.
- **Tools → Reload Config** reads the settings files again.
- **Tools → Performance Report** shows in the status bar how long the last
  start, open, save, search and viewport refresh took.

## Preferences

Click **Roxanne** in the top-left corner, or choose **Help → Preferences**. The
page covers the Colony theme and accent, text size, high contrast, the
OpenDyslexic font, reduced motion, the interface language, the editor display,
the keymap profile, rust-analyzer and the plugins. A change applies at once and
is written into your `config.toml`. See [configuration.md](configuration.md).
