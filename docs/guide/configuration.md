# Configuration

Roxanne reads its settings from TOML files. Most of them are also on the
Preferences page, which writes them back for you.

## Where the files are

Your settings live in `Colony/Roxanne/` under the platform's config directory:

| Platform | Folder |
|---|---|
| Linux | `~/.config/Colony/Roxanne/` (or `$XDG_CONFIG_HOME/Colony/Roxanne/`) |
| Windows | `%LOCALAPPDATA%\Colony\Roxanne\` |
| macOS | `~/Library/Application Support/Colony/Roxanne/` |

In that folder:

- `config.toml`: your configuration.
- `profiles/<name>.toml`: profiles, which `profile = "<name>"` in `config.toml`
  loads on top of it.

A project can carry its own `.roxanne.toml`, found in the working directory or
any of its parents.

Preferences → About shows the folder on your machine.

### Moving from `~/.config/roxanne`

Roxanne used to keep its settings in `~/.config/roxanne/`. On the first start
with the new folder, when that folder has no `config.toml` yet, Roxanne copies
`config.toml` and `profiles/` from the old one and says so in the status bar.
It then writes `.migrated-from-legacy-config` in the new folder, so the copy
happens once. The old folder is left exactly as it was: delete it yourself once
everything looks right. If the copy fails, the status bar says why, nothing is
marked as done, and the next start tries again.

## Order

Each layer overrides the one before, key by key:

1. Built-in defaults.
2. Your `config.toml`.
3. The profile it selects, if any.
4. The project's `.roxanne.toml`, then the profile it selects.

A file sets only the keys it names. A project's `[editor]` with `tab_size = 2`
changes the tab size and keeps your line numbers, word wrap and minimap.

A project file never changes your theme, your other preferences or the
interface language, and it never loads code: `[preferences]`, `[theme]` and
`language` in a `.roxanne.toml` are ignored with a warning, and so are
`plugins.dynamic` and `[lsp]`, which would let opening a folder run a program.

## The Preferences page

Click **Roxanne** in the top-left corner; click it again, or Close, to go back
to the editor. The page has the three categories every Colony program starts
with (General, Appearance, Accessibility), then Editor, Language server and
Plugins, and About last.

There is no Save button. A change applies at once, and only the key that
changed is written: into your profile when the profile sets that key, else into
your `config.toml`. A key the project sets is never written into your files:
its row in Preferences names the project file, the change lasts until the
settings reload, and the status bar says it was not saved. Roxanne edits a file
rather than rewriting it, so your comments, the order of your keys and the keys
it does not know all stay. A file that does not parse is left alone: the change
still applies until you quit, and the status bar says it could not be saved.

## Live reload

**Tools → Reload Config** reads the files again and applies what changed.

Roxanne also reloads on its own when `config.toml`, any file in `profiles/` or
the project's `.roxanne.toml` changes on disk, is removed or is moved away,
including a file created after Roxanne started. It watches the directories that
hold these files, so an editor that saves by renaming a new file over the old
one keeps triggering reloads. A missing `Colony/Roxanne/` or `profiles/`
directory is picked up once it is created, as long as the directory above it
exists, and so is one that is removed or moved away and then restored. With no
project config at startup, a new `.roxanne.toml` is looked for in the working
directory, unless that is the home folder or a filesystem root: there, use
**Tools → Reload Config**. A reload that finds the settings already in effect,
such as the one after the Preferences page saved a change, does nothing.

Open files are watched the same way. When another program changes one, a bar
above the editor offers to reload it. Roxanne's own saves never ask.

## Example

```toml
config_version = 1
language = "fr"
keymap_profile = "vim"

[preferences]
selected_theme = "gruvbox"
selected_variant = "dark"
selected_accent = "violet"
auto_accent = false
font_size = "default"
text_size_a11y = "large"
high_contrast = false
dyslexia_font = false
reduce_motion = false
animations = true

[editor]
line_numbers = true
word_wrap = false
minimap = false

[lsp]
enabled = true
rust_analyzer = "rust-analyzer"

[keymap]
save = "cmd+s"
find_next = "f3"

[keymap.insert]
completion = "ctrl+space"

[keymap.normal]
enter_insert_mode = "i"

[keymap_profiles.vim]
enter_insert_mode = "i"
enter_normal_mode = "escape"

