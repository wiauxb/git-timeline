# `src/app/viewport.rs`

## Purpose
Screen-row geometry of the file pane. `cursor` and `top` are **logical rows** (a line of
Content / Blame, a row of a diff). With soft wrap on (`file.wrap`), one logical row takes
1+ screen rows. Every place that turns logical rows into screen rows goes through here.
With wrap off, every height is 1 and each function reduces to the old arithmetic.

## Contents

```rust
/// Screen rows taken by `row`: 1 when wrap is off, for hunk headers, and for rows
/// without text; else `wrap_breaks(text, text_cols).len() + 1`.
pub fn row_height(s: &AppState, d: &Displayed, row: usize) -> usize;
/// Text columns of the current view: `file_cols() − file_pane::gutter_cols(s, d)`, min 1.
pub fn text_cols(s: &AppState, d: &Displayed) -> usize;
/// Set `top`, resetting `top_skip` when it changes. Every `top` write outside
/// `reveal` goes through it.
pub fn set_top(s: &mut AppState, top: usize);
fn visible_height(s: &AppState, d: &Displayed, row: usize) -> usize;   // minus top_skip on `top`
/// Screen rows from the top of `top` to the end of `row` (inclusive), capped at `cap`.
pub fn screen_span(s: &AppState, d: &Displayed, top: usize, row: usize, cap: usize) -> usize;
/// Largest `top` for which rows `top..` still fill the screen (0 if the content is
/// shorter): no blank rows at the bottom.
pub fn max_top(s: &AppState, d: &Displayed) -> usize;
/// Largest `top` with at most `offset` screen rows above `cursor` (anchor restore).
pub fn top_for_offset(s: &AppState, d: &Displayed, cursor: usize, offset: usize) -> usize;
/// Screen rows above `cursor`, from `top` (anchor capture).
pub fn offset_of(s: &AppState, d: &Displayed) -> usize;
/// Logical rows to move so the move covers `screen` screen rows (≥ 1) — page / half page.
pub fn rows_for_screen(s: &AppState, d: &Displayed, from: usize, screen: usize, forward: bool) -> usize;
/// Logical row drawn at screen row `y` of the body (mouse click); None past the content.
pub fn row_at_screen(s: &AppState, d: &Displayed, y: usize) -> Option<usize>;
/// `top` adjusted so the whole cursor row is visible (its first screen row when it is
/// taller than the body). Resets `top_skip` to 0 whenever it changes `top`.
pub fn scroll_to_cursor(s: &mut AppState);
/// Make screen row `segment` of `row` visible: like `scroll_to_cursor`, but if `row`
/// is taller than the body and `segment` is not on screen, then `top = row` and
/// `top_skip = segment − body/3` (saturating; clamped so the line's end isn't followed
/// by blank rows when the line could fill the body).
pub fn reveal(s: &mut AppState, d: &Displayed, row: usize, segment: usize);
```

## Specs
* Heights are computed on demand, only for the rows the caller walks over. A walk is
  capped at `file_rows()` screen rows, so the cost depends on the screen height, not on
  the file size. (A very long single line still costs O(length) for its own breaks.)
* `d` is passed in so a caller resolves `displayed()` once (it rebuilds the diff rows).
* Row text comes from `Displayed::row_text`. That's the same text the views render and
  the search uses.
* `top_skip` hides the first screen rows of `top`. Every function that counts screen
  rows from `top` subtracts it; the renderers skip those segments of the top row.
  Anything that moves `top` (cursor moves, pages, anchor restore, view or revision
  change, wrap toggle, resize) sets it back to 0, so it only lives while you step
  through matches of one tall line.
* Otherwise a row taller than the body is shown from its first screen row and clipped.
  Scrolling through a tall line row by row (`gj`-style) is out of scope; `n`/`N` are the
  way to move inside one.
