# Architecture

Roxanne is one crate: a library in `src/lib.rs` that holds everything, and a
small `src/main.rs` that answers `--version` and opens the window. The window
follows iced's model: one state struct, one message enum, an `update` function
that changes the state, and a `view` function that draws it.

## Modules

| Module | What it owns |
|---|---|
| `ui` | the window: `RoxanneApp`, its `Message`s and their dispatch, split by area (`editing`, `files`, `tabs`, `menu`, `keys`, `search_panel`, `panels`, `file_tree`, `editor_view`, `status_bar`, `preferences`, `style`) |
| `editor` | the text model: `TextBuffer` on a rope with undo history, the tree-sitter highlighter, and the viewport cache. See [editor-core.md](editor-core.md) and [rendering.md](rendering.md) |
| `config` | loading the layered TOML settings (`mod.rs`), writing Preferences changes back (`store.rs`), and the one-time copy from `~/.config/roxanne` (`migrate.rs`) |
| `keymap` | parsing shortcuts, the default bindings, and matching key events to actions in insert or normal mode |
| `search` | matching in the open file, and the workspace search that walks the folder tree |
| `lsp` | the rust-analyzer client: one session per workspace root, on its own thread |
| `diagnostics` | the local checks for brackets, strings and character literals |
| `completion` | completion items from Rust keywords and the words in the file |
| `plugins` | the built-in counters and the loader for native plugins |
| `file_ops` | the size limit and the atomic write every save goes through |
| `watch` | the subscription that reports files changed by other programs |
| `i18n` | the English and French strings (`en.toml`, `fr.toml`), on top of the shared strings from colony-ui |

The look, the fonts, the Preferences skeleton and the theme and accent settings
come from [colony-ui](https://crates.io/crates/colony-ui), the Project Colony
design-system crate; `ui::style` only maps its palette onto Roxanne's widgets.

## How an edit flows

1. The focused text editor widget turns the keys it knows into editor
   actions, each reported as a `Message::Edit`. Keys no widget used reach the
   window as `Message::Key` and go through the keymap: a shortcut becomes a
   `KeyAction`, handled like the menu entry of the same name.
2. An editor action changes the tab's `text_editor::Content`, which holds the
   text, the cursor and the selection the widget draws. With several cursors,
   Roxanne applies the edit to the text itself instead.
3. The tab's `TextBuffer` takes the new text. Undo, redo, search, replace,
   diagnostics and the LSP read the buffer, not the widget.
4. `text_changed` then updates what depends on the text: the highlighter's
   copy, the search matches, the diagnostics, the plugins and the viewport
   cache.

## What runs outside the interface thread

The interface thread never waits on the disk, the file system or another
process:

- Opening and saving a file, opening a search result and the workspace search
  are `Task`s; their result comes back as a message.
- rust-analyzer runs on a thread owned by its `LspSession`. A request sends a
  job to that thread and gets a future back, so a slow or stuck server leaves
  the editor responsive.
- File watching is an iced `Subscription` over a `notify` watcher. It watches
  the directories that hold the files rather than the files, so a save that
  renames a new file over the old one keeps being seen. Roxanne records its own
  writes, so its saves never ask to reload the file.

Plugins are the exception: their callbacks run on the interface thread.

## Settings

`AppConfig::load` reads the built-in defaults, the user's `config.toml`, the
profile it selects, then the project's `.roxanne.toml` and its profile, each
layer over the one before, key by key: a table such as `[editor]` sets only the
keys it names. `AppConfig::sources` records which file set each key. A project
file is filtered before it is applied: it cannot set the preferences or the
language, and its `plugins.dynamic` and `[lsp]` are dropped, so opening a
folder never loads or runs code from it.

The Preferences page changes the setting in memory and applies it. Then
`store::save_changes` compares the settings before and after and writes only
the keys that changed, each into the user's file that sets it (the profile or
`config.toml`), else into `config.toml`, with `toml_edit`, which keeps
comments, key order and unknown keys. A key a project file sets is not written. A watched config file that changes on disk triggers a reload;
a reload that finds the settings already in effect does nothing.

The user's files live in `Colony/Roxanne/` under the platform's config
directory, as colony-ui's `paths` module resolves it.

## Decisions

- **A rope for the text.** `ropey` keeps edits in the middle of large files
  cheap and makes undo snapshots cheap to clone. The buffer also caches the
  whole text as a `String` for the callers that need a `&str`, which costs a
  copy per edit; the benchmarks keep that cost in check.
- **iced's text editor for drawing and cursor handling,** with a custom
  highlighter, instead of a hand-written text widget.
- **Saves through one rename.** A save never truncates the real file, so a
  crash or a full disk cannot leave it half written.
- **Never run the project's code by surprise.** Project settings cannot load
  plugins or choose the language server program, and rust-analyzer starts only
  on an LSP action, with build scripts and proc macros off.