[plugins]
enabled = ["word_count", "line_count"]
dynamic = ["/opt/roxanne/plugins/libsample_plugin.so"]
```

## Keys

### Top level

- `config_version`: the schema version, currently `1`. A file without it, or
  with an older one, is migrated in memory when it loads.
- `profile`: the profile to load from `profiles/<profile>.toml`.
- `language`: `"en"` or `"fr"`. Without it, Roxanne follows the system
  language (`LANGUAGE`, `LC_ALL`, `LC_MESSAGES` or `LANG` on Linux, the user's
  language on Windows and macOS), and falls back to English.
- `keymap_profile`: the profile from `[keymap_profiles]` to use.

### `[preferences]`

Your own files only. The Preferences page writes this table; the names match
what every Colony program stores.

- `selected_theme`, `selected_variant`: a Colony theme family and one of its
  variants, as Preferences → Appearance → Theme lists them. An unknown name
  falls back to Gruvbox dark.
- `selected_accent`: one of `red`, `orange`, `yellow`, `green`, `blue`,
  `indigo`, `violet`, `amber`. Without it, the theme's own accent.
- `auto_accent`: `true` hands the accent back to the theme.
- `font_size`: `"small"`, `"default"` or `"large"`.
- `text_size_a11y`: `"small"`, `"default"`, `"large"` or `"xlarge"`. The two
  sizes multiply.
- `high_contrast`, `dyslexia_font`, `reduce_motion`, `animations`: `true` or
  `false`. Code stays in JetBrainsMono Nerd Font with the dyslexia font on,
  because OpenDyslexic is not monospaced.

A value Roxanne does not understand falls back to its default; the rest of the
file still loads.

Before the Colony themes, Roxanne had a `[theme]` table. In your own file,
`name = "light"` is still read as the light variant of the default theme; its
colour overrides are no longer used, and loading them gives a warning.

### `[editor]`

- `line_numbers`: show line numbers. They are hidden while `word_wrap` is on,
  since a wrapped line takes several rows.
- `word_wrap`: long lines continue on the next row.
- `minimap`: show an outline of the file beside the text.
- `tab_size`, `use_spaces`, `line_ending`: read, not applied yet.

### `[lsp]`

Your own files only.

- `enabled`: `false` stops rust-analyzer and turns the LSP actions off.
- `rust_analyzer`: the program to run, a name looked up in `PATH` or a path.

The language server support is experimental and covers Rust only. **Tools →
Hover Information** and **Tools → Go to Definition** work on a saved `.rs`
file: Roxanne starts rust-analyzer on the first of them, in the nearest folder
above the file that holds a `Cargo.lock`, else a `Cargo.toml`, else the file's
own folder. Asking about a file in another workspace replaces the server. The
request runs in the background and its answer shows in the status bar, or opens
the definition; a server that does not answer in time gives an error there
instead of freezing the editor.

Roxanne asks rust-analyzer not to run build scripts, proc macros or
`cargo check`, since all three run code from the project. It still runs
`cargo metadata`, and a project's own files can change what it runs:
`rust-analyzer.toml`, Cargo configuration, and `rust-toolchain.toml`, which can
point to a toolchain stored inside the project. Use the LSP actions only on code
you trust.

### `[keymap]`

Keys: `save`, `open`, `find`, `find_next`, `find_previous`, `select_all`,
`copy`, `cut`, `paste`, `undo`, `redo`, `completion`, `completion_close`,
`enter_insert_mode`, `enter_normal_mode`.

`[keymap.insert]` and `[keymap.normal]` override them for one mode. An action
that does not belong to a mode is ignored there, with a warning.

A shortcut is modifiers and a key joined by `+`: `cmd`, `ctrl`, `shift`, `alt`,
then a character or a named key (`space`, `escape`, `enter`, `tab`,
`backspace`, `delete`, `home`, `end`, `pageup`, `pagedown`, `insert`, the
arrows `left`, `right`, `up`, `down`, and `f1` to `f12`). Off macOS, `cmd` and
`ctrl` are the same key. The menus show the shortcut each action has in the
keymap in use.

`[keymap_profiles.<name>]` defines a reusable set with the same keys, picked by
`keymap_profile` or in Preferences → Editor → Shortcuts.

The old `[keybindings]` section is read as `[keymap]`; keys already in
`[keymap]` win.

### `[plugins]`

- `enabled`: the built-in plugins to run: `word_count`, `line_count`,
  `character_count`, `byte_count`, `longest_line`. Preferences → Plugins turns
  them on and off.
- `dynamic`: native plugins (`.so`, `.dylib`, `.dll`), by absolute path, and
  only in your own files (see [Order](#order)): a relative path would resolve
  against the working directory, so it is refused. See [plugins.md](plugins.md).
