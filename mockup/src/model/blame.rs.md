# `src/model/blame.rs`

## Purpose
Result of `git blame` for one file at one revision, shaped for the blame gutter.

## Contents

```rust
#[derive(Debug, Default)]
pub struct BlameData {
    pub lines: Vec<BlameLine>,                    // one per content line, same order
    pub commits: HashMap<Oid, Arc<BlameCommit>>,  // deduplicated commit info
}

#[derive(Debug, Clone)]
pub struct BlameLine {
    pub commit: Oid,          // null oid = not committed yet
    pub orig_path: RepoPath,  // path in the originating commit (renames)
    pub orig_line: usize,     // 0-based
    pub group_start: bool,    // first of a run of lines from the same commit
}

#[derive(Debug)]
pub struct BlameCommit {
    pub oid: Oid,
    pub author: String,
    pub author_time: DateTime<FixedOffset>,
    pub summary: String,
    pub boundary: bool,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum BlameKey {
    Commit { oid: Oid, path: RepoPath },
    /// Index / WorkingTree rows: keyed by the exact bytes being blamed.
    Local { rev: RevisionId, content: ContentKey },
}
```

## Specs
* `BlameKey` is built by `AppState::blame_key`: commits with an absent file get no key;
  local rows need their content loaded first (the key embeds its `ContentKey`).
* The renderer uses the blame only if `lines.len() == line_count()` of the displayed text
  (guards against a working-tree edit between content read and blame).
* `group_start` ⇒ gutter shows hash/author/date only on the first line of a group.
* Age coloring uses `author_time` relative to the oldest/newest commit in `commits`.
