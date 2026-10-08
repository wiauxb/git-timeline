# `src/ops/mod.rs`

## Purpose
User-triggered side effects on the outside world (clipboard, file system). Called only by
`App::execute` for an `Effect`; each returns an `OpResult` fed back as `Action::OpDone`.

## Contents

```rust
pub mod clipboard;   // copy text
pub mod export;      // write a version to a chosen file
pub mod restore;     // write a version into the working tree

/// Write to `<dest>.git-timeline.tmp` in the same directory, fsync, then rename over
/// `dest`. If `dest` already exists, its permissions are copied onto the temp file before
/// the rename. On write failure the temp file is removed.
pub(crate) fn atomic_write(dest: &Path, bytes: &[u8]) -> std::io::Result<()>;
```

## Rules shared by all ops
* Synchronous and fast: bytes are already in memory, no git call.
* Never touch the index, HEAD, refs, or any file other than the explicit destination.
* All file writes go through `atomic_write`.
