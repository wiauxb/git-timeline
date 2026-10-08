# `src/ui/mod.rs`

## Purpose
Root rendering function. **Pure**: `&AppState` in, frame out; returns the computed
layout so the app can do mouse hit-testing and page-size math.

## Contents

```rust
pub mod theme; pub mod timeline_pane; pub mod file_pane;
pub mod content_view; pub mod diff_view; pub mod blame_view;
pub mod status_bar; pub mod help; pub mod dialog;

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub struct LayoutInfo {
    pub timeline: Rect,        // zero-size when zoomed
    pub file: Rect,
    pub file_body_rows: u16,   // rows available for text (minus header)
    pub file_body_cols: u16,
}

pub fn draw(frame: &mut Frame, s: &AppState) -> LayoutInfo;
```

## Screen layout

```
┌ Timeline ─ src/app.rs (42) ──────┐┌ abc1234 · Alice · 3 days ago ─ [Content] ──────────────┐
│● Working tree        modified    ││ src/app.rs  · 214 lines · rust                          │
│● Staged              modified    ││  1 │ use std::sync::Arc;                                │
│  abc1234 3d  Alice   Fix crash.. ││  2 │                                                    │
│◆ 9f0e1d2 1w  Bob     Refactor .. ││  3 │ pub struct App {                                   │
│  77aa010 2w  Alice   R  app.rs ← ││  4 │     state: AppState,                               │
│  …                                ││  …                                                     │
│  loading… 1 280 commits          ││                                                         │
└───────────────────────────────────┘└─────────────────────────────────────────────────────────┘
 TIMELINE │ 1 Content 2 Diff 3 Cmp 4 Blame │ m mark · y copy · e export · R restore · ? help
```

* Horizontal split: timeline **35 %** (min 30 cols, max 60 cols), file pane the rest.
* `zoomed` ⇒ file pane takes the full width.
* Terminal narrower than 60 cols ⇒ auto-zoom on file focus (only one pane visible).
* Bottom: 1-row status bar.
* Overlays (help, dialogs) are drawn last, centered, over a `Clear` widget.

## Draw order
1. split layout → `LayoutInfo`
2. `timeline_pane::render` (unless zoomed)
3. `file_pane::render`
4. `status_bar::render`
5. `dialog::render` / `help::render` if `s.overlay` is some
