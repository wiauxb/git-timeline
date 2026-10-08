# `src/git/show.rs`

## Purpose
Fetch on demand what the timeline doesn't load: the **full commit message**, refs, and the
files touched by the commit, for the details popup (`d`).

## Contents

```rust
const MAX_FILES: usize = 200;

#[derive(Debug, Clone)]
pub struct CommitDetails {
    pub oid: Oid,
    pub body: String,                    // %B, trailing whitespace trimmed
    pub refs: Vec<String>,               // %D split on ", " (e.g. "HEAD -> main", "tag: v1")
    pub files: Vec<(char, RepoPath)>,    // status letter + path (new path for R/C)
    pub files_truncated: bool,           // more than MAX_FILES
}

pub fn commit_details(git: &Git, oid: &Oid) -> Result<CommitDetails>;
```

## Commands
```
git show -s --format=%B%x00%D <oid>
git diff-tree --no-commit-id -r --name-status -z -M --root <oid>
```
`diff-tree` on a merge compares against the first parent; `--root` lists the files of a root
commit.

## Specs
* Cached by oid in `app/cache.rs`. Author / committer / dates / parents come from
  `CommitMeta`, so the popup shows them immediately and fills body / refs / files when the
  job returns.
