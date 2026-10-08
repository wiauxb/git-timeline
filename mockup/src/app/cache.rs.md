# `src/app/cache.rs`

## Purpose
Bounded in-memory caches owned by the main thread (`AppState::cache`). Make moving back
and forth in history instant after the first visit. Workers are stateless.

## Contents

```rust
const CONTENT_BYTES: usize = 256 * 1024 * 1024;

/// Shared `Arc<FileContent::Absent>` (OnceLock), never stored in the LRU.
pub fn absent() -> Arc<FileContent>;

pub struct Caches {
    pub source_keys: HashMap<BlobSource, ContentKey>,       // where each source resolved to
    pub content_errors: HashMap<BlobSource, String>,        // failed loads (no retry loop)
    content: LruCache<ContentKey, Arc<FileContent>>,        // 256 entries
    content_bytes: usize,                                   // running total (approx_size)
    pub highlight: LruCache<HighlightKey, HighlightOutcome>,              // 64
    pub diff:      LruCache<DiffKey, Arc<FileDiff>>,                       // 128
    pub blame:     LruCache<BlameKey, Result<Arc<BlameData>, String>>,     // 32
    pub details:   LruCache<Oid, Result<Arc<CommitDetails>, String>>,      // 128
}

impl Default for Caches;
impl Caches {
    pub fn key_of(&self, src: &BlobSource) -> Option<ContentKey>;   // Absent → Some(Absent)
    pub fn content_by_key(&self, key: &ContentKey) -> Option<Arc<FileContent>>; // peek
    /// Err = cached load error; Ok(None) = not loaded yet.
    pub fn content_for(&self, src: &BlobSource)
        -> Result<Option<(ContentKey, Arc<FileContent>)>, String>;
    pub fn insert_content(&mut self, src: BlobSource, key: ContentKey, c: Arc<FileContent>);
    pub fn touch(&mut self, keys: &[ContentKey]);    // promote: never evict what is shown
    pub fn invalidate_local(&mut self);
}
```

## Specs
* Reads use `peek` (no LRU mutation, works through `&AppState`); recency is refreshed by
  `touch` from `update::after_batch` for the displayed blobs and the pending anchor.
* `insert_content` clears a previous error for that source, records `source_keys`, and
  evicts LRU entries while `content_bytes > 256 MiB` (keeping at least one).
* `Object` content never goes stale (content-addressed). Working-tree keys include
  mtime+len.
* `invalidate_local` (reload, after restore): forgets `WorkingTree` source mappings (so
  the file is re-read), clears all `content_errors`, and drops **error** entries from the
  blame and details caches.
