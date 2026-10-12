# Rendering

Roxanne draws its text with iced's `text_editor` widget. The widget lays out
and paints the text, the cursor and the selection; Roxanne gives it the font,
the size, the wrapping mode and a highlighter. The code for the text area is in
`src/ui/editor_view.rs`.

## Fonts and sizes

Code is set in JetBrainsMono Nerd Font, and the rest of the interface in the
Colony interface font, both embedded by colony-ui and registered when the
window opens. The code size and the line height follow the user's text scale
(`font_size` times `text_size_a11y`), and the line height is an absolute value
so the line numbers in the gutter stay level with their lines. With the
dyslexia font on, the interface switches to OpenDyslexic but code stays
monospaced.

## Highlighting

`editor::highlight::RoxanneHighlighter` implements iced's `Highlighter` trait.
iced asks it for one line at a time, from the first line that changed, and
caches the result until the settings change.

- The language comes from the file extension (`Language::from_extension`).
  Plain text gets no colours.
- For the eight supported languages, `editor::syntax` keeps a tree-sitter
  parser, the language's highlights query and the current syntax tree. When
  the text changes, it works out the single edited range between the old and
  the new text, applies it to the old tree and reparses incrementally, so a
  keystroke does not reparse the whole file.
- A line's colours are the query captures inside that line's byte range,
  mapped to a few tokens: keyword, type, string, comment and number.
- Search matches are added on top as their own token.

The colours come from the active Colony palette (`SyntaxColors::current`):
keywords take the accent, strings, types and numbers take the palette's status
colours, comments its dimmest text colour. The colours are part of the
highlighter's settings, so a theme or accent change highlights every line
again.

## Gutter and minimap

- The gutter shows the numbers of the lines in view, from the scroll offset the
  window tracks alongside the widget. It is hidden while words wrap, since a
  wrapped line takes several rows and the numbers would drift from their
  lines.
- The minimap samples up to 60 lines spread evenly over the file and draws one
  block character for each, denser for longer lines, with a solid block where
  the cursor is and the cursor's line number below.

## The viewport cache

`editor::ViewportCache` keeps the text of the lines around the cursor, about one
editor height, and reuses the lines it already holds when the buffer's revision
has not changed. The widget does not draw from it. Its refresh time is what
**Tools → Performance Report** calls *Render*. The `viewport` benchmarks measure it on 10,000 and
50,000 lines; see [performance.md](performance.md).
