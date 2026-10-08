# `src/model/mod.rs`

## Purpose
Domain types shared by every layer. **Pure**: no IO, no git process, no ratatui
(exception: none — styles live in `highlight.rs`/`ui/`).

## Contents

```rust
mod path;      pub use path::RepoPath;
mod revision;  pub use revision::*;   // Oid, RevisionId, CommitMeta, ChangeKind, FileMode,
                                      // BlobSource, TimelineEntry, Timeline
mod content;   pub use content::*;    // FileContent, TextContent, ContentKey
mod diff;      pub use diff::*;       // FileDiff, Hunk, DiffLine, DiffKey
mod blame;     pub use blame::*;      // BlameData, BlameLine, BlameCommit
pub mod line_map;                     // LineMap, MappedLine
```

## Invariants shared across the module
* **Line numbers are 0-based internally**, rendered 1-based by `ui/`.
* All shared, potentially large data is held in `Arc` (`Arc<FileContent>`,
  `Arc<FileDiff>`, …) so caches, state and workers can share without copying.
* Everything here is `Send + Sync`.
