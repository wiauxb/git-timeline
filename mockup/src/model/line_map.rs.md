# `src/model/line_map.rs`

## Purpose
Implement **"keep the same logical line"** when switching revision: given a line in
version A, find the corresponding line in version B.

## Contents

```rust
pub enum MappedLine {
    Exact(usize),     // the line exists unchanged in B
    Nearest(usize),   // the line was changed/removed; closest surviving line in B
}

impl MappedLine { pub fn line(&self) -> usize; }

/// Direction-agnostic: `diff` is old→new; `forward=true` maps old→new,
/// `forward=false` maps new→old.
pub fn map_line(diff: &FileDiff, line: usize, forward: bool) -> MappedLine;

/// What the view remembers to re-anchor after a revision change.
#[derive(Clone)]
pub struct LineAnchor {
    pub content: ContentKey,     // version the line number refers to
    pub line: usize,             // cursor line in that version
    pub viewport_offset: usize,  // cursor row - top row, to restore the same screen position
}
```

## Algorithm
Walk `diff.ops` (sorted by position):
* `Equal { old_index, new_index, len }` containing `line` ⇒ `Exact(new_index + (line - old_index))`.
* `Delete { old_index, old_len, new_index }` containing `line` ⇒ `Nearest(new_index)` (the
  line right after the deletion point, clamped to `new_len - 1`).
* `Replace { old_index, old_len, new_index, new_len }` containing `line` ⇒
  `Nearest(new_index + min(line - old_index, new_len - 1))` (proportional position
  inside the replaced block).
* `Insert` never contains an old line.
Empty target ⇒ `Nearest(0)`.

## Specs
* The diff used is **between the two versions directly** (A vs B, whatever their
  distance in history), not a composition of per-commit diffs. `update.rs` requests
  `ComputeDiff{A, B}` if not cached; both contents are already loaded at that point.
* After mapping, the view restores `viewport_offset` so the line stays at the same
  screen row (clamped to bounds).
* In **diff views** the anchor is translated to/from the *new side* line of the cursor row
  (a `Removed` row anchors to the next new-side line).
* Unit tests: unchanged file, line inside deleted block, inside replaced block of
  different size, line at EOF, empty → non-empty and reverse.
