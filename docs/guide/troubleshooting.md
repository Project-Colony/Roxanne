# Troubleshooting

## The build fails

Roxanne needs Rust 1.88 or newer. Check the version and update the toolchain:

```bash
rustc --version
rustup update stable
```

## A file does not open

The status bar says why. Two limits apply to every file:

- files larger than 5 MiB are refused;
- bytes that are not valid UTF-8 are replaced when the file loads. The file
  opens, and the first save asks you to save again before it writes the
  replaced text, so nothing is changed by accident.

A path in the file name field is relative to the folder Roxanne was started
from. Type an absolute path when in doubt.

## A setting has no effect

- After loading the settings, the status bar counts the warnings: a file that
  does not parse, a missing profile, an unknown plugin or theme, a shortcut
  that does not parse. Choose **Tools → Reload Config** to load the files
  again and see the count.
- A project's `.roxanne.toml` cannot change your theme, the other preferences,
  the interface language, `plugins.dynamic` or `[lsp]`: those keys are ignored
  there, with a warning. Put them in your own `config.toml`.
- `tab_size`, `use_spaces` and `line_ending` are read but not applied yet.

Preferences → About shows where your `config.toml` is.

## The settings were not saved

The Preferences page writes into your `config.toml` and leaves a file that does
not parse alone. The change still applies until you quit, and the status bar
says it could not be saved. Fix the TOML error, then change the setting again.

## rust-analyzer does not answer

- The LSP actions work on saved `.rs` files only.
- rust-analyzer must be installed and on your `PATH`, for example with
  `rustup component add rust-analyzer`, or set `lsp.rust_analyzer` in your
  `config.toml` to its full path.
- The first request on a large workspace can take a while: rust-analyzer loads
  the workspace before it answers. A server that gives no answer in time
  reports an error in the status bar; ask again once it has loaded.
- Preferences → Language server, or `lsp.enabled = false`, turns it off.

## A native plugin does not load

The path in `plugins.dynamic` must be absolute and listed in your own
`config.toml`, not in a project's `.roxanne.toml`. The library must export
`roxanne_plugin_api_v1`; see [plugins.md](plugins.md).
