# `src/ops/mod.rs`

## Purpose
User-triggered side effects on the outside world (clipboard, file system). Called only
by `App::execute` in response to an `Effect`; each returns an `OpResult`.

## Contents

```rust
pub mod clipboard;   // copy text
pub mod export;      // write a version to a chosen file
pub mod restore;     // write a version into the working tree
```

## Rules shared by all ops
* Synchronous and fast (bytes are already in memory; no git call). If one ever becomes
  slow, it moves to a worker job — the `Effect`/`OpResult` interface doesn't change.
* Never touch the index, HEAD, refs or any file other than the explicit destination.
* All writes are **atomic**: write to `<dest>.git-timeline.tmp` in the same directory,
  `fsync`, then `rename` over the destination.
