# `src/ui/diff_view.rs`

## Purpose
Render a unified diff between two versions, used by both `DiffPrev` (change introduced
by the selected revision) and `DiffMarked` (marked ↔ selected).

## Contents

```rust
pub fn render(f: &mut Frame, area: Rect, s: &AppState, d: &DiffInputs);

pub struct DiffInputs<'a> {
    pub diff: &'a FileDiff,
    pub old: Option<&'a TextContent>, pub old_hl: Option<&'a HighlightedText>,
    pub new: Option<&'a TextContent>, pub new_hl: Option<&'a HighlightedText>,
    pub old_label: String,     // "a/old/path @ 9f0e1d2"
    pub new_label: String,     // "b/src/app.rs @ abc1234"
}

/// Rendered rows (built lazily per frame for the visible window; hunk boundaries are
/// precomputed in FileDiff so random access is O(log hunks)).
pub(crate) enum DiffRow { FileHeader, HunkHeader(usize), Line(DiffLine), NoNewlineMarker(Side), Fold(usize /*hidden lines*/) }
pub(crate) fn row_count(d: &FileDiff) -> usize;
pub(crate) fn row_at(d: &FileDiff, row: usize) -> DiffRow;
pub(crate) fn source_line_of_row(d: &FileDiff, row: usize) -> usize;   // new-side line (for anchors)
pub(crate) fn row_of_source_line(d: &FileDiff, line: usize) -> usize;
```

## Layout
```
--- a/app.rs @ 9f0e1d2
+++ b/src/app.rs @ abc1234          (renamed, 97 %)
@@ -10,7 +10,8 @@ fn main()
  10  10 │   let x = 1;
  11     │ - let y = 2;
      11 │ + let y = compute();
      12 │ + log(y);
  12  13 │   run(x, y);
```
* Two line-number columns (old, new), then `+`/`-`/space marker, then text with
  syntax highlighting from the **corresponding side's** highlighted content and a
  background tint for added/removed rows.
* Hunk header shows git-style `@@ -a,b +c,d @@`, without the function context (v1).
* Identical versions: centered "no changes" (e.g. merge where the file equals first parent,
  or a mode-only change → "mode changed 100644 → 100755").
* Added file: everything `+`; deleted: everything `-`.
* `timed_out` ⇒ status warning "diff simplified (file too large for exact diff)".
* Non-text pairs ⇒ one-line summary (see `model/diff.rs`).
