# Vendored `iced_widget` — Patch Documentation

This directory contains a vendored copy of `iced_widget v0.12.3` with two modifications.

## Why

The standard iced 0.12 `text_editor` widget does not expose configurable **line height**.
Roxanne needs consistent line spacing in the editor area, so we vendor this crate to add
that support.

## Changes from upstream `iced_widget 0.12.3`

### 1. Line height support in `text_editor` (src/text_editor.rs)

Added a `.line_height(LineHeight)` builder method to the `TextEditor` widget, allowing
the caller to control vertical spacing between lines. This is used in `app.rs`:

```rust
text_editor(&self.content)
    .line_height(LineHeight::Absolute(line_height.into()))
```

### 2. Lifetime fix in `pane_grid` (src/pane_grid.rs, line 451)

Fixed an incorrect anonymous lifetime in the `overlay` method:

```rust
// Before (upstream):
) -> Option<overlay::Element<'_, Message, Theme, Renderer>> {

// After (patched):
) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
```

This corrects the returned lifetime to match the method's `'b` parameter.

## Maintenance

When upgrading iced, check whether upstream has added line height support to
`text_editor`. If so, this vendor can be removed and replaced with the crate
dependency. The patch is tracked in the project's `Cargo.toml`:

```toml
[patch.crates-io]
iced_widget = { path = "vendor/iced_widget" }
```

## Commits

- `e36509d` — Add line height support to text editor
- `1d6c011` — Fix warning visibilities and lifetime
