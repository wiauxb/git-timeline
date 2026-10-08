# `src/model/diff.rs`

## Purpose
Compute the difference between two versions **in-process** and expose it both as
display hunks (for `ui/diff_view.rs`) and as raw ops (for `line_map.rs`).

## Contents

```rust
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct DiffKey { pub old: ContentKey, pub new: ContentKey }

pub struct FileDiff {
    pub ops: Vec<similar::DiffOp>,   // full op list (Equal/Delete/Insert/Replace),
                                     // indices are line numbers in old/new
    pub hunks: Vec<Hunk>,            // grouped with CONTEXT lines of context
    pub added: usize,
    pub removed: usize,
    pub timed_out: bool,             // deadline hit: diff is valid but not minimal
}

pub struct Hunk {
    pub old_start: usize, pub old_len: usize,
    pub new_start: usize, pub new_len: usize,
    pub lines: Vec<DiffLine>,
}

#[derive(Clone, Copy)]
pub enum DiffLine {
    Context { old: usize, new: usize },
    Removed { old: usize },
    Added   { new: usize },
}

pub const CONTEXT: usize = 3;

/// `old`/`new` may be `None` for Absent (treated as empty).
pub fn compute(old: Option<&TextContent>, new: Option<&TextContent>) -> FileDiff;
```

## Specs
* Algorithm: `similar::Algorithm::Myers` (git's default), line granularity, comparing
  **line bytes** (so CRLF vs LF differences show as changes, like git).
* Deadline: 300 ms per diff (`similar` returns a coarser but correct diff ⇒ `timed_out`).
* Diff lines store **indices**, not text: the renderer pulls text (and highlighting)
  from the two `TextContent`s ⇒ diff lines are syntax highlighted for free.
* "No newline at end of file": represented by comparing `ends_with_newline` of both
  sides; the renderer prints git's `\ No newline at end of file` marker.
* Diffs involving `Binary`/`TooLarge`/`Symlink`/`Submodule` are **not** computed; the view
  shows a summary ("Binary file changed: 12.3 KiB → 14.0 KiB", "Submodule abc123 → def456",
  "Symlink target: a → b").
* Pure function ⇒ unit tests with literal strings, including empty files, added/deleted
  files, only-whitespace changes, missing final newline.
