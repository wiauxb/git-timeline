# `src/ui/timeline_pane.rs`

## Purpose
Render the left pane: the list of revisions of the file.

## Contents

```rust
pub fn render(f: &mut Frame, area: Rect, s: &AppState);
fn row(e: &TimelineEntry, s: &AppState, width: u16, now: DateTime<Local>) -> Line<'_>;
```

## Row format (fitted to width, subject truncated with `…`)

```
<mark><hash>  <age> <author>  <change> <subject>
 ◆    abc1234  3d   Alice     R      Move app to src/
```
| Column | Content |
|---|---|
| mark | `◆` if marked, `●` for pseudo-entries, else space |
| hash | 7-char short oid; pseudo-entries: `Working tree` / `Staged` spanning hash+age+author |
| age | compact relative author date: `5m`, `3h`, `2d`, `3w`, `4mo`, `2y` |
| author | author name, truncated to 12 cols (hidden if pane < 45 cols) |
| change | one letter colored: `A` added, `M` modified (blank to reduce noise), `D` deleted, `R` renamed, `C` copied, `T` type change, `⑂` merge, `!` conflicted |
| subject | commit subject; for `R`, the subject is followed by dim `← old/path` on the selected row only |

## Specs
* Title: `Timeline ─ <current path> (<count>)` and ` …` + spinner while streaming.
* Selected row style differs when the pane has focus vs not.
* Scrolling: `list_offset` maintained by `update.rs` (keeps 2 rows of margin).
* One row per entry, no separator rows (keeps row index == entry index). Renames are
  conveyed by the `R` letter and the `← old/path` suffix on the selected row.
* Empty timeline while loading: centered `loading history…`.
* `-L` mode: title shows `lines 40-80`.
