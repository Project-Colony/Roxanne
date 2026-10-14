# Why Roxanne exists

## The problem

Editing a file of code usually means choosing between two kinds of tool. An IDE
understands the code, but wants a project first, takes its time to start, and
runs the project's own build scripts and macros to understand it, which is
code you may not have read yet. A plain text editor opens anything at once but
knows nothing about what it shows.

Roxanne sits between the two: a native editor that opens any file without a
project, colours the code, and brings in a language server only when asked.

## What Roxanne promises

- **Responsive on large files.** The text lives in a rope, highlighting reparses
  only what changed, and slow work (reading, saving, searching a workspace,
  rust-analyzer) never runs on the interface thread. Benchmarks with CI limits
  guard the core; see [../internals/performance.md](../internals/performance.md).
- **Safe by default.** Opening a folder never loads or runs code from it, and a
  save can never leave a file half written.
- **Keyboard first, configurable.** Every action is in a menu and the command
  palette, shortcuts can be rebound and grouped into profiles, and settings
  live in plain TOML files that the Preferences page edits without rewriting.
- **A Colony program.** Roxanne shares the Project Colony themes, fonts,
  accessibility options and Preferences layout with the other programs of the
  organisation, installs and updates through
  [Colony](https://github.com/Project-Colony/Colony), and ships signed releases.

## Goals

These are targets Roxanne aims for, not measurements: nothing checks them yet.
The benchmarks in [../internals/performance.md](../internals/performance.md)
guard the editor core against regressions, which is not the same thing.

- Start in under one second on a mid-sized project, with a modest memory
  footprint on a modest machine.
- Scroll and edit a file of 50,000 lines with no visible latency.

## Risks

- Fast text rendering on Linux, Windows and macOS is hard. Roxanne leaves it to
  iced's text editor widget, so its limits are Roxanne's limits.
- The window and the text model can grow coupled if the line between `ui` and
  `editor` blurs, which makes both harder to test and to change. The module
  rules in [CONTRIBUTING.md](../../CONTRIBUTING.md#module-structure) are there
  to prevent it.
- Keyboard ergonomics take more work than they seem to: shortcuts, modes and
  the way focus moves between panels need care to stay predictable.

## Who it is for

- Developers who want a quick editor for code and configuration files beside,
  or instead of, an IDE.
- People reading unfamiliar code, who want highlighting and navigation without
  running that code.
- Anyone who works mostly from the keyboard.

## What Roxanne is not

- An IDE: no debugger, no build system integration, no project model.
- A rich-text or WYSIWYG editor.
- A collaborative or web-based editor.

## Where it stands

Roxanne is in early development. The status at the top of the
[README](../../README.md) says what is proven and what is not yet, and the
[releases](https://github.com/Project-Colony/Roxanne/releases) say what changed
in each version.
