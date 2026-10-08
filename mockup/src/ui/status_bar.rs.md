# `src/ui/status_bar.rs`

## Purpose
One-row bar at the bottom: focus badge, view tabs, transient message or key hints, and the
cursor position.

## Contents

```rust
pub fn render(f: &mut Frame, area: Rect, s: &AppState);
fn badge(s: &AppState) -> Span<'static>;
fn hints(s: &AppState) -> String;
fn position(s: &AppState) -> String;   // "cursor+1/rows " when the view has rows
fn search_info(s: &AppState) -> String; // "/<query> k/M · " for the focused pane's search
fn prompt(f: &mut Frame, area: Rect, s: &AppState, p: &SearchPrompt);
```

## Layout
```
 TIMELINE   1 Content  2 Diff  3 Cmp  4 Blame  │ m mark · y copy · e export · R restore · d details · ? help      1/347
```
1. Badge ` TIMELINE ` / ` FILE ` (`status_badge`), a space.
2. Tabs ` 1 Content `, ` 2 Diff `, ` 3 Cmp `, ` 4 Blame `: current = `status_tab_active`;
   `3 Cmp` is dim while no mark is set.
3. ` │ `, then either the status message (`status_info` / `status_warn` /
   `status_error` by level) or the hints (dim), fitted to the remaining width.
4. Right-aligned position `N/M ` (dim). When the focused pane has a search, it is
   preceded by `/<query> k/M · ` (`<query>` fitted to 20 cols): `k` = 1-based number of
   the hit on the current row, or `-` when the current row is not a hit; `M` = hit count
   (`no match` when 0).

## Search prompt (while `Overlay::Search` is open)
The whole bar is replaced, vim-style:
```
 FILE  /fn\s+ma▏                                                       3 matches
```
Badge, then `/` plus the input; the terminal cursor is placed at `input.cursor` (as in
the export prompt). On the right (fitted to half the width): `N matches` / `1 match` /
`no match` (dim), or `invalid regex: <msg>` (`status_error`); nothing while the input is
empty.

## Hints
* Timeline focus: `/ search · m mark · y copy · e export · R restore · d details · ? help`
* File focus, Content: `/ search · [ ] older/newer · # numbers · z zoom · ? help`
* File focus, diff views: `{ } hunk · / search · [ ] older/newer · z zoom · ? help`
* File focus, Blame: `⏎ jump to commit · [ ] older/newer · # numbers · ? help`

Message expiry is handled by `update.rs` (`Tick`): info 3 s, warn 5 s, errors persist.
