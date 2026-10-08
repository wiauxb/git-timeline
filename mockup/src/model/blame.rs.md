# `src/model/blame.rs`

## Purpose
Result of `git blame` for one file at one revision, shaped for the blame gutter.

## Contents

```rust
pub struct BlameData {
    pub lines: Vec<BlameLine>,                    // one per content line, same order
    pub commits: HashMap<Oid, Arc<BlameCommit>>,  // deduplicated commit info
}

pub struct BlameLine {
    pub commit: Oid,              // null oid = "Not Committed Yet" (working tree / staged)
    pub orig_path: RepoPath,      // path in the originating commit (renames)
    pub orig_line: usize,         // 0-based line in the originating commit
    pub group_start: bool,        // first line of a run of lines from the same commit
}

pub struct BlameCommit {
    pub oid: Oid,
    pub author: String,
    pub author_time: chrono::DateTime<chrono::FixedOffset>,
    pub summary: String,
    pub boundary: bool,           // "boundary" flag (range-limited blame)
}

/// Key: blame is for one revision of one path.
#[derive(Clone, PartialEq, Eq, Hash)]
pub enum BlameKey {
    Commit { oid: Oid, path: RepoPath },
    Index(RepoPath),
    WorkingTree { path: RepoPath, mtime_ns: i128, len: u64 },
}
```

## Specs
* `lines.len()` must equal the content's `line_count()`; if not (race with a working-tree
  edit), the blame is discarded and re-requested once.
* `group_start` lets the renderer print the gutter (hash, author, date) only on the first
  line of a group and a thin bar `│` on the following ones.
* Age coloring (`ui/blame_view.rs`) uses `author_time` relative to the oldest/newest
  commit present in `commits`.
