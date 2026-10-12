# Plugins

Plugins add counters to the status bar. There are two kinds: built-in plugins,
which ship with Roxanne, and native plugins, shared libraries you build or
download yourself.

## Built-in plugins

| Name | Shows |
|---|---|
| `word_count` | the number of words |
| `line_count` | the number of lines |
| `character_count` | the number of characters |
| `byte_count` | the size in bytes |
| `longest_line` | the length of the longest line |

All five run by default. Turn them on and off in Preferences → Plugins, or list
the ones you want in `plugins.enabled`:

```toml
[plugins]
enabled = ["word_count", "line_count"]
```

A project's `.roxanne.toml` may set `plugins.enabled` too.

## Native plugins

A native plugin is a `.so`, `.dylib` or `.dll` that Roxanne loads at startup,
and again when the plugin settings change. It runs inside Roxanne with your
rights, so Roxanne loads one only when all of these hold:

- it is listed in `plugins.dynamic` in your own `config.toml`, or in a profile
  that file selects. `plugins.dynamic` in a project's `.roxanne.toml` is
  ignored, so opening a folder never loads code from it;
- its path is absolute. A relative path would resolve against the working
  directory, which may be the project you opened.

```toml
[plugins]
dynamic = ["/home/you/plugins/libsample_plugin.so"]
```

On Windows, the plugin's own DLL dependencies are looked for next to the plugin,
next to Roxanne and in System32, never in the working directory.

Preferences → Plugins lists the native plugins in use. A plugin that fails to
load is skipped, and the status bar counts it among the configuration
warnings.

## Writing a native plugin

A native plugin exports one C function, `roxanne_plugin_api_v1`, which returns
its name and its callbacks:

```rust
#[repr(C)]
pub struct PluginStatusV1 {
    pub label: *const c_char,
    pub value: *const c_char,
}

#[repr(C)]
pub struct PluginApiV1 {
    pub name: unsafe extern "C" fn() -> *const c_char,
    pub on_text_changed: Option<unsafe extern "C" fn(*const c_char, *const c_char)>,
    pub on_file_opened: Option<unsafe extern "C" fn(*const c_char, *const c_char)>,
    pub on_file_saved: Option<unsafe extern "C" fn(*const c_char, *const c_char)>,
    pub status: Option<unsafe extern "C" fn() -> PluginStatusV1>,
}
```

- `on_text_changed` runs after every change to the text, `on_file_opened` after
  a file is opened and `on_file_saved` after it is saved. Each gets the whole
  text and the file name as NUL-terminated UTF-8; a NUL inside the text is
  replaced before the call. The pointers are valid only during the call.
- `status` returns the label and value shown in the status bar. Both strings
  must stay valid until the next call into the plugin, and must be UTF-8.
- Every callback runs on the interface thread, so a slow plugin slows the
  editor down.

[`examples/sample_plugin.rs`](../../examples/sample_plugin.rs) is a complete
plugin that counts words and lines. Build it from the repository root:

```bash
cargo build --release --example sample_plugin
```

The library lands in `target/release/examples/`: `libsample_plugin.so` on
Linux, `libsample_plugin.dylib` on macOS, `sample_plugin.dll` on Windows. Add
its absolute path to `plugins.dynamic` in your own `config.toml`, in
`Colony/Roxanne/` under your config directory (`~/.config/Colony/Roxanne/` on
Linux).
