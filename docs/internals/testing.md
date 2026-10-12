# Testing

## Running the checks

CI runs these on every push and pull request; run them before you open one:

```bash
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
```

`cargo test` also builds `examples/sample_plugin.rs`, and `tests/native_plugin.rs`
loads the library it produces, so a change to the plugin API or to plugin
loading that breaks the example fails the tests. That test needs the example
built: run it through `cargo test`, not `cargo test --test native_plugin` alone. A second CI job runs
`cargo check --all-targets` with Rust 1.90, the `rust-version` in `Cargo.toml`.

For the benchmarks and their limits, see [performance.md](performance.md).

## Where the tests are

**Unit tests** sit in a `mod tests` at the bottom of the module they test. The
interface has its own file, `src/ui/tests.rs`, which drives `RoxanneApp` through
its messages without opening a window. The ones worth knowing about:

- `editor/buffer.rs`: positions on large texts, insertion and deletion, undo
  and redo.
- `keymap.rs`: shortcut parsing and matching, `cmd` and `ctrl` included.
- `config/`: layering, the one-time copy from `~/.config/roxanne`, and
  writing Preferences changes without losing comments or touching a file that
  does not parse.
- `lsp.rs`: the client against a fake server on a thread, so the tests need no
  rust-analyzer.
- `watch.rs`: only the watched files are reported, a removed or recreated
  directory keeps being watched, and Roxanne's own saves are ignored where
  asked.
- `i18n/mod.rs`: English and French have the same keys and the same `{name}`
  placeholders.
- `file_ops.rs`: a failed save leaves the original file as it was.
- `plugins.rs`: relative native plugin paths are refused, and an unknown
  plugin name gives a warning.

**Integration tests** in `tests/` use only the public library API, or the
binary:

| File | Covers |
|---|---|
| `atomic_write.rs` | a save keeps the file mode and writes through a symlink |
| `cli.rs` | `--version` prints the version without opening a window |
| `config_loading.rs` | profiles, workspace overrides, editor defaults, refused native plugins in a project, the watched paths |
| `config_migration.rs` | old `[keybindings]` sections become `[keymap]` |
| `native_plugin.rs` | the sample plugin loads and its status reaches the status bar |

## Writing tests

- Tests that change `HOME`, `XDG_CONFIG_HOME` or the working directory hold a
  lock while they do, like `ENV_LOCK` in `tests/config_loading.rs`: these are
  process-wide and the harness runs tests in parallel.
- Use `tempfile` for anything on disk, never the real config directory.
- A bug fix comes with a test that fails without the fix.
- Nothing in the test suite needs a display, a network connection or
  rust-analyzer.
