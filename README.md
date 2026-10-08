# Roxanne

A lightweight, extensible code editor built with Rust and [iced](https://iced.rs).

> **Status:** early development. There is no release yet, so you have to build it from source. It is not ready for everyday use.

## What it does

- Rope-based text buffer with undo and redo, tabs and multiple cursors.
- Syntax highlighting with tree-sitter for Rust, C, Go, JavaScript, JSON, Markdown, Python and TOML.
- File tree and workspace-wide search and replace.
- Language Server Protocol client that starts the language servers you configure.
- TOML configuration in `~/.config/roxanne/` with profiles and live reload.
- Built-in plugins and native plugins loaded from shared libraries (see `plugins/roxanne_sample`).

## Build from source

You need a recent stable Rust toolchain (edition 2024, Rust 1.85 or newer) from [rustup](https://rustup.rs).

```bash
git clone https://github.com/Project-Colony/Roxanne.git
cd Roxanne
cargo build --release
./target/release/roxanne
```

## License

Roxanne is licensed under the [GNU General Public License v3.0 or later](LICENSE).

`vendor/iced_widget` is a patched copy of `iced_widget` 0.12.3 by the iced contributors, under the MIT license in [`vendor/iced_widget/LICENSE`](vendor/iced_widget/LICENSE). The changes are described in [`vendor/iced_widget/PATCH.md`](vendor/iced_widget/PATCH.md).
