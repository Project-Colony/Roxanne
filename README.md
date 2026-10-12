# Roxanne

A lightweight, extensible code editor built with Rust and [iced](https://iced.rs).

> **Status:** early development, not ready for everyday use. CI builds and tests it on Linux; release binaries are built for Linux, Windows and macOS.

## What it does

- Rope-based text buffer with undo and redo, tabs and multiple cursors.
- Syntax highlighting with tree-sitter for Rust, C, Go, JavaScript, JSON, Markdown, Python and TOML.
- File tree and workspace-wide search and replace.
- Experimental Language Server Protocol support, for Rust only: hover and go to definition through `rust-analyzer` from your `PATH`. It starts on the first LSP action, one server per Cargo workspace, and answers in the background without freezing the editor. Roxanne asks it not to run build scripts, proc macros or `cargo check`. It still runs `cargo metadata`, and a project's own files can change what it runs: `rust-analyzer.toml`, Cargo configuration, and `rust-toolchain.toml`, which can point to a toolchain stored inside the project. Use the LSP actions only on code you trust.
- Preferences, opened by clicking **Roxanne** in the top-left corner: the Colony themes and accents, text size, high contrast, the OpenDyslexic font, reduced motion, the interface language (English or French), and the editor's own options. A change applies at once and is saved in your `config.toml`; there is no Save button.
- TOML configuration in `Colony/Roxanne/` under your config directory (`~/.config/Colony/Roxanne/` on Linux), with keymap profiles and live reload. Settings from an older `~/.config/roxanne/` are copied there once, and the old folder is left as it was. See [Configuration](docs/configuration.md).
- Built-in plugins and native plugins loaded from shared libraries (see `plugins/roxanne_sample`).
- Safe saving: the text goes to a temporary file next to the real one, which it replaces in a single rename, so a file is never left half written. The file keeps its permissions and a symlink stays a link. A new tab asks for a file name before its first save, Save As asks before replacing an existing file, and reloading a file never discards unsaved edits.

Native plugins run with your rights, so Roxanne loads only the ones listed in your own `config.toml` (or a profile it selects), and only by absolute path. A project's `.roxanne.toml`, found in the working directory or any parent, can set the keymap, editor options and built-in plugins, but not your theme or language, and its `plugins.dynamic` and `[lsp]` are ignored: opening a folder never loads or runs code from it.

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

You need a stable Rust toolchain, Rust 1.88 or newer, from [rustup](https://rustup.rs).

```bash
git clone https://github.com/Project-Colony/Roxanne.git
cd Roxanne
cargo build --release
./target/release/roxanne
```

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
| Settings | stored | `config.toml` and the profiles in `profiles/`, in `Colony/Roxanne/` under this machine's config directory (`~/.config/Colony/Roxanne/` on Linux). The Preferences page writes your changes into `config.toml`. |
| Older settings | read once | `~/.config/roxanne/`, copied to `Colony/Roxanne/` on the first start that finds no `config.toml` there. Roxanne never changes or deletes the old folder. |
| Project settings | read | the first `.roxanne.toml` found in the working directory or one of its parents |
| Your files | stored | where you save them. A save writes a temporary file next to the file and renames it over the file, which keeps its permissions, and a symlink stays a link. |
| Text of the open file | sent to rust-analyzer | only when you use an LSP action on a saved Rust file. rust-analyzer is started from your `PATH`, or as the program set in Preferences, and runs on this machine, with build scripts and proc macros turned off. It runs `cargo metadata`, so Cargo or rustup can download the project's dependencies or toolchain. |
| Text and name of the open file | sent to native plugins | only the plugins listed in your own `config.toml` or a profile it selects. They run with your rights, and Roxanne does not limit what they do with it. |

This program will not transfer any information to other networked systems
unless specifically requested by the user or the person installing or
operating it.

## License

Roxanne is licensed under the [GNU General Public License v3.0 or later](LICENSE).

Roxanne embeds the Colony fonts through [colony-ui](https://crates.io/crates/colony-ui): JetBrainsMono Nerd Font, OpenDyslexic and Font Awesome 6 Free, each under the SIL Open Font License 1.1.
