# `src/ops/restore.rs`

## Purpose
Replace the file in the working tree with the selected historical version
("restore this version"). The only destructive operation of the tool.

## Contents

```rust
pub fn restore(bytes: &[u8], mode: FileMode, target: &Path) -> OpResult;
```

## Specs
* `target` = `ctx.target` in the working tree (the **current** path), even when the
  version comes from before a rename.
* Equivalent to `git show <rev>:<path-at-rev> > <current-path>` plus mode:
  * `Executable` ⇒ set `0o755`-style exec bits (preserve other permission bits);
    `Regular` ⇒ clear exec bits. (Unix only; no-op on Windows.)
  * `Symlink` ⇒ recreate the symlink with the target (remove + `symlink`).
* **Does not stage** (index untouched) — the user reviews with `git diff` and commits.
* Requires confirmation (`ConfirmRestore` overlay). The dialog shows:
  * the revision being restored (`abc1234 "subject"`),
  * the destination path,
  * a warning in red if the working tree already has uncommitted changes to this file
    ("local changes will be lost — they are NOT recoverable through git").
* After success: status "Restored main.rs from abc1234 (not staged)", local state is
  reloaded ⇒ a `Working tree` row appears, selection stays on the restored commit.
* Refused for: the `WorkingTree` row (no-op), `Absent` (would mean deleting the file —
  out of scope), `Submodule`, `TooLarge` (bytes not in memory).
