# `src/ui/file_pane.rs`

## Purpose
Frame of the right pane: border with title, one header line, then the body rendered by
the view of `s.displayed()` (or a placeholder). Returns the body rect.

## Contents

```rust
pub fn render(f: &mut Frame, area: Rect, s: &AppState) -> Rect;   // body rect
fn placeholder(f: &mut Frame, area: Rect, msg: &str);              // dim, centered, at 1/3 height
fn describe(c: &FileContent, s: &AppState) -> String;              // non-text content message
fn title(s: &AppState, width: usize) -> Line<'static>;
fn header(s: &AppState, d: &Displayed) -> Line<'static>;
/// Columns left of the text in the current view: dispatches to the `gutter_cols` of
/// `content_view` / `diff_view` / `blame_view` (which the renderers use themselves), so
/// the search's horizontal reveal and the rendering agree.
pub fn gutter_cols(s: &AppState, d: &Displayed) -> usize;
```

## Geometry
Inner area of the bordered block: row 0 = header, rows 1.. = body. Inner height < 2 ⇒
nothing drawn, returns `Rect::default()`. A vertical `Scrollbar` (no end symbols) is drawn on
the right border over the body rows when `row_count > body.height`, positioned at `file.top`.

## Title (top border)
* Normal: ` <label> ` (`hash` style, or `pseudo_entry` for Working tree / Staged), then for
  commits `· <author> · <ago> · <subject> ` and finally `─ [<View>] `.
  View names: `Content`, `Diff`, `Compare`, `Blame`.
* The subject is fitted to `width − (spans so far + 2 borders + 3 for "· " and space +
  len(view) + 5 for "─ [ ] ")`, so the `[View]` tag always stays visible.
* `DiffMarked` with a distinct mark: ` <older label> → <newer label> ─ [Compare] `.
* Nothing selected: ` [<View>] `.

## Header (first body row, dim, parts joined by ` · `)
| Displayed | Parts |
|---|---|
| `Text` | path at revision · `N lines` · `wrap`? · syntax name or highlight note (`plain text`, `no highlighting (too large)`, …) · `CRLF`? · `non-UTF-8`? · `⏎ jump to commit` (blame view) |
| `Diff` | `a/<old path> @ <old label>  →  b/<new path> @ <new label>` · `+A −R` · `N hunks` · `wrap`? |
| `DiffSummary` | the same `old → new` labels |
| `NotText` / others | the path |
| any non-diff view of a renamed entry | `renamed from <old> (<similarity>%)` |

## Body dispatch
| `Displayed` | Rendering |
|---|---|
| `Text` + `ViewMode::Blame` | `blame_view::render` with `s.blame()` |
| `Text` | `content_view::render` |
| `Diff` | `diff_view::render` (`DiffInputs`) |
| `DiffSummary` | placeholder `diff_view::summary(old, new)` |
| `NotText` | placeholder `describe()`: `file deleted in this revision` (commit) / `file does not exist here` (local), `binary file, 12.3 KiB`, `file too large to display (a > b)`, `symlink → target`, `submodule at commit abc1234` |
| `Waiting` | `<spinner> loading…` |
| `Failed(e)` | `error: e` (error style) on the first body row |
| `NoMark` | `no compare base — press m on a revision to mark it` |
| `SameRevision` | `the marked revision is the selected one` |
| `NoSelection` | `loading history…` / `nothing selected` |
