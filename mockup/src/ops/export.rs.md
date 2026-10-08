# `src/ops/export.rs`

## Purpose
Save the selected version of the file to a new file, without touching the repository.

## Contents

```rust
/// Default destination, relative to `ctx.invocation_dir`:
///   <stem>@<short-oid>.<ext>   e.g.  main@3fa2c1d.rs
///   <stem>@staged.<ext>        for the Index row
/// (WorkingTree row: export is allowed but pointless; name uses "@worktree").
pub fn default_name(entry: &TimelineEntry) -> String;

/// Resolve user input (relative to invocation_dir, `~` expanded) and write bytes.
/// `overwrite=false` and destination exists ⇒ OpResult::ExportExists.
pub fn write(bytes: &[u8], dest: &Path, overwrite: bool) -> OpResult;
```

## Specs
* Uses the file name **at that revision** (`entry.path`), not the current name.
* Bytes are the exact blob bytes (no line-ending conversion, no lossy UTF-8).
* Symlink versions export the link **target text** as a regular file (documented).
* Submodule / Absent / TooLarge versions: action disabled (status bar explains why).
  TooLarge could be exported by streaming `cat-file` — v2.
* Parent directory must exist; no `mkdir -p`.
