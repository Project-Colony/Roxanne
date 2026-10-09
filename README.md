# Roxanne

A lightweight, extensible code editor built with Rust and [iced](https://iced.rs).

> **Status:** early development. There is no release yet, so you have to build it from source. It is not ready for everyday use.

## What it does

- Rope-based text buffer with undo and redo, tabs and multiple cursors.
- Syntax highlighting with tree-sitter for Rust, C, Go, JavaScript, JSON, Markdown, Python and TOML.
- File tree and workspace-wide search and replace.
- Experimental Language Server Protocol support, for Rust only: hover and go to definition through `rust-analyzer` from your `PATH`. It starts on the first LSP action, one server per Cargo workspace, and answers in the background without freezing the editor. Roxanne asks it not to run build scripts, proc macros or `cargo check`. It still runs `cargo metadata`, and a project's own files can change what it runs: `rust-analyzer.toml`, Cargo configuration, and `rust-toolchain.toml`, which can point to a toolchain stored inside the project. Use the LSP actions only on code you trust.
- TOML configuration in `~/.config/roxanne/` with profiles and live reload.
- Built-in plugins and native plugins loaded from shared libraries (see `plugins/roxanne_sample`).
- Safe saving: the text goes to a temporary file next to the real one, which it replaces in a single rename, so a file is never left half written. The file keeps its permissions and a symlink stays a link. A new tab asks for a file name before its first save, Save As asks before replacing an existing file, and reloading a file never discards unsaved edits.

Native plugins run with your rights, so Roxanne loads only the ones listed in your own `~/.config/roxanne/config.toml` (or a profile it selects), and only by absolute path. A project's `.roxanne.toml`, found in the working directory or any parent, can set the theme, keymap, editor options and built-in plugins, but its `plugins.dynamic` is ignored: opening a folder never loads code from it.

## Build from source

You need a recent stable Rust toolchain (edition 2024, Rust 1.88 or newer) from [rustup](https://rustup.rs).

```bash
git clone https://github.com/Project-Colony/Roxanne.git
cd Roxanne
cargo build --release
./target/release/roxanne
```

## License

Roxanne is licensed under the [GNU General Public License v3.0 or later](LICENSE).

`vendor/iced_widget` is a patched copy of `iced_widget` 0.12.3 by the iced contributors, under the MIT license in [`vendor/iced_widget/LICENSE`](vendor/iced_widget/LICENSE). The changes are described in [`vendor/iced_widget/PATCH.md`](vendor/iced_widget/PATCH.md).
