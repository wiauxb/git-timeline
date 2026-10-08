# `src/ui/content_view.rs`

## Purpose
Render the full file at the selected revision (the default view), and provide the
**line renderer shared by all views** (content, diff, blame).

## Contents

```rust
pub fn render(f: &mut Frame, area: Rect, s: &AppState, text: &TextContent, hl: Option<&HighlightedText>);

/// Cursor-line style: only on row == s.file.cursor and when no anchor is pending;
/// `cursor_line` (bg) when the file pane has focus, `cursor_line_unfocused` (underline) otherwise.
pub fn cursor_style<'a>(line: Line<'a>, s: &AppState, row: usize) -> Line<'a>;

pub fn digits(n: usize) -> usize;   // decimal width of max(n, 1)

pub fn styled_line(text: &TextContent, hl: Option<&HighlightedText>, line: usize,
                   base: Style, hscroll: usize, width: usize, tab_width: u8) -> Vec<Span<'static>>;

struct LineBuilder { hscroll, width, col, used, buf: String, buf_style: Style, out: Vec<Span<'static>> }
impl LineBuilder {
    fn push(&mut self, piece: &str, w: usize, style: Style) -> bool;  // false = line full
    fn flush(&mut self);
}
```

## Layout
```
  9 │ # Protect against non-zsh execution
 10 │ [ -n "$ZSH_VERSION" ] || {
```
Gutter (when `show_line_numbers`): right-aligned 1-based number, width `digits(line_count)`,
then ` │ ` (`line_number` style). Text width = body width − gutter. Rows `top..` of the
content, one row per line (no soft wrap).

## `styled_line` rules
* Works on `TextContent::line_str(line)` (lossy UTF-8); highlight spans are byte ranges in
  that string, applied as `base.patch(span_style)`.
* Tab ⇒ spaces up to the next multiple of `tab_width` (min 1).
* Control chars ⇒ `^X` (`char ^ 0x40`), 2 cols, `dim` style.
* Other chars: display width from `unicode-width`.
* Horizontal scroll by **display columns**: pieces fully left of `hscroll` are skipped; a
  piece cut by the scroll edge (wide char, tab) is replaced by spaces for its visible part.
* Stops at the first piece that doesn't fit in `width` (a wide char never half-shows on
  the right).
* Consecutive pieces with the same style are merged into one span.
* If `base` has a background (diff tint), the rest of the row is filled with spaces in
  `base` so the tint reaches the edge.

## Tests
`tabs_scroll_and_wide_chars`: tab expansion, hscroll, half-cut wide char → space, `^A`,
wide char not fitting.
