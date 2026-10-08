# `src/ui/blame_view.rs`

## Purpose
Render the file at the selected revision with a blame gutter showing, for each line,
which commit last changed it (as of that revision).

## Contents

```rust
pub fn render(f: &mut Frame, area: Rect, s: &AppState,
              text: &TextContent, hl: Option<&HighlightedText>, blame: Option<&BlameData>);
```

## Layout
```
abc1234 Alice   2024-03-02 │  1 │ use std::sync::Arc;
                           │  2 │
9f0e1d2 Bob     2023-11-20 │  3 │ pub struct App {
                           │  4 │     state: AppState,
00000000 (uncommitted)     │  5 │     dirty: bool,
```
* Gutter width 28 cols: short oid, author (truncated to 8), ISO date; printed only on
  `group_start` lines.
* The short oid is colored by **age** (`theme.blame_age`, 6 buckets between oldest and
  newest commit in this blame).
* If the commit is **in the timeline**, its oid is underlined: `Enter` (BlameJump) selects
  that revision; otherwise status "commit not in current timeline (filtered)".
* Cursor line: gutter shows the full summary of that line's commit in the status bar.
* Blame loading: content shown immediately with an empty gutter and a spinner in the title.
* Blame error: inline red line at the top of the body, content still shown.
