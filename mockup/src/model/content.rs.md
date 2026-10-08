# `src/model/content.rs`

## Purpose
In-memory representation of the file at one revision, with fast line access.

## Contents

```rust
/// Cache key for content. Working-tree content is keyed with its mtime+len so an
/// edit on disk invalidates the cache entry naturally.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum ContentKey {
    Object(Oid),
    WorkingTree { path: RepoPath, mtime_ns: i128, len: u64 },
    Absent,
}

pub enum FileContent {
    Text(TextContent),
    Binary   { size: u64 },
    TooLarge { size: u64, limit: u64 },
    Symlink  { target: bstr::BString },     // mode 120000: blob content is the target
    Submodule{ commit: Oid },               // mode 160000: no blob, the oid is a commit
    Absent,                                 // deleted / not yet created
}

pub struct TextContent {
    bytes: Arc<[u8]>,
    line_starts: Vec<u32>,      // byte offset of each line start
    pub ends_with_newline: bool,
    pub has_crlf: bool,         // shown as a hint in the file pane header
    pub invalid_utf8: bool,     // lines rendered lossy, header shows "non-UTF-8"
}

impl TextContent {
    pub fn from_bytes(bytes: Vec<u8>) -> Self;
    pub fn line_count(&self) -> usize;
    pub fn line_bytes(&self, i: usize) -> &[u8];  // without trailing \n / \r\n
    pub fn line_str(&self, i: usize) -> std::borrow::Cow<'_, str>; // lossy
    pub fn lines(&self) -> impl Iterator<Item = &[u8]>;
}

pub fn classify(bytes: Vec<u8>, mode: FileMode) -> FileContent;
pub const MAX_BLOB_SIZE: u64 = 50 * 1024 * 1024;
```

## Specs
* **Binary detection** follows git's heuristic: a NUL byte in the first 8000 bytes.
* Size limit is checked **before** reading the bytes (cat-file `info` gives the size):
  bigger than `MAX_BLOB_SIZE` ⇒ `TooLarge` (no read).
* `line_starts` is `u32`: a 50 MiB file fits. Empty file ⇒ 0 lines.
* Tabs are **not** expanded here (render-time concern in `ui/`).
* No encoding detection beyond UTF-8 (lossy). UTF-16 files are usually detected as binary
  by the NUL heuristic, matching git's behavior.
