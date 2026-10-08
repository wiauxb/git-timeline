# `src/model/content.rs`

## Purpose
In-memory representation of the file at one revision, with fast line access.

## Contents

```rust
pub const MAX_BLOB_SIZE: u64 = 50 * 1024 * 1024;

/// Cache key. Working-tree content includes mtime+len so an edit on disk misses the cache.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum ContentKey {
    Object(Oid),
    WorkingTree { path: RepoPath, mtime_ns: i128, len: u64 },
    Absent,
}

#[derive(Debug)]
pub enum FileContent {
    Text(TextContent),
    Binary   { bytes: Arc<[u8]> },          // bytes kept for export / restore
    TooLarge { size: u64, limit: u64 },     // never read
    Symlink  { target: BString },           // mode 120000: blob content is the target
    Submodule{ commit: Oid },               // mode 160000: the oid is a commit, no blob
    Absent,
}
impl FileContent {
    pub fn as_text(&self) -> Option<&TextContent>;
    /// Raw bytes for Text / Binary / Symlink (target); None otherwise.
    pub fn bytes(&self) -> Option<Arc<[u8]>>;
    /// Approximate memory footprint (cache byte budget).
    pub fn approx_size(&self) -> usize;
}

#[derive(Debug)]
pub struct TextContent {
    bytes: Arc<[u8]>,
    line_starts: Vec<u32>,
    pub ends_with_newline: bool,
    pub has_crlf: bool,
    pub invalid_utf8: bool,
}
impl TextContent {
    pub fn from_bytes(bytes: Vec<u8>) -> Self;
    pub fn line_count(&self) -> usize;
    pub fn line_bytes(&self, i: usize) -> &[u8];     // without trailing \n / \r\n
    pub fn line_str(&self, i: usize) -> Cow<'_, str>; // lossy UTF-8
    pub fn lines(&self) -> impl Iterator<Item = &[u8]>;
    pub fn bytes(&self) -> &Arc<[u8]>;
}

/// git's heuristic: a NUL byte in the first 8000 bytes.
pub fn is_binary(bytes: &[u8]) -> bool;
/// Symlink mode ⇒ Symlink; binary ⇒ Binary; else Text.
pub fn classify(bytes: Vec<u8>, mode: FileMode) -> FileContent;
```

## Specs
* Line splitting: a final `\n` does not start an extra empty line (`"x\n\n"` = 2 lines);
  an empty file has 0 lines. A trailing `\r` is stripped from each line.
* The size limit is applied by the readers (`git/blob.rs`) **before** reading bytes.
* Tabs are not expanded here (render-time concern in `ui/content_view.rs`).
* No encoding detection beyond UTF-8 (lossy). UTF-16 files are usually binary by the NUL
  heuristic, as in git.
* Highlight byte ranges (`highlight.rs`) refer to `line_str(i)`, so they stay valid for
  invalid UTF-8 lines too.

## Tests
CRLF / missing final newline / empty file line splitting; NUL before vs after byte 8000.
