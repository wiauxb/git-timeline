# `src/ui/timeline_pane.rs`

## Purpose
Render the left pane: the list of revisions of the file.

## Contents

```rust
pub fn render(f: &mut Frame, area: Rect, s: &AppState);
fn change_span(c: &ChangeKind, conflicted: bool) -> Span<'static>;
fn row(e: &TimelineEntry, marked: bool, selected: bool, width: usize, now: DateTime<Local>,
       search: Option<&Query>) -> Line<'static>;
/// `text` as spans, the byte ranges `marks` patched with `search_match`.
fn marked(text: &str, marks: &[Range<usize>], style: Style) -> Vec<Span<'static>>;
/// Matches in `hay` (a prefix of `full`), cut to the prefix `fit` kept in `shown`.
fn fitted_marks(q: Option<&Query>, hay: &str, full: &str, shown: &str) -> Vec<Range<usize>>;
```

## Title
` Timeline ─ <ctx.target> (<count>)` + ` lines <L>` in `-L` mode + spinner while the
history is streaming. Border style: `border_focused` when the pane has focus.

## Row format

```
◆5b089ca   3d Alice        R move to src/main.rs  ← app.rs      (commit row, selected)
●Working tree                A new file                          (pseudo-entry row)
```
| Part | Content |
|---|---|
| marker (1 col) | `◆` marked (`mark` style) · `●` pseudo-entry · space |
| hash | 7-char short oid (`hash` style) |
| age | ` {:>4} ` compact age of the **author** date (`ui::age`) |
| author | `{:<12} ` (fitted to 12 cols); **only when the pane is ≥ 45 cols wide** |
| change | 1 col: `A` added/untracked, `D` deleted, `R` renamed, `C` copied, `T` type change, `⑂` merge, `!` conflicted, blank for modified |
| subject | rest of the width, fitted with `…`; on the **selected** row of a rename/copy, `  ← <old path>` is appended |

Pseudo-entries: `Working tree` / `Staged` padded to 12 cols (+13 blank cols when the author
column is shown), the change letter, then a dim word: `conflicted`, `untracked`,
`new file`, `deleted` or `modified`.

## Specs
* Rows `list_offset .. list_offset + inner.height` (offset maintained by `update.rs`).
* Selected row: whole line styled `selected_row` (focused) or `selected_row_unfocused`.
* One row per entry, no separator rows (row index == entry index; mouse clicks rely on it).
* Timeline search: matches are styled `search_match` (patched over the row style,
  selected row included). Hash: the matched prefix, cut to the 7 shown chars. Subject:
  every `find_iter` range, cut where `fit` truncates. Pseudo-entries: the matched part of
  the label.
* Empty timeline: centered dim `loading history…` (while streaming) or `no revisions`.
