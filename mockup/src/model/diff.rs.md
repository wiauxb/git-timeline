# `src/model/diff.rs`

## Purpose
Compute the difference between two versions **in-process** and expose it as display
hunks/rows (for `ui/diff_view.rs`) and as raw ops (for `line_map.rs`).

## Contents

```rust
pub const CONTEXT: usize = 3;
const DEADLINE: Duration = Duration::from_millis(300);

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct DiffKey { pub old: ContentKey, pub new: ContentKey }

#[derive(Debug, Default)]
pub struct FileDiff {
    pub ops: Vec<similar::DiffOp>,   // full op list, indices = line numbers in old/new
    pub hunks: Vec<Hunk>,            // grouped with CONTEXT lines of context
    pub added: usize,
    pub removed: usize,
    pub timed_out: bool,             // deadline hit: valid but not minimal
}
impl FileDiff {
    /// Display rows: for each hunk, a header row then its lines.
    pub fn rows(&self) -> Vec<DiffRow>;
}

pub struct Hunk { pub old_start: usize, pub old_len: usize,
                  pub new_start: usize, pub new_len: usize, pub lines: Vec<DiffLine> }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiffLine {
    Context { old: usize, new: usize },
    Removed { old: usize },
    Added   { new: usize },
}
impl DiffLine { pub fn new_line(&self) -> Option<usize>; }   // None for Removed

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiffRow { HunkHeader(usize /*hunk index*/), Line(DiffLine) }

/// `None` = absent (empty).
pub fn compute(old: Option<&TextContent>, new: Option<&TextContent>) -> FileDiff;

/// New-side line a row stands for: its own, else the next row's, else the previous row's;
/// if no row has a new-side line (pure deletion), the first removed old line.
pub fn source_line_of_row(rows: &[DiffRow], row: usize) -> Option<usize>;
/// Row showing new-side `line`, or the row with the closest new-side line (0 if none).
pub fn row_of_source_line(rows: &[DiffRow], line: usize) -> usize;
```

## Specs
* `similar::capture_diff_slices_deadline(Algorithm::Myers, …)` over **line bytes** (CRLF vs
  LF differences show as changes, like git). Hunks via `similar::group_diff_ops(ops, CONTEXT)`;
  a `Replace` op emits all removed lines then all added lines.
* `timed_out` = the deadline passed while computing.
* Rows store **indices**, not text: the renderer pulls text and highlighting from the two
  `TextContent`s, so diff lines are syntax highlighted for free.
* Only `Text`/`Absent` pairs are diffed (`app::state::diffable`); other pairs are shown as a
  one-line summary by `ui/diff_view.rs::summary`.
* Identical versions ⇒ no hunks, no rows.
* The row helpers are used to anchor the cursor across revisions in diff views.

## Tests
Counts and hunk bounds for a replace+insert; added / deleted / both-absent; identical.
