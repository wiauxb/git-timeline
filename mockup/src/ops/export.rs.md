# `src/ops/export.rs`

## Purpose
Save the selected version of the file to a new file, without touching the repository.

## Contents

```rust
/// `<stem>@<tag>.<ext>` using the file name AT THAT REVISION (`entry.path`);
/// tag = short oid, `staged` (Index row) or `worktree` (WorkingTree row).
/// No extension (or dot-file like `.env`) ⇒ `<name>@<tag>`.
pub fn default_name(e: &TimelineEntry) -> String;

/// Exists and !overwrite ⇒ OpResult::ExportExists (→ ConfirmOverwrite dialog);
/// directory ⇒ Failed; else atomic_write ⇒ Exported / Failed.
pub fn write(bytes: &[u8], dest: &Path, overwrite: bool) -> OpResult;
```

## Specs
* Destination resolution happens in `update.rs`: input trimmed, `~/` expanded with `HOME`,
  relative paths joined to `ctx.invocation_dir` (where `git timeline` was launched).
* Bytes are the exact stored bytes (no line-ending conversion). Symlink versions export the
  link target text as a regular file.
* Not available (refused in `update.rs` with a status message) when the content is not
  loaded yet, `Absent`, `Submodule` or `TooLarge`.
* The parent directory must exist (no `mkdir -p`).
* Existence check uses `symlink_metadata` (a dangling symlink counts as existing).
