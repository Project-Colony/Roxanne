# Installation

Roxanne is one executable with no installer. Pick one of the three ways below.

## Via Colony

Search for **Roxanne** in [Colony](https://github.com/Project-Colony/Colony) and
install it. Colony checks the release signatures before it installs anything
and brings updates as they come out.

## Release binaries

Each release on the [Releases page](https://github.com/Project-Colony/Roxanne/releases)
ships one binary per platform:

| Platform | Asset |
|---|---|
| Linux x86_64 | `roxanne-linux` |
| Windows x86_64 | `roxanne-windows.exe` |
| macOS, Apple Silicon | `roxanne-macos` |
| macOS, Intel | `roxanne-macos-x86` |

Every asset comes with `<asset>.sig`, `<asset>.meta` and `<asset>.meta.sig`.
The README's [Code signing policy](../../README.md#code-signing-policy) says how
to check them with OpenSSL before you run the binary.

On Linux and macOS, make the file executable first:

```bash
chmod +x roxanne-linux
./roxanne-linux
```

`roxanne --version` prints the version and exits without opening a window.

## From source

You need Rust 1.88 or newer, installed with [rustup](https://rustup.rs).

```bash
git clone https://github.com/Project-Colony/Roxanne.git
cd Roxanne
cargo build --release
./target/release/roxanne
```

## Uninstalling

Delete the executable, or uninstall Roxanne from Colony. Your settings stay in
`Colony/Roxanne/` under your config directory (`~/.config/Colony/Roxanne/` on
Linux, `%LOCALAPPDATA%\Colony\Roxanne\` on Windows,
`~/Library/Application Support/Colony/Roxanne/` on macOS): delete that folder
too to remove them. Settings from before the move to that folder may still be
in `~/.config/roxanne/`, which Roxanne never deletes.
