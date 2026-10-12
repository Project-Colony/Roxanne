<div align="center">

# Roxanne

**A lightweight code editor for developers who want a fast, keyboard-first editor without an IDE around it.**

</div>

[![License: GPL-3.0-or-later](https://img.shields.io/badge/License-GPL--3.0--or--later-blue.svg)](LICENSE)
[![Colony app](https://img.shields.io/badge/Colony-development-purple)](https://github.com/Project-Colony/Colony)
[![Platforms](https://img.shields.io/badge/platforms-linux%20%7C%20windows%20%7C%20macOS-lightgrey)](#installation)

Roxanne is a native desktop code editor written in Rust with [iced](https://iced.rs).
An IDE wants a project before it opens a file and runs that project's code to
understand it, while a plain text editor knows nothing about code. Roxanne opens
any file without a project, highlights eight languages with tree-sitter, and
asks rust-analyzer about Rust code only when you use an LSP action.

> **Status:** early development, not ready for everyday use. CI builds and
> tests Roxanne on Linux at every change and checks it against Rust 1.88; the
> release workflow builds the Linux, Windows and macOS binaries, which have not
> seen daily use yet. Language server support is experimental and covers Rust
> only, through rust-analyzer: hover and go to definition.

## Why Roxanne

- **No project needed.** Type a path and open the file. A `.roxanne.toml` in a
  folder adds project settings, but nothing requires one.
- **Opening a folder runs nothing.** A project's settings can never load a
  plugin or start a program. rust-analyzer starts only when you ask for an LSP
  action, with build scripts and proc macros turned off.
- **Saves you can trust.** Every save replaces the file in one rename, keeps its
  permissions and symlinks, and reloading never discards unsaved edits.
- **Keyboard first.** Menus, a command palette and shortcuts you can rebind,
  with keymap profiles and an optional vim-like normal mode.

## What it does

- Tabs, undo and redo, and multiple cursors.
- Syntax highlighting with tree-sitter for Rust, C, Go, JavaScript, JSON,
  Markdown, Python and TOML.
- Find and replace in the open file, with regular expressions and capture
  groups, and find in files across the workspace.
- A file tree, a command palette, go to line, a minimap, word wrap and a
  diagnostics panel for unbalanced brackets and unterminated strings.
- Experimental Language Server Protocol support for Rust: hover and go to
  definition through `rust-analyzer` from your `PATH`, started for the Cargo
  workspace of the file you ask about and answering in the background.
- Preferences, opened by clicking **Roxanne** in the top-left corner: the
  Colony themes and accents, text size, high contrast, the OpenDyslexic font,
  reduced motion, English or French, and the editor's own options. A change
  applies at once and is saved in your `config.toml`.
- TOML configuration with keymap profiles and live reload, and built-in and
  native plugins for the status bar.
- Safe saving: the text goes to a temporary file next to the real one, which
  replaces it in a single rename. When another program changes an open file,
  Roxanne asks before reloading it.

## Installation

### Via Colony (recommended)

Search for **Roxanne** in [Colony](https://github.com/Project-Colony/Colony) and
install it. Updates arrive through the launcher.

### Direct binary download

Grab the asset for your platform from the
[latest release](https://github.com/Project-Colony/Roxanne/releases/latest):

| Platform | Asset |
|---|---|
| Linux | `roxanne-linux` |
| Windows | `roxanne-windows.exe` |
| macOS (Apple Silicon) | `roxanne-macos` |
| macOS (Intel) | `roxanne-macos-x86` |

```bash
chmod +x roxanne-linux && ./roxanne-linux
```

`roxanne --version` prints the installed version.

### Build from source

```bash
git clone https://github.com/Project-Colony/Roxanne.git
cd Roxanne
cargo build --release
./target/release/roxanne
```

Requires Rust 1.88 or newer, from [rustup](https://rustup.rs).

## Documentation

Full documentation is in [docs/](docs/). Start at
[docs/README.md](docs/README.md).

## Code signing policy

Every release asset is signed by the Project Colony organisation in CI, never on
a developer machine. Next to each asset on the release page:

| File | What it is |
|---|---|
| `<asset>.sig` | an ed25519 signature over the asset, made with the organisation's release key |
| `<asset>.meta` | three lines binding the asset to its file name, its sha256 and the release version |
| `<asset>.meta.sig` | an ed25519 signature over the `.meta` |

The private key is an organisation secret, used only by the shared
[sign-and-publish workflow](https://github.com/Project-Colony/Project-Colony-Resources/blob/main/.github/workflows/sign-and-publish.yml)
in a job that builds nothing; the jobs that compile Roxanne never see it.
Colony checks all three files before it installs or updates Roxanne, and
refuses a release older than the one installed. To check a download yourself
with OpenSSL 3 (the same commands work for every asset):

```bash
cat > colony-release.pub <<'KEY'
-----BEGIN PUBLIC KEY-----
MCowBQYDK2VwAyEARNjg3Nn8H6/aBg1unwGjkUTcrdTxERNefVaqU8cFu0s=
-----END PUBLIC KEY-----
KEY
a=roxanne-linux
openssl pkeyutl -verify -pubin -inkey colony-release.pub -rawin -in "$a" -sigfile "$a.sig"
openssl pkeyutl -verify -pubin -inkey colony-release.pub -rawin -in "$a.meta" -sigfile "$a.meta.sig"
cat "$a.meta"     # version=<tag>, asset=<file name>, sha256=<digest>
sha256sum "$a"    # the digest must equal the sha256 line
```

How releases are built, signed and published:
[design/releases.md](https://github.com/Project-Colony/Project-Colony-Resources/blob/main/design/releases.md#5-signing).

## Privacy

Roxanne sends no telemetry, no analytics and no crash reports, and opens no
network connection of its own.

| Data | Stored or sent | Where, and why |
|---|---|---|
| Settings | stored | `config.toml` and the profiles in `profiles/`, in `Colony/Roxanne/` under your config directory (`~/.config/Colony/Roxanne/` on Linux). Roxanne reads them, and the Preferences page writes the settings you change into `config.toml`. |
| Older settings | read, copied | `config.toml` and `profiles/` from `~/.config/roxanne/`, copied once into `Colony/Roxanne/` when that has no `config.toml` yet. A `.migrated-from-legacy-config` file there marks the copy as done. The old folder is never changed. |
| Project settings | read | the first `.roxanne.toml` found in the working directory or one of its parents |
| Your files | stored | where you save them. A save writes a temporary file next to the file and renames it over the file, which keeps its permissions, and a symlink stays a link. |
| Text of the open file | sent to rust-analyzer | only when you use an LSP action on a saved Rust file. rust-analyzer is started from your `PATH`, or the program set in your own `config.toml`, and runs on this machine with build scripts and proc macros turned off. It runs `cargo metadata`, so Cargo or rustup can download the project's dependencies or toolchain. |
| Text and name of the open file | sent to native plugins | only the plugins listed in your own `config.toml` or a profile it selects. They run with your rights, and Roxanne does not limit what they do with it. |

This program will not transfer any information to other networked systems
unless specifically requested by the user or the person installing or
operating it.

## License

Roxanne is licensed under the [GNU General Public License v3.0 or later](LICENSE).

Roxanne embeds the Colony fonts through [colony-ui](https://crates.io/crates/colony-ui):
JetBrainsMono Nerd Font, OpenDyslexic and Font Awesome 6 Free, each under the
SIL Open Font License 1.1.
