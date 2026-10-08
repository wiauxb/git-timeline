# `src/ui/status_bar.rs`

## Purpose
One-row bar at the bottom: focus badge, view tabs, transient message or key hints, and the
cursor position.

## Contents

```rust
pub fn render(f: &mut Frame, area: Rect, s: &AppState);
fn hints(s: &AppState) -> String;
fn position(s: &AppState) -> String;   // "cursor+1/rows " when the view has rows
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
4. Right-aligned position `N/M ` (dim).

## Hints
* Timeline focus: `m mark · y copy · e export · R restore · d details · ? help`
* File focus, Content: `[ ] older/newer · # numbers · z zoom · ? help`
* File focus, diff views: `n/N hunk · [ ] older/newer · z zoom · ? help`
* File focus, Blame: `⏎ jump to commit · [ ] older/newer · # numbers · ? help`

Message expiry is handled by `update.rs` (`Tick`): info 3 s, warn 5 s, errors persist.
