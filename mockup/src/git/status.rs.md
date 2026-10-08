# `src/git/status.rs`

## Purpose
Detect uncommitted changes to the target file and build the `Working tree` and `Staged`
**pseudo-entries** shown above HEAD. Also hosts small helpers shared with `git/blob.rs`.

## Contents

```rust
#[derive(Debug, Default)]
pub struct LocalState {
    pub head_blob: Option<(Oid, FileMode)>,    // None: unborn HEAD or not in HEAD
    pub index_blob: Option<(Oid, FileMode)>,   // stage 0
    pub conflicted: bool,                      // `u` record or stages 1-3
    pub index_differs: bool,                   // X != '.'
    pub worktree_differs: bool,                // Y != '.' or untracked
    pub untracked: bool,
    pub worktree_mode: Option<FileMode>,       // None: missing on disk
}

pub fn read_local_state(git: &Git, path: &RepoPath) -> Result<LocalState>;
/// 0..=2 entries, newest first: WorkingTree, then Index.
pub fn local_entries(st: &LocalState, path: &RepoPath) -> Vec<TimelineEntry>;
/// First record of `git ls-tree -z`: `<mode> <type> <oid>\t<path>`.
pub fn parse_ls_tree(out: &[u8]) -> Option<(Oid, FileMode)>;
/// symlink ⇒ Symlink; any exec bit (Unix) ⇒ Executable; else Regular.
pub fn mode_of(m: &std::fs::Metadata) -> FileMode;
```

## Commands
* `git status --porcelain=v2 -z --no-renames --untracked-files=all -- :(literal)PATH`
  * `1`/`2 <XY> …` → `index_differs |= X != '.'`, `worktree_differs |= Y != '.'`
  * `u …` → conflicted; `? <path>` → untracked + worktree_differs.
* `git ls-files -s -z -- :(literal)PATH` → `<mode> <oid> <stage>\t<path>`: stage 0 ⇒
  `index_blob`, other stages ⇒ conflicted.
* `git ls-tree -z HEAD -- PATH` via `probe` (fails silently on unborn HEAD) ⇒ `head_blob`.
* `symlink_metadata(toplevel/PATH)` ⇒ `worktree_mode`.

## Entry construction
| Condition | Row | blob | prev_blob | change |
|---|---|---|---|---|
| `conflicted` | WorkingTree only, `conflicted = true` | WorkingTree(path) or Absent | HEAD blob | Modified |
| `worktree_differs` | WorkingTree | WorkingTree(path) or Absent if missing | untracked: Absent; else index blob if any, else HEAD blob | Untracked / Deleted (missing) / Added (prev absent) / Modified |
| `index_differs` | Index ("Staged") | index blob or Absent | HEAD blob or Absent | Added (no HEAD blob) / Deleted (no index blob) / Modified |

`meta = None`, `path = prev_path = target`, modes from the corresponding blobs.

## Specs
* Called synchronously at startup (`App::new`) and as a `LoadLocalState` job on reload and
  after a restore. Not used when a revision range was given (`ctx.show_local == false`).
* No file-system watching (manual reload `r`).

## Tests (tests/timeline.rs)
Staged + worktree rows and their `prev_blob` chain; untracked; unborn HEAD with a staged file.
