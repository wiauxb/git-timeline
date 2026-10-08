# `src/model/line_map.rs`

## Purpose
**"Keep the same logical line"** when switching revision: given a line in version A, find
the corresponding line in version B.

## Contents

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MappedLine {
    Exact(usize),     // unchanged line
    Nearest(usize),   // changed/removed; closest surviving line
}
impl MappedLine { pub fn line(&self) -> usize; }

/// What the view remembers to re-anchor after a revision or view change.
#[derive(Clone, Debug)]
pub struct LineAnchor {
    pub content: ContentKey,     // version the line refers to
    pub line: usize,
    pub viewport_offset: usize,  // cursor row - top row
}

/// `diff` is old→new; forward=true maps old→new, false maps new→old.
pub fn map_line(diff: &FileDiff, line: usize, forward: bool) -> MappedLine;
```

## Algorithm
Target length = end of the last op's target range; empty target ⇒ `Nearest(0)`.
Find the op whose source range contains `line` (`off = line - src.start`):
* `Equal` ⇒ `Exact(dst.start + off)`.
* empty target range (pure delete) ⇒ `Nearest(min(dst.start, target_len - 1))`.
* otherwise (replace) ⇒ `Nearest(dst.start + min(off, dst.len - 1))`.
No op contains it (beyond the end) ⇒ `Nearest(target_len - 1)`.

## Specs
* The diff used is **between the two versions directly** (A vs B), never a composition of
  per-commit diffs. `app/update.rs::resolve_anchor` requests it (`ComputeDiff`) when not
  cached; same `ContentKey` on both sides ⇒ no diff needed, line kept.
* After mapping, the view restores `viewport_offset` (clamped), avoids blank rows at the
  bottom, and in diff views shows the hunk header just above the top row.

## Tests
Unchanged, insertion before (both directions), deleted block, replaced block of different
size, delete at EOF, to/from empty.
