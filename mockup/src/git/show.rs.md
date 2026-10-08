# `src/git/show.rs`

## Purpose
Fetch, on demand, information not loaded with the timeline: the **full commit message**
and the list of other files touched by the commit, for the details popup (`d`).

## Contents

```rust
pub struct CommitDetails {
    pub oid: Oid,
    pub body: String,                    // full message (%B), trailing whitespace trimmed
    pub refs: Vec<String>,               // branch/tag names pointing at it (%D)
    pub files: Vec<(char, RepoPath)>,    // status letter + path, first 200
    pub files_truncated: bool,
}

pub fn commit_details(git: &Git, oid: &Oid) -> Result<CommitDetails>;
```

## Command
```
git show -s --format=%B%x00%D <oid>
git diff-tree --no-commit-id -r --name-status -z -M --root <oid>   (first parent)
```

## Specs
* Cached by oid in `app/cache.rs` (tiny).
* Author / committer / dates / parents come from `CommitMeta` already in the timeline; the
  popup shows them immediately and fills body/refs/files when the job returns.
