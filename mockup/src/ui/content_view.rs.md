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

/// Screen lines of the body: rows `top..n`, each expanded to 1+ lines by `row_lines`;
/// the first `top_skip` lines of the top row are skipped; stops at `height`; cursor style
/// applied to every line of the cursor row. Shared by the three views.
pub fn body_lines<'a>(s: &AppState, height: usize, n: usize,
                      row_lines: impl FnMut(usize) -> Vec<Line<'a>>) -> Vec<Line<'a>>;

/// Columns taken by `ch` at column `col` (tab / `^X` / unicode width).
fn char_cols(ch: char, col: usize, tab: usize) -> usize;
/// Shared core of `styled_line` / `styled_wrapped`: renders a char-aligned byte range,
/// columns restarting at its start.
fn styled_range(s: &str, range: Range<usize>, spans: &[(Range<usize>, Style)], base: Style,
                marks: &[Mark], hscroll: usize, width: usize, tab_width: u8) -> Vec<Span<'static>>;

/// Byte offsets where the continuation segments of `line` start when wrapped to `width`
/// columns (empty = fits on one row).
pub fn wrap_breaks(line: &str, width: usize, tab_width: u8) -> Vec<usize>;

/// Like `styled_line`, one span list per screen row (`wrap_breaks` segments), no hscroll.
#[allow(clippy::too_many_arguments)]
pub fn styled_wrapped(text: &TextContent, hl: Option<&HighlightedText>, line: usize,
                      base: Style, marks: &[Mark], width: usize, tab_width: u8)
                      -> Vec<Vec<Span<'static>>>;

pub fn styled_line(text: &TextContent, hl: Option<&HighlightedText>, line: usize,
                   base: Style, marks: &[Mark],
                   hscroll: usize, width: usize, tab_width: u8) -> Vec<Span<'static>>;

/// Display column of byte `byte` in `line`, with the same rules as `styled_line`
/// (tabs, `^X`, wide chars). Used for the search's horizontal reveal.
pub fn col_of_byte(line: &str, byte: usize, tab_width: u8) -> usize;

/// The file search's matches on display row `row` (empty without a file search), from
/// `s.search.file_hits` (binary search by row, so renderers don't rerun the regex).
/// The current match (`search::current_match`) is flagged.
pub fn search_marks(s: &AppState, row: usize) -> Vec<Mark>;
pub struct Mark { pub bytes: Range<usize>, pub current: bool }

/// Columns left of the text: `digits(line_count) + 3` with line numbers, else 0
/// (2 when wrap is on, room for the `↪` marker).
pub fn gutter_cols(s: &AppState, text: &TextContent) -> usize;

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
content, one row per line (the top row starting at its `top_skip`-th segment); with wrap on, a line continues on the next screen rows, whose
gutter is the number column blank with a right-aligned dim `↪` (`  ↪ │ `), or `↪ `
when numbers are off. Rendering stops when the body is full (a partly visible last line
is clipped).

## Wrapping rules (`wrap_breaks`)
* Same column rules as `styled_line` (tabs, `^X`, wide chars). Tab stops are relative to
  the segment's start.
* Break **at word boundaries**: the segment ends after the last whitespace (following a
  word) that keeps it within `width`; the next segment starts at the following word.
  Leading indentation is not a break opportunity.
* Whitespace that overflows the width hangs at the end of the segment (the whole run),
  so no continuation row starts with spaces.
* A word longer than `width` is hard-broken at the last char that fits. A wide char
  never splits; with `width` 1 it still gets its own row.
* `width == 0` ⇒ no breaks (nothing is drawn anyway).

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
* `marks` (search matches, byte ranges in the same string) are applied last as
  `.patch(theme.search_match)`, or `.patch(theme.search_current)` for the current one,
  over syntax and diff tint. They come from the cached hit list (`search_marks`), so the
  renderer stays pure and cheap.
* Consecutive pieces with the same style are merged into one span.
* If `base` has a background (diff tint), the rest of the row is filled with spaces in
  `base` so the tint reaches the edge.

## Tests
`tabs_scroll_and_wide_chars`: tab expansion, hscroll, half-cut wide char → space, `^A`,
wide char not fitting. `wrap_word_boundaries`: breaks after spaces, long word
hard-broken, wide char at the edge moves to the next row, tabs. `styled_wrapped_spans`:
highlight / search marks crossing a break are split across the two rows.
`marks_styled_and_scrolled`: a mark spanning the hscroll edge is
only styled on its visible part; `col_of_byte` agrees with `styled_line` on tabs / wide chars.
