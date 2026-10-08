# `src/ui/file_pane.rs`

## Purpose
Frame of the right pane: border, title, header line; dispatches the body to the
renderer of the current `ViewMode`, or renders a placeholder (loading / absent / binary /
error).

## Contents

```rust
pub fn render(f: &mut Frame, area: Rect, s: &AppState);
fn title(s: &AppState) -> Line<'_>;
fn header(s: &AppState) -> Line<'_>;
fn placeholder(f: &mut Frame, area: Rect, p: Placeholder);

enum Placeholder<'a> {
    Loading,                                  // spinner
    Absent { deleted_in: Option<&'a str> },   // "file does not exist at this revision"
    Binary { size: u64 },
    TooLarge { size: u64, limit: u64 },
    Submodule { commit: &'a Oid },
    Symlink { target: &'a [u8] },
    NoMark,                                   // DiffMarked without mark
    SameRevision,                             // DiffMarked with mark == selected
    Error(&'a str),
}
```

## Title (top border)
`<short-oid | Working tree | Staged> · <author> · <relative date> ─ [<View>]`
For `DiffMarked`: `9f0e1d2 → abc1234 ─ [Compare]`.

## Header line (first body row)
`<path at this revision> · <N lines> · <syntax> [· CRLF] [· non-UTF-8] [· +12 −3]`
(`+/−` counts only in diff views; `· no highlighting (too large)` when skipped).

## Dispatch
| ViewMode | Renderer |
|---|---|
| Content | `content_view::render` |
| DiffPrev / DiffMarked | `diff_view::render` |
| Blame | `blame_view::render` |

Before content is available: show **the previous displayed content dimmed** if the
switch happened < 150 ms ago (avoids flashing), then `Loading`.
