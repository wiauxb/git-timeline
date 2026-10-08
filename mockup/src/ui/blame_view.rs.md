# `src/ui/blame_view.rs`

## Purpose
Render the file at the selected revision with a blame gutter showing, for each line, the
commit that last changed it (as of that revision).

## Contents

```rust
const GUTTER: usize = 28;

pub fn render(f: &mut Frame, area: Rect, s: &AppState, text: &TextContent,
              hl: Option<&HighlightedText>, blame: Option<Result<&BlameData, &str>>);
/// Columns left of the text: GUTTER + 2 + (`digits + 3` with line numbers, else 1).
pub fn gutter_cols(s: &AppState, text: &TextContent) -> usize;
```

## Layout
```
0fed366 Colin Ba… 2024-03-15 │  1 │ # ANSI formatting function
   ┆                         │  2 │ # 0: reset, 1: bold
0da7919 Robby Ru… 2026-09-07 │  3 │ omz_f() {
(not committed yet)          │  4 │   x=5
```
* Gutter = exactly 28 cols: short oid (7) + ` ` + author fitted to 9 cols + ` ` + date
  `%Y-%m-%d` (10). Printed on group-start lines **and on the top visible row**; other rows
  show a dim `   ┆` padded to 28.
* Null oid ⇒ dim `(not committed yet)` padded to 28.
* Then ` │`, then line numbers `{:>digits} │ ` (when shown) or a single space, then the
  text via `styled_line`, with search matches (`content_view::search_marks`) on the text only.
* Hash color = age bucket `round((newest − t) / (newest − oldest) × 5)` into
  `theme.blame_age` (skipped in monochrome); underlined (`link`) when that commit is in
  the current timeline (⇒ `Enter` can jump to it).

## States
* `blame == None` (loading): top visible row shows `<spinner> blame…` in the gutter, the
  rest blank gutters; content is shown normally.
* `Some(Err(e))`: a `blame failed: e` row (error style) at the top of the body, content
  shifted down one row, no gutter.
* `Some(Ok(b))` with `b.lines.len() != text.line_count()` (working-tree file changed
  between read and blame): treated like `None` (no gutter).
