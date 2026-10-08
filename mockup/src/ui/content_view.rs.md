# `src/ui/content_view.rs`

## Purpose
Render the full file at the selected revision (the default view).

## Contents

```rust
pub fn render(f: &mut Frame, area: Rect, s: &AppState,
              text: &TextContent, hl: Option<&HighlightedText>);

/// Shared with diff_view / blame_view.
pub(super) fn styled_line<'a>(text: &'a TextContent, hl: Option<&HighlightedText>,
                              line: usize, hscroll: usize, width: u16,
                              tab_width: u8) -> Line<'a>;
```

## Specs
* Rows = content lines (no wrap) or wrapped segments (`wrap = true`; then `cursor`/`top`
  count *visual* rows and a `row → (line, segment)` index is computed for the viewport
  only).
* Gutter: right-aligned 1-based line numbers, width = digits of `line_count`, dim; hidden
  with `#`.
* Cursor line: full-width `cursor_line` background (only when the file pane is focused;
  dim underline otherwise).
* `styled_line`: expand tabs to `tab_width`, compute display widths with
  `unicode-width`, slice by `hscroll` columns (never splitting a wide char: replace a
  half-cut wide char by a space), apply highlight spans; control chars shown as `^X`
  (dim).
* Lines longer than the viewport: a dim `›` at the right edge.
* Vertical scrollbar (ratatui `Scrollbar`) on the right border when content overflows.
* No trailing-newline marker in this view (only in diffs).
