# `src/model/mod.rs`

## Purpose
Domain types shared by every layer. **Pure**: no IO, no git process. The only external
types used are `similar::DiffOp` (diff ops) and `chrono` dates.

## Contents

```rust
mod blame;          pub use blame::*;     // BlameData, BlameLine, BlameCommit, BlameKey
mod content;        pub use content::*;   // FileContent, TextContent, ContentKey, classify,
                                          // is_binary, MAX_BLOB_SIZE
pub mod diff;       pub use diff::*;      // FileDiff, Hunk, DiffLine, DiffRow, DiffKey,
                                          // compute, source_line_of_row, row_of_source_line
pub mod line_map;   pub use line_map::{LineAnchor, MappedLine};   // + map_line
mod path;           pub use path::RepoPath;
mod revision;       pub use revision::*;  // Oid, RevisionId, Signature, CommitMeta, FileMode,
                                          // ChangeKind, BlobSource, TimelineEntry, Timeline
```

## Invariants shared across the module
* **Line numbers are 0-based internally**, rendered 1-based by `ui/`.
* Large shared data travels as `Arc` (`Arc<FileContent>`, `Arc<FileDiff>`, `Arc<BlameData>`,
  `Arc<CommitMeta>`), so caches, state and workers share without copying.
* Everything here is `Send + Sync`.
