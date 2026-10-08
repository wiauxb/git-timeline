# `src/git/status.rs`

## Purpose
Detect uncommitted changes to the target file and build the `Staged` and
`Working tree` **pseudo-entries** shown above HEAD.

## Contents

```rust
pub struct LocalState {
    pub head_blob: Option<(Oid, FileMode)>,    // None: unborn HEAD or file not in HEAD
    pub index_blob: Option<(Oid, FileMode)>,   // stage 0 entry
    pub conflicted: bool,                      // stages 1..3 present
    pub index_differs: bool,                   // X != '.'
    pub worktree_differs: bool,                // Y != '.' or untracked
    pub untracked: bool,
    pub worktree_exists: bool,
}

pub fn read_local_state(git: &Git, path: &RepoPath) -> Result<LocalState>;
pub fn local_entries(state: &LocalState, path: &RepoPath) -> Vec<TimelineEntry>; // 0..=2
```

## Commands
* `git status --porcelain=v2 -z --no-renames --untracked-files=all -- :(literal)PATH`
  * `1 <XY> …` ordinary change → X = index vs HEAD, Y = worktree vs index (`.` = same).
  * `u <XY> …` unmerged → `conflicted = true`.
  * `? <path>` untracked → `untracked = true`.
  * no line → clean.
* `git ls-files -s -z -- :(literal)PATH` → `<mode> <oid> <stage>\t<path>` (stage 0 or 1-3).
* `git ls-tree -z HEAD -- :(literal)PATH` → `<mode> blob <oid>\t<path>` (skipped if unborn).

## Entry construction
| Condition | Row | blob | prev_blob | change |
|---|---|---|---|---|
| `index_differs && !conflicted` | `Index` ("Staged") | Object(index oid) or Absent if staged deletion | HEAD blob or Absent | Added / Modified / Deleted |
| `worktree_differs` | `WorkingTree` | WorkingTree(path) or Absent if deleted on disk | index blob (or HEAD blob) | Modified / Deleted / Untracked |
| `conflicted` | `WorkingTree` only, `conflicted = true` | WorkingTree(path) | HEAD blob | Modified |

Order in the timeline: `WorkingTree` first, then `Index`, then commits.

## Specs
* Called at startup and on reload (`r`), and after a successful **restore**.
* Not called at all when a revision range was given (`ctx.show_local == false`).
* No file-system watching in v1 (manual reload).
