# `src/ops/restore.rs`

## Purpose
Replace the file in the working tree with the selected historical version. The only
destructive operation of the tool.

## Contents

```rust
pub fn restore(bytes: &[u8], mode: FileMode, target: &Path) -> OpResult;  // Restored / Failed
fn restore_inner(bytes: &[u8], mode: FileMode, target: &Path) -> std::io::Result<()>;
fn restore_symlink(bytes: &[u8], target: &Path) -> std::io::Result<()>;   // unix; Unsupported elsewhere
fn set_exec(target: &Path, exec: bool) -> std::io::Result<()>;             // unix; no-op elsewhere
```

## Behavior
* `target` = `ctx.target` in the working tree: the **current** path, even when the
  version comes from before a rename. Missing parent directories are created.
* `Regular` / `Executable`: an existing symlink at `target` is removed (replaced, not
  followed), bytes written with `atomic_write`, then exec bits set:
  * `Executable` ⇒ add exec where read is granted (`mode | (mode & 0o444) >> 2`, like git
    checkout);
  * `Regular` ⇒ clear all exec bits.
* `Symlink` ⇒ remove whatever is at `target`, recreate the symlink to the stored target.
* `Submodule` / `Absent` ⇒ `InvalidInput` error.
* Does **not** stage: index, HEAD and refs are untouched.

## Guards (in `update.rs`, before this is ever called)
* Refused for the `WorkingTree` row, and when the content is not loaded, `Absent`,
  `Submodule` or `TooLarge`.
* Requires the `ConfirmRestore` dialog (`y`); it warns when a `Working tree` row exists,
  i.e. local changes will be lost.
* `Absent` mode at confirm time is mapped to `Regular`.
* On success: status `restored <path> from <label> (not staged)`, local caches invalidated,
  `LoadLocalState` resubmitted so the `Working tree` row reflects the new file.
