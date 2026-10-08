# `src/ui/mod.rs`

## Purpose
Root rendering function. **Pure**: `&AppState` in, frame out. Returns the computed
layout; the app feeds it back through `Action::Layout` (mouse hit-testing, page sizes).
Also hosts small text helpers shared by the panes.

## Contents

```rust
pub mod blame_view; pub mod content_view; pub mod dialog; pub mod diff_view;
pub mod file_pane; pub mod help; pub mod status_bar; pub mod theme; pub mod timeline_pane;

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub struct LayoutInfo {
    pub timeline: Rect,        // zero-size when hidden
    pub file: Rect,            // zero-size when hidden
    pub file_body: Rect,       // rows where file text is drawn (below the header line)
    pub file_body_rows: u16,   // = file_body.height
    pub file_body_cols: u16,   // = file_body.width
}

pub fn draw(f: &mut Frame, s: &AppState) -> LayoutInfo;

pub fn age(now: DateTime<Local>, t: &DateTime<FixedOffset>) -> String;  // 5m 3h 2d 3w 4mo 2y
pub fn ago(now: DateTime<Local>, t: &DateTime<FixedOffset>) -> String;  // "just now", "3 days ago"
pub fn fit(s: &str, w: usize) -> String;          // truncate to w display cols, `…` when cut
pub fn centered(area: Rect, w: u16, h: u16) -> Rect; // clamped to area minus 1-cell margin
```

## Screen layout

```
┌ Timeline ─ src/main.rs (6) ───────────────┐┌ 5b089ca · Alice · 3 days ago · v3: bump ─ [Content] ────┐
│●Working tree   modified                   ││src/main.rs · 4 lines · Rust                              │
│●Staged         modified                   ││1 │ fn main() {                                            │
│ 5b089ca   3d   v3: bump                   ││2 │     let x = 3;                                         │
│◆cbf9825   1w R move to src/main.rs        ││3 │     println!("v3 {x}");                                │
│ aa0c487   2w   v2: add x                  ││4 │ }                                                      │
│ a6ac609  1mo A v1: hello                  ││                                                          │
└───────────────────────────────────────────┘└──────────────────────────────────────────────────────────┘
 TIMELINE   1 Content  2 Diff  3 Cmp  4 Blame  │ m mark · y copy · e export · R restore · d details · ? help   1/4
```

* Vertical split: main area + 1-row status bar.
* Pane choice:
  * `zoomed`, or terminal narrower than **60 cols** with focus on the file ⇒ file pane only;
  * narrower than 60 cols with focus on the timeline ⇒ timeline only;
  * otherwise timeline width = 35 % of the width, clamped to `[30, 60]` and to
    `width - 20`; the file pane takes the rest.
* A hidden pane gets `Rect::default()` and is not rendered.

## Draw order
1. layout split
2. `timeline_pane::render` (if visible)
3. `file_pane::render` (if visible) → returns `file_body`
4. `status_bar::render`
5. `dialog::render(f.area(), …)` if `s.overlay` is some (it also dispatches help)

## Helpers
* `age`: `<1h` minutes, `<1d` hours, `<14d` days, `<8w` weeks, `<365d` months (30-day),
  else years. Negative ages clamp to 0.
* `ago`: same idea, long form, singular/plural ("1 day ago", "2 months ago").
* `fit`: uses `unicode-width`; returns the string as-is if it fits, otherwise as many
  chars as fit in `w-1` cols plus `…` (empty when `w == 0`).
